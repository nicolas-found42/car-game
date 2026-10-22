# Rust code limits & complexity gates: alternatives to CodeLimit

> **Research note — 2026-10-22.** Primary sources only: the upstream repositories, their docs,
> source files, release metadata, and the GitHub REST API (stars/last-push checked 2026-10-22).
> No third-party blog posts were used.
>
> **Versions studied:** lizard **1.24.2** (released 2026-10-22), `mozilla/rust-code-analysis`
> master plus latest release **v0.0.25** (2023-01-13), Clippy master (`clippy` 0.1.101) and the
> stable-toolchain docs, `boyter/scc` master (README shows **v4.1.0**), `minikin/cargo-crap`
> **0.9.0**, and `getcodelimit/codelimit` **v0.21.0** (2025-10-21).
>
> **Two figures in the brief are stale.** GitHub's API on 2026-10-22 reports **2,562 stars** for
> `terryyin/lizard` (not 4.2k) and **462 stars** for `mozilla/rust-code-analysis` (not 1.2k).
> Both numbers below come from the API, not from the brief.

---

## 0. TL;DR

1. **`terryyin/lizard` is the closest mature, cross-language CodeLimit replacement with real Rust
   support.** It has a dedicated Rust reader, computes NLOC, cyclomatic complexity, parameters,
   tokens and raw span, can also compute Sonar-style Cognitive Complexity, enforces thresholds on
   the command line, exits non-zero on violations, honors `.gitignore`, and supports a whitelist
   plus inline forgiveness comments. It is a token/state-machine analyzer, not an AST parser, so
   its Rust CCN has measurable quirks (see §2.2 and §7).
2. **`mozilla/rust-code-analysis` is a metrics library/exporter, not a gate.** Its CLI has no
   threshold flags and exits 0 after a successful dump; CI gating would require a custom JSON
   post-processor. It is the most accurate of the cross-language metric calculators here because
   it uses Tree-sitter ASTs, but it is not a drop-in CodeLimit replacement.
3. **Clippy is the best Rust-native gate and is already installed in `car-game`.** `too_many_lines`
   (pedantic, default 100), `too_many_arguments` (complexity, default 7), `type_complexity`
   (complexity, default 250) and `excessive_nesting` (complexity, only active when configured)
   cover length, arity, type shape and nesting with compiler-grade accuracy. Clippy's
   `cognitive_complexity` lint is explicitly documented as *not* a real cognitive-complexity
   measurement and should not be used as a gate.
4. **`scc` and `cargo-crap` are not substitutes for CodeLimit.** `scc` computes a file-level
   complexity estimate and repository statistics, with no function-level thresholds. `cargo-crap`
   is a promising purpose-built Rust gate (CRAP = complexity × coverage, plus structural signals
   and baseline/regression gates) but requires LCOV coverage and is only months old.
5. **For `car-game`, keep Clippy and replace CodeLimit with lizard.** Use lizard as the single
   changed-file gate for both Python tooling and Rust, with language-specific NLOC thresholds
   (`60` for Python to preserve CodeLimit parity, `100` for Rust to preserve the current Clippy
   threshold), `-a 7`, and a calibrated CCN threshold. Keep Clippy's `too_many_lines` as the
   AST-accurate Rust backstop. Details and a concrete script/CI shape are in §8.

---

## 1. What "CodeLimit-like" means in this note

CodeLimit itself measures exactly one thing: function length in non-comment, non-blank lines,
with hard-coded bands (31–60 = warning, `> 60` = `check` failure) and no configurable thresholds,
no baseline file, and no Rust support
([`.codelimit.yml` and `scripts/check_codelimit.sh` in this repo](../../.codelimit.yml),
[`docs/codelimit-research.md`](./codelimit-research.md),
[`codelimit/languages/`](https://github.com/getcodelimit/codelimit/tree/main/codelimit/languages)).
The useful capability to replace is therefore broader than "count lines":

- per-function **length** (ideally code-only, not raw span);
- **cyclomatic** and/or **cognitive** complexity;
- **parameter count**;
- **threshold enforcement with a non-zero exit code**;
- **ratcheting/grandfathering** so existing debt does not block unrelated work;
- **Rust and Python in one workflow**, because `car-game` has both.

A tool that only prints metrics (rust-code-analysis, scc) is not a gate. A tool that only lints
Rust (Clippy) is a gate but not cross-language. Lizard is the only mature project in this set that
does all of the above for Rust and Python.

---

## 2. `terryyin/lizard`

### 2.1 Maturity and status

| Fact | Value | Source |
| --- | --- | --- |
| Stars / forks | 2,562 / 303 (2026-10-22) | [GitHub API](https://api.github.com/repos/terryyin/lizard) |
| Created | 2012-06-21 | [GitHub API](https://api.github.com/repos/terryyin/lizard) |
| Last push | 2026-10-22 | [GitHub API](https://api.github.com/repos/terryyin/lizard) |
| Latest release | 1.24.2, 2026-10-22 | [release 1.24.2](https://github.com/terryyin/lizard/releases/tag/1.24.2) |
| License | MIT (PyPI says MIT; `LICENSE.txt` is MIT) | [`LICENSE.txt`](https://github.com/terryyin/lizard/blob/master/LICENSE.txt), [PyPI](https://pypi.org/project/lizard/) |
| PyPI status | `Development Status :: 5 - Production/Stable` | [PyPI JSON](https://pypi.org/pypi/lizard/json) |
| Install | `pip install lizard`; also runnable as `python lizard.py` | [`README.rst`](https://github.com/terryyin/lizard/blob/master/README.rst) |

Lizard is old, actively maintained, and recent releases are still fixing Rust-specific behavior:
1.24.2 adds Rust `let-else` CCN counting and fixes HTML reports returning zero when thresholds are
exceeded; the preceding release added Rust `match`-arm CCN counting and the Cognitive Complexity
extension ([`CHANGELOG.md`](https://github.com/terryyin/lizard/blob/master/CHANGELOG.md),
[release 1.24.2 notes](https://github.com/terryyin/lizard/releases/tag/1.24.2)). This is not a
dormant project.

The GitHub API labels the repo language "JavaScript" because the project ships a website; the
installable analyzer is Python and the PyPI package is Python
([`setup.py`/PyPI metadata](https://pypi.org/pypi/lizard/json)). It requires Python 3.8+ and depends
on `pygments` and `pathspec`.

### 2.2 How it parses Rust

Rust is a first-class language in lizard: `rust.py` registers `ext = ['rs']` and the language name
`rust`, and it defines a dedicated `RustReader` plus `RustStates`
([`lizard_languages/rust.py#L10-L34`](https://github.com/terryyin/lizard/blob/master/lizard_languages/rust.py#L10-L34)).
The parser is **not** an AST parser. It is a tokenizer plus a hand-written state machine:

- `RustReader` uses `CodeReader`'s regex tokenizer and adds a lifetime/label token pattern
  (`(?:'\w+\b)`) so `'a` does not confuse the lexer
  ([`rust.py#L26-L31`](https://github.com/terryyin/lizard/blob/master/lizard_languages/rust.py#L26-L31)).
- `RustStates` extends `GoLikeStates`; the function keyword is literally `fn`
  ([`golike.py#L7-L12`](https://github.com/terryyin/lizard/blob/master/lizard_languages/golike.py#L7-L12)).
- After `fn`, it reads the name, skips generic parameter lists delimited by `<…>` or `[…]`, reads
  the parameter list inside `(...)`, then waits for `{` to enter the body
  ([`golike.py#L45-L83`](https://github.com/terryyin/lizard/blob/master/lizard_languages/golike.py#L45-L83)).
- `match` expressions get their own sub-state: the first `=>` is not counted; each subsequent
  `=>` adds one condition (so a 4-arm match contributes 3 CCN)
  ([`rust.py#L34-L47`](https://github.com/terryyin/lizard/blob/master/lizard_languages/rust.py#L34-L47)).
- `else` and `let ... else` are handled explicitly; the `else` of an `if` does not double-count as
  a `let-else`, but `else if` still contributes both an `else` and an `if`
  ([`rust.py#L49-L70`](https://github.com/terryyin/lizard/blob/master/lizard_languages/rust.py#L49-L70)).

Because it is a partial parser, lizard's own README warns that incorrect/unknown syntax can cause
"omission, misinterpretation, improper analysis / tally", while guaranteeing termination rather
than hard failure ([`README.rst` — Limitations](https://github.com/terryyin/lizard/blob/master/README.rst)).
That trade-off is why it works on incomplete code without include paths, and why its Rust CCN
should be calibrated rather than assumed to equal Clippy/rustc McCabe.

### 2.3 Metrics and their exact semantics

Lizard's core function metrics are:

| Metric | What lizard counts | Source |
| --- | --- | --- |
| `nloc` | lines of code without comments | [README](https://github.com/terryyin/lizard/blob/master/README.rst) |
| `cyclomatic_complexity` (CCN) | base 1 plus counted condition tokens | [`lizard.py#L306-L338`](https://github.com/terryyin/lizard/blob/master/lizard.py#L306-L338) |
| `token_count` | tokens in the function | [README](https://github.com/terryyin/lizard/blob/master/README.rst) |
| `parameter_count` | params parsed from the signature, comma-split and regex-matched | [`lizard.py#L338-L371`](https://github.com/terryyin/lizard/blob/master/lizard.py#L338-L371) |
| `length` | `end_line - start_line + 1` — the raw span, including comments and blanks | [`lizard.py#L352-L354`](https://github.com/terryyin/lizard/blob/master/lizard.py#L352-L354) |
| `max_nesting_depth` (ND) | nesting depth, reported by default | [`lizard.py#L715-L721`](https://github.com/terryyin/lizard/blob/master/lizard.py#L715-L721) |

Optional extensions add:

- **Cognitive Complexity** (`-Ecognitive`) — per-function SonarSource-style metric, a `CogC`
  column, a `--CogC` threshold (default 15), and a `cognitive_complexity` field usable with
  `-s`/`-T`; nesting is followed for Rust
  ([extensions docs](https://github.com/terryyin/lizard/blob/master/docs/extensions-and-warnings.rst)).
- Halstead metrics (`-Ehalstead`), nested-structure counting (`-ENS`), duplicate/code-clone
  detection (`-Eduplicate`), and word-count/tag-cloud (`-Ewordcount`)
  ([CLI docs](https://github.com/terryyin/lizard/blob/master/docs/cli.rst),
  [extensions docs](https://github.com/terryyin/lizard/blob/master/docs/extensions-and-warnings.rst)).

**Important CCN caveat for Rust.** The condition set is
`{'if', 'for', 'while', 'catch', 'where'} ∪ {'&&', '||'} ∪ {'?'}`, and every token in that set
increments CCN ([`rust.py#L13-L22`](https://github.com/terryyin/lizard/blob/master/lizard_languages/rust.py#L13-L22),
[`code_reader.py#L96-L118`](https://github.com/terryyin/lizard/blob/master/lizard_languages/code_reader.py#L96-L118),
[`lizard.py#L572-L577`](https://github.com/terryyin/lizard/blob/master/lizard.py#L572-L577)).
That means:

- **`where` clauses add CCN**, even though they are type constraints, not branches.
- **`?` (error propagation) adds CCN**, even though the comment says it is not a ternary; it is
  placed in `_ternary_operators`, which is unioned into the condition set.
- **`loop` is not in the set**, so an unconditional `loop` adds no CCN.
- `else if` contributes 2 (one for `else`, one for `if`).
- A `match` with N arms contributes N−1 (the first arm is free).

None of this makes lizard unusable, but it means a Rust CCN gate at 15 will not correspond to
Clippy/standard McCabe. Treat CCN as a heuristic and calibrate on the repo before enforcing (§8).

### 2.4 CLI: thresholds, exit codes, outputs

The complete option list is in [`docs/cli.rst`](https://github.com/terryyin/lizard/blob/master/docs/cli.rst).
The options that matter for a gate:

| Option | Default | Meaning |
| --- | --- | --- |
| `-l`, `--languages` | all known languages | Select languages, e.g. `-l rust -l python`. **This is not length.** |
| `-C`, `--CCN` | 15 | Warn when a function's CCN is greater than the value. |
| `-L`, `--length` | 1000 | Warn when a function's **raw span** is greater than the value. |
| `-a`, `--arguments` | 100 | Warn when a function's parameter count is greater than the value. |
| `-T`, `--Threshold` | — | Set any field threshold, e.g. `-T nloc=60`, `-T cyclomatic_complexity=10`, `-T length=10`, `-T parameter_count=7`. `-T nloc` is the code-only length gate; `-L` is the raw-span gate. |
| `-i`, `--ignore_warnings` | 0 | Exit 0 if the warning count is ≤ N; negative means always exit 0. Intended for legacy code. |
| `-W`, `--whitelist` | `./whitelizard.txt` | Whitelist file. |
| `-w`, `--warnings_only` | off | Print only warnings in clang/gcc format. |
| `--warning-msvs` | off | Print only warnings in MSVC format. |
| `-x`, `--exclude` | — | Exclude glob patterns; repeatable. |
| `--no-gitignore` | off | Do not use `.gitignore` files as an additional filter. |
| `-f`, `--input_file` | — | Read a list of filenames from a file. Useful for a changed-file ratchet. |
| `-t`, `--working_threads` | 1 | Parallel workers. |
| `-o`, `--output_file` | stdout | Output file; format inferred from extension unless a printer flag is given. |
| `-X`, `--xml` | off | cppncss-style XML. |
| `--csv` | off | CSV. |
| `-H`, `--html` | off | Interactive HTML report. |
| `--checkstyle` | off | Checkstyle XML for Jenkins and similar CI. |
| `-s`, `--sort` | — | Sort warnings by `nloc`, `cyclomatic_complexity`, `token_count`, `parameter_count`, etc. |
| `-E`, `--extension` | — | `-Ecognitive` (CogC), `-Ehalstead`, `-ENS`, `-Eduplicate`, etc. |

**Exit codes** are explicit and CI-friendly:

- `main()` returns `1` when `0 <= ignore_warnings < warning_count`; with the default `-i 0`, any
  warning fails the command ([`lizard.py#L1156-L1158`](https://github.com/terryyin/lizard/blob/master/lizard.py#L1156-L1158)).
- A negative `-i` always returns 0, which is how lizard supports legacy codebases
  ([`docs/cli.rst`](https://github.com/terryyin/lizard/blob/master/docs/cli.rst)).
- Usage/import/validation failures exit `2`
  ([`lizard.py#L1040`](https://github.com/terryyin/lizard/blob/master/lizard.py#L1040),
  [`lizard.py#L1086`](https://github.com/terryyin/lizard/blob/master/lizard.py#L1086)).
- The README states the same contract: "The exit code of lizard will be non-zero if there are
  warnings" ([README](https://github.com/terryyin/lizard/blob/master/README.rst)).
- As of 1.24.2, HTML output also returns non-zero when thresholds are exceeded
  ([release notes](https://github.com/terryyin/lizard/releases/tag/1.24.2)).

**File selection and ignores.** Lizard reads `.gitignore` in the scanned path automatically and
uses it as an additional filter; `--no-gitignore` disables that. `-x` adds glob exclusions, and
`-f` lets a script pass an explicit changed-file list
([README](https://github.com/terryyin/lizard/blob/master/README.rst),
[`docs/cli.rst`](https://github.com/terryyin/lizard/blob/master/docs/cli.rst)). This is enough to
implement a changed-files ratchet without a separate baseline format.

### 2.5 Whitelist, forgiveness, and ratcheting

Lizard has **three** grandfathering mechanisms, none of which is an automatic baseline:

1. **Whitelist file** (`whitelizard.txt` by default). Lines may list bare function names, comma
   separated, optionally prefixed with an exact file path:
   `file/path/name:function1, function2`. `#` starts a comment. The implementation matches
   `warning.name` against the list and, if a path was given, requires
   `warning.filename == path` exactly; there are no wildcards and no line numbers
   ([`lizard.py#L653-L687`](https://github.com/terryyin/lizard/blob/master/lizard.py#L653-L687),
   [extensions docs](https://github.com/terryyin/lizard/blob/master/docs/extensions-and-warnings.rst)).
   A missing whitelist file is only a printed warning, not an error
   ([`lizard.py#L672-L678`](https://github.com/terryyin/lizard/blob/master/lizard.py#L672-L678)).
   Practical consequences: whitelisting a common Rust method name such as `new` without a path
   suppresses it in every file; whitelisting a file path suppresses every listed function name in
   that file, including same-named methods in different `impl` blocks; renaming a function
   invalidates its whitelist entry.
2. **Inline forgiveness comments.** `// #lizard forgives` inside or before a function suppresses
   all warnings for it. Selective forgiveness is supported:
   `// #lizard forgives(length, parameter_count)`. `// #lizard forgive global` suppresses
   non-function/global code warnings
   ([extensions docs](https://github.com/terryyin/lizard/blob/master/docs/extensions-and-warnings.rst)).
   This is the finest-grained escape hatch and the best fit for a one-off exception with a
   documented reason.
3. **Global warning budget** (`-i N`). Exit 0 while the total warning count is ≤ N, or always with
   a negative N. This is a blunt ratchet: lower N over time as debt is paid down
   ([`docs/cli.rst`](https://github.com/terryyin/lizard/blob/master/docs/cli.rst)).

There is **no baseline JSON, no `--since`, and no built-in diff mode**. The ratchet is achieved by
feeding lizard only the changed files (`-f` or explicit paths) and using the whitelist/inline
forgiveness for exceptions. That is exactly the pattern already used by
`scripts/check_codelimit.sh` for codelimit and Clippy, so it fits this repo.

### 2.6 CI usage

Typical invocation:

```bash
lizard -l rust -l python \
  -T nloc=100 -C 15 -a 7 \
  -W whitelizard.txt -w -t 4 src tools
```

- `-w` prints `path:line: warning: name has …` lines and still exits 1 on any warning.
- `--checkstyle -o artifacts/logs/lizard.xml` produces a CI-consumable XML report
  ([`docs/cli.rst`](https://github.com/terryyin/lizard/blob/master/docs/cli.rst)).
- `-X`, `--csv`, `-H`, and `-o` cover Jenkins XML, spreadsheets, and HTML dashboards
  ([`docs/cli.rst`](https://github.com/terryyin/lizard/blob/master/docs/cli.rst)).
- The README documents pip installation and CLI use; it does not document a Marketplace Action
  ([README installation](https://github.com/terryyin/lizard/blob/master/README.rst),
  [PyPI](https://pypi.org/project/lizard/)). That is a trivial `pip`/`pipx` step on any runner.

### 2.7 Strengths and limitations for Rust

**Strengths**

- One installed tool covers Rust and Python, with the same threshold/exit-code/whitelist model.
- Real function-level CCN, parameter count, NLOC, raw length, nesting depth, and optional
  cognitive/Halstead metrics.
- Exit codes and output formats designed for CI; `.gitignore` aware; thread support.
- Active maintenance with Rust-specific fixes in the current release.
- No compilation or Cargo metadata required; works on a single file or a changed-file list.

**Limitations**

- Rust parsing is a partial token/state-machine parser, not an AST; macros and unusual syntax can
  produce soft failures (README Limitations).
- CCN counts `where` and `?`, does not count `loop`, and double-counts `else if` (§2.2); calibrate
  before enforcing CCN.
- Parameter counting is token/regex based, not type-aware; `self`, patterns, and complex generic
  types can be miscounted (`lizard.py#L338-L371`).
- `-L` measures the raw span including comments/blank lines; use `-T nloc=…` for code-only length.
- Whitelist is name/path based with no line numbers, no wildcards, and no automatic baseline
  generation; function renames require whitelist maintenance.
- No Rust-specific test awareness in `rust.py`; `#[test]`/`#[cfg(test)]` are ordinary tokens, so
  long test functions count unless excluded or whitelisted [INFERENCE from the reader source].
- Default `-L 1000` and `-a 100` mean only CCN is enforced unless you set length/parameter
  thresholds explicitly (`lizard.py#L74`, `#L166-L176`).

---

## 3. `mozilla/rust-code-analysis`

### 3.1 What it is

`rust-code-analysis` is a Rust library plus a CLI (`rust-code-analysis-cli`) and a web/REST
front-end. It parses source with **Tree-sitter** and computes a broad set of maintainability
metrics ([README](https://github.com/mozilla/rust-code-analysis/blob/master/README.md)).
Supported languages include C, C++, Mozcpp, Ccomment, Preproc, Java, JavaScript, Mozjs, Python,
Rust, and TypeScript
([`rust-code-analysis-book/src/languages.md`](https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-book/src/languages.md)).

| Fact | Value | Source |
| --- | --- | --- |
| Stars / forks | 462 / 72 (2026-10-22) | [GitHub API](https://api.github.com/repos/mozilla/rust-code-analysis) |
| Created | 2019-07-16 | [GitHub API](https://api.github.com/repos/mozilla/rust-code-analysis) |
| Last push | 2026-04-06 | [GitHub API](https://api.github.com/repos/mozilla/rust-code-analysis) |
| Latest GitHub release | v0.0.25, 2023-01-13 | [release v0.0.25](https://github.com/mozilla/rust-code-analysis/releases/tag/v0.0.25) |
| License | MPL-2.0 (grammars MIT) | [README licenses](https://github.com/mozilla/rust-code-analysis/blob/master/README.md) |
| Install | `cargo build -p rust-code-analysis-cli`; prebuilt Linux/Windows CLI assets | [README building](https://github.com/mozilla/rust-code-analysis/blob/master/README.md), [release assets](https://github.com/mozilla/rust-code-analysis/releases/tag/v0.0.25) |

The release cadence is stale: the latest GitHub release is from January 2023, although `master`
still receives commits into 2026. That matters for CI adoption: you either build from `master` or
use an old tagged release.

### 3.2 Metrics

The book lists: ABC, BLANK, CC (cyclomatic), CLOC, COGNITIVE, HALSTEAD, LLOC, MI, NARGS, NEXITS,
NOM, NPA, NPM, PLOC, SLOC, and WMC
([`metrics.md`](https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-book/src/metrics.md)).
Metrics are computed over code units (file and nested spaces such as functions/classes); the CLI's
metrics export calls `get_function_spaces(...)` and serializes that space tree
([`rust-code-analysis-cli/src/main.rs`](https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-cli/src/main.rs)).
So per-function CC, COGNITIVE, NARGS and line metrics are available; NOM/NPM/WMC are aggregate
class/file concepts.

### 3.3 CLI: export, not enforcement

The CLI options are `--dump`, `--comments`, `--find`, `--function`, `--count`, `--metrics`,
`--ops`, `--include`, `--exclude`, `--jobs`, `--language-type`, `--output-format`, `--output`,
`--line-start`, `--line-end`, `--warning`, etc. There is **no threshold option** and no
`--fail-on`/`--max-*` flag
([`main.rs` — `Opts`](https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-cli/src/main.rs)).
`--output-format` is limited to cbor, json, toml, and yaml
([`formats.rs`](https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-cli/src/formats.rs)).

Exit behavior is not a metrics gate: `main()` exits 1 only when the concurrent runner returns an
error or when `--output` is a file for `--metrics`/`--ops` (it must be a directory). A successful
metrics dump exits 0 regardless of any metric value
([`main.rs`](https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-cli/src/main.rs)).
CI gating therefore requires a wrapper: export JSON, parse each function's metric, compare against
thresholds, and exit non-zero yourself. That is a custom script with schema coupling, not a
CodeLimit replacement.

Performance-wise, it is a native Rust binary using Tree-sitter and a concurrent file runner with
`-j` jobs. No published benchmark is needed to justify it; the relevant point for this comparison
is accuracy, not speed.

### 3.4 Assessment

Use it when you want **accurate, language-neutral metric data** for dashboards, research, or a
custom quality report. Do not use it as the primary CI gate unless you are willing to own the
post-processing script and the stale-release/build-from-master risk. It is the best *calculator*
in this set, but not a *gate*.

---

## 4. `rust-lang/rust-clippy`

### 4.1 The relevant lints

Clippy is first-party Rust tooling: it runs as a rustc driver and lints compiler HIR/AST, so its
line/argument counts are exact for Rust syntax (including macros expanded by the compiler, with
the usual `from_expansion` caveats).

| Lint | Group / default | Default threshold | What it measures | Source |
| --- | --- | --- | --- | --- |
| `too_many_lines` | `pedantic`, allow-by-default | `too-many-lines-threshold = 100` | Non-comment, non-blank code lines in a function body; strips surrounding braces, skips closures and external-macro expansions | [lint docs](https://rust-lang.github.io/rust-clippy/master/index.html#too_many_lines), [impl](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/functions/too_many_lines.rs) |
| `too_many_arguments` | `complexity`, warn-by-default | `too-many-arguments-threshold = 7` | `decl.inputs.len()`; skips trait impls and non-Rust ABI functions | [lint docs](https://rust-lang.github.io/rust-clippy/master/index.html#too_many_arguments), [impl](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/functions/too_many_arguments.rs) |
| `type_complexity` | `complexity`, warn-by-default | `type-complexity-threshold = 250` | Complexity of types in structs, params, and `let` declarations | [lint docs](https://rust-lang.github.io/rust-clippy/master/index.html#type_complexity), [config](https://doc.rust-lang.org/clippy/lint_configuration.html#type-complexity-threshold) |
| `excessive_nesting` | `complexity`, warn-by-default but **only active when configured** | `excessive-nesting-threshold = 0` (disabled) | Blocks nested beyond the threshold | [lint docs](https://rust-lang.github.io/rust-clippy/master/index.html#excessive_nesting), [config](https://doc.rust-lang.org/clippy/lint_configuration.html#excessive-nesting-threshold) |
| `cognitive_complexity` | `restriction`, allow-by-default | `cognitive-complexity-threshold = 25` | A legacy approximation; docs explicitly say it does **not** measure true cognitive complexity | [lint docs](https://rust-lang.github.io/rust-clippy/master/index.html#cognitive_complexity), [config](https://doc.rust-lang.org/clippy/lint_configuration.html#cognitive-complexity-threshold) |

Two details matter for parity with CodeLimit:

- Clippy's `too_many_lines` counts **code lines**, not the raw span: it removes the outer braces,
  trims blank lines, and counts a line only if it contains code outside `//`/`/* */` comments
  ([`too_many_lines.rs`](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/functions/too_many_lines.rs)).
  That is directly comparable to lizard's `-T nloc=…`, not to lizard's `-L`.
- The threshold test is `line_count > threshold`, so `100` means 101+ code lines fail
  ([`too_many_lines.rs`](https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/functions/too_many_lines.rs)).
  Lizard's warning filter is also strictly `>` (`lizard.py#L638-L646`).

Clippy's own docs recommend `excessive_nesting` and `too_many_lines` instead of
`cognitive_complexity`, and explain that the latter was left in `restriction` "so as to not mislead
users into using this lint as a measurement tool"
([`cognitive_complexity` docs](https://rust-lang.github.io/rust-clippy/master/index.html#cognitive_complexity)).
Treat that as authoritative: do not build a complexity gate on `clippy::cognitive_complexity`.

### 4.2 Configuration and enforcement

Thresholds live in `clippy.toml`:

```toml
too-many-lines-threshold = 100
too-many-arguments-threshold = 7
type-complexity-threshold = 250
# excessive-nesting-threshold = 4   # only if you have measured the baseline
```

The keys and defaults are documented in the Clippy book
([lint configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)).
`too_many_lines` is allow-by-default, so it must be enabled explicitly with
`-W clippy::too_many_lines` (or a crate-level `#![warn]`). `too_many_arguments` and
`type_complexity` are in the warn-by-default `complexity` group.

Warnings can be promoted to build failures with `-D warnings` or, since Cargo 1.97, with
`CARGO_BUILD_WARNINGS=deny` / `[build] warnings = "deny"`; Clippy documents both
([Usage — elevating warnings to errors](https://doc.rust-lang.org/clippy/usage.html)).
The existing `car-game` CI already runs `cargo clippy --all-targets -- -D warnings`
([`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)).

### 4.3 Assessment

Clippy is the **lowest-cost, highest-accuracy Rust gate**, and it is already part of the toolchain
and CI. It is not a CodeLimit replacement because it cannot see Python, has no cross-language
report, and does not offer a real cyclomatic-complexity lint. Its `too_many_lines` +
`too_many_arguments` + `type_complexity` + optional `excessive_nesting` combination is, however,
a strong Rust-native approximation of the "refactor when a function gets too big/complex" rule.

---

## 5. Other notable tools

### 5.1 `boyter/scc`

| Fact | Value | Source |
| --- | --- | --- |
| Stars / forks | 8,842 / 356 (2026-10-22) | [GitHub API](https://api.github.com/repos/boyter/scc) |
| License | MIT | [README](https://github.com/boyter/scc/blob/master/README.md) |
| Language | Go | [GitHub API](https://api.github.com/repos/boyter/scc) |
| Install | `brew install scc`, `go install github.com/boyter/scc/v4@latest`, release binaries, Docker | [README install](https://github.com/boyter/scc/blob/master/README.md) |

`scc` is a very fast line counter with a **file-level** complexity estimate. Its README is explicit
that the estimate is "done only on a file level", is its "own definition", approximates cyclomatic
complexity, and does not build an AST
([README — Complexity Estimates](https://github.com/boyter/scc/blob/master/README.md)).
It can also compute cognitive complexity with `--cognitive` and rank files with
`--by-file -s complexity`, and it has COCOMO/LOCOMO/ULOC/DRY reports
([README help](https://github.com/boyter/scc/blob/master/README.md)).

There is **no function-level length metric and no threshold/gate flag** in `scc -h`; it is a
reporting/statistics tool, not a refactoring gate. The brief's "Cockcroft complexity" label is not
how scc documents itself; scc documents a file-level branch-count approximation, not a per-function
Cockcroft metric.

### 5.2 `minikin/cargo-crap`

| Fact | Value | Source |
| --- | --- | --- |
| Stars / forks | 428 / 13 (2026-10-22) | [GitHub API](https://api.github.com/repos/minikin/cargo-crap) |
| Created | 2026-04-21 | [GitHub API](https://api.github.com/repos/minikin/cargo-crap) |
| Version | 0.9.0 | [`Cargo.toml`](https://github.com/minikin/cargo-crap/blob/main/Cargo.toml) |
| License | MIT | [`Cargo.toml`](https://github.com/minikin/cargo-crap/blob/main/Cargo.toml) |
| Install | `cargo binstall cargo-crap` or `cargo install cargo-crap --locked` (Rust 1.88+) | [README](https://github.com/minikin/cargo-crap/blob/main/README.md) |

`cargo-crap` parses Rust with `syn`, reads an LCOV coverage file, and computes
`CRAP(m) = comp(m)² × (1 − cov(m)/100)³ + comp(m)`, where `comp` is cyclomatic complexity and
`cov` is line coverage ([README](https://github.com/minikin/cargo-crap/blob/main/README.md)).
It has real gates: `--fail-above` (exit 1 above `--threshold`), `--baseline <json>` +
`--fail-regression` (exit 1 when a score rises), `--allow` for function/path globs, and exit 2 for
a failed run ([CLI reference](https://github.com/minikin/cargo-crap/blob/main/docs/reference/cli.md),
[README](https://github.com/minikin/cargo-crap/blob/main/README.md)).

Its README documents optional structural signals — nesting, lines, bumps, boolean operators, stubs,
panics — with regression gates and suggested actions; no calibrated absolute thresholds are
invented ([signals docs](https://github.com/minikin/cargo-crap/blob/main/docs/explanation/signals.md)).
This is the most **ratchet-native** Rust option here, but it requires a working coverage pipeline
(`cargo llvm-cov`) and is only months old. For a Bevy game with a slow/headless test path, that
coverage cost is the main trade-off.

### 5.3 `getcodelimit/codelimit` (baseline)

| Fact | Value | Source |
| --- | --- | --- |
| Stars / forks | 42 / 4 (2026-10-22) | [GitHub API](https://api.github.com/repos/getcodelimit/codelimit) |
| License | GPL-3.0 | [GitHub API](https://api.github.com/repos/getcodelimit/codelimit) |
| Latest release | v0.21.0, 2025-10-21 | [release](https://github.com/getcodelimit/codelimit/releases/tag/v0.21.0) |
| Rust support | **none** | [`codelimit/languages/`](https://github.com/getcodelimit/codelimit/tree/main/codelimit/languages) has C, C++, C#, Java, JavaScript, Python, TypeScript only |

CodeLimit measures only function length in non-comment lines, with hard-coded 30/60 bands; `check`
fails only above 60, has no baseline file, and has no function whitelist
([`docs/codelimit-research.md`](./codelimit-research.md)). Its ratchet exists only because the
official Action passes changed files to `check`
([`docs/codelimit-research.md` §CI](./codelimit-research.md)). `.rs` files are silently skipped, so
pointing it at Rust produces a false green. It stays useful only as the current Python gate or as
a design reference; it is not a candidate for the Rust requirement.

---

## 6. Comparison matrix

### 6.1 Project facts

| Tool | Stars / maturity (2026-10-22) | License | Primary language | Rust parsing method | Install |
| --- | --- | --- | --- | --- | --- |
| **lizard** | 2,562; created 2012; v1.24.2 (2026-10-22); active | MIT | Python | Regex tokenizer + hand-written state machine (`RustStates`/`GoLikeStates`); no AST | `pip install lizard` / `pipx`; standalone `python lizard.py` |
| **rust-code-analysis** | 462; created 2019; latest GitHub release v0.0.25 (2023); master pushed 2026-04 | MPL-2.0 | Rust | Tree-sitter AST | `cargo build -p rust-code-analysis-cli`; Linux/Windows release binaries |
| **rust-clippy** | 13,589; created 2014; ships with Rust; very active | MIT OR Apache-2.0 | Rust | rustc compiler HIR/AST (runs as rustc driver) | `rustup component add clippy` (already in toolchain) |
| **scc** | 8,842; created 2018; active | MIT | Go | Line scanner; explicitly does not build an AST | `brew install scc`, `go install`, binaries, Docker |
| **cargo-crap** | 428; created 2026-04; v0.9.0; active | MIT | Rust | `syn` AST + LCOV coverage | `cargo binstall` / `cargo install` (Rust 1.88+) |
| **codelimit** | 42; created 2022; v0.21.0 (2025-10); one maintainer | GPL-3.0 | Python | Pygments lexer + token grammar; **no Rust** | Homebrew / pipx / release binaries |

### 6.2 Capability matrix

| Tool | Rust + Python? | Metrics measured | Config file / CLI options | Threshold enforcement & exit codes | Baseline / ratchet / whitelist | CI integration ease |
| --- | --- | --- | --- | --- | --- | --- |
| **lizard** | **Yes / Yes** (26 languages listed) | NLOC, CCN, params, tokens, raw length, nesting depth; optional CogC, Halstead, duplicates | No project config; CLI flags `-C`, `-L`, `-a`, `-T`, `-W`, `-x`, `-f`, `-t`; `.gitignore` honored; inline `#lizard forgives` | **Yes.** `-C 15`, `-L 1000`, `-a 100` defaults; warnings > threshold; exit 1 if warnings > `-i` (default 0); exit 2 on usage/IO errors | Whitelist file (name, optional exact path), inline selective forgiveness, global `-i N` budget, changed-file list via `-f`; **no automatic baseline/diff** | **High:** `pip` + one command; non-zero exit; clang/MSVS/checkstyle/CSV/HTML/XML outputs |
| **rust-code-analysis** | Yes / Yes (11 languages) | ABC, BLANK, CC, CLOC, COGNITIVE, HALSTEAD, LLOC, MI, NARGS, NEXITS, NOM, NPA, NPM, PLOC, SLOC, WMC | CLI only: `-m`, `-p`, `-O`, `-o`, `-j`, `-I`, `-X`, `--ls`, `--le`; no thresholds | **No.** Exit 1 only on runner/output errors; metrics values never affect exit | None built in; would need a custom JSON baseline/threshold wrapper | **Low/Medium:** export JSON/TOML/YAML/CBOR, then write a parser/gate; stale release/build risk |
| **rust-clippy** | Yes / No | Code lines (`too_many_lines`), args, type complexity, nesting; `cognitive_complexity` exists but is explicitly unreliable | `clippy.toml` keys: `too-many-lines-threshold`, `too-many-arguments-threshold`, `type-complexity-threshold`, `excessive-nesting-threshold`, `cognitive-complexity-threshold` | **Yes.** Warnings; `-D warnings` or `CARGO_BUILD_WARNINGS=deny` fails the build; lint levels in source/CLI | Per-item `#[allow]`/`#[expect]`, threshold ratchet; no baseline file | **Highest:** already in `car-game` CI; zero extra install; exact Rust semantics |
| **scc** | Yes / Yes (many) | File-level complexity estimate, cognitive, LOC, COCOMO/LOCOMO, ULOC/DRY | `.sccconfig` + CLI (`--by-file`, `-s complexity`, `--cognitive`, `-c`, `-i`, `--ci`) | **No.** No threshold/gate flag in `scc -h`; exits 0 after a report | None | **Medium:** trivial to run, but report-only; would need a custom awk/jq gate on CSV/JSON |
| **cargo-crap** | Yes / No | CRAP (CC × coverage), CC, coverage, duplicate groups, six structural signals | `.cargo-crap.toml` + CLI (`--threshold`, `--fail-above`, `--baseline`, `--fail-regression`, `--allow`, `--format`) | **Yes.** Exit 1 when `--fail-above` or `--fail-regression` trips; exit 2 on run failure | **Strong:** JSON baseline + regression gate, `--allow` globs, signal limits; tests/benches/examples skipped by default | **Medium:** needs `cargo llvm-cov` LCOV + baseline artifact; GitHub Action exists |
| **codelimit** | **No / Yes** (+ C/C++/C#/Java/JS/TS) | Function length only (non-comment lines) | `.codelimit.yml` with `exclude`/`verbose`; `check` takes file list; thresholds hard-coded | **Yes, but fixed:** `check` exits 1 only above 60 LOC; 31–60 warns but exits 0 | Changed-files only via official Action; no CLI baseline, no function whitelist | **High for Python, but false green for Rust** |

Legend: "Yes" means documented and implemented upstream; "No" means not present in the current
source/CLI. Every row is sourced in §§2–5.

---

## 7. Why the numbers differ: length and complexity semantics

This matters more than star counts, because a gate is only useful if the metric is stable and
comparable across tools.

### 7.1 Function length

| Tool | What counts | Nested functions/closures | Comments/blanks |
| --- | --- | --- | --- |
| CodeLimit | Non-comment, non-blank lines in the function scope; header counts; nested function bodies excluded | Excluded from parent | Excluded |
| Clippy `too_many_lines` | Code lines in the body span; outer braces stripped; a line counts only if it has code outside comments | Closures skipped as separate lint targets; parent body still counted | Excluded |
| Lizard `nloc` | Non-comment code lines | Separate `FunctionInfo`; parent does not accrue child NLOC | Excluded |
| Lizard `length` (`-L`) | Raw span `end_line - start_line + 1` | Child span still contributes to parent span | **Included** |
| rust-code-analysis | PLOC/LLOC/SLOC/CLOC per code unit | Tree-sitter spaces | Separately counted |
| scc | File-level lines only | N/A | Separately counted |
| cargo-crap `lines` signal | Function span, signature to closing brace | Nested items not scored as separate functions in CC | Included in span |

**Practical rule:** use `-T nloc=…` (lizard) or Clippy `too_many_lines` when you want a
CodeLimit-like code-line gate. Do **not** use lizard `-L` if you expect parity with CodeLimit or
Clippy; it will be more permissive because comments and blank lines inside the function count.

### 7.2 Complexity

- **Clippy has no true cyclomatic-complexity lint.** It has `cognitive_complexity` (restriction,
  documented as unreliable) and structural proxies (`too_many_lines`, `excessive_nesting`,
  `type_complexity`, `too_many_arguments`).
- **Lizard has a real CCN counter, but its Rust token set is idiosyncratic.** `where` and `?` add
  CCN, `loop` does not, `else if` adds 2, and match arms add N−1. Use it as a relative signal and
  calibrate thresholds against the repo, not against a textbook McCabe value.
- **rust-code-analysis has the most principled CC/COGNITIVE implementation** because it walks a
  Tree-sitter AST, but it does not gate.
- **scc's complexity is file-level and explicitly approximate** ("its own definition", no AST).
- **cargo-crap's CC is per-function via `syn`** and is the basis of CRAP; it is a different metric
  (risk = complexity × untestedness), not a pure complexity gate.

### 7.3 Threshold semantics

All three gate-capable tools fail on **strictly greater than**:
Clippy `line_count > threshold`; lizard `getattr(fun, attr) > limit`; CodeLimit `> 60`.
So `-T nloc=100` and Clippy `too_many_lines-threshold = 100` both mean "101+ code lines fails".

---

## 8. Evaluation and recommendation for `car-game`

### 8.1 Current state

`car-game` is a Rust + Bevy project with Python asset/verification tooling. The current gates are:

- `cargo fmt --check`.
- `cargo clippy --all-targets -- -D warnings`.
- `scripts/check_codelimit.sh <base>`:
  - codelimit on changed `tools/*.py` files (fails only above 60 LOC);
  - Clippy `too_many_lines` on changed `src/*.rs` files, using `clippy.toml`'s
    `too-many-lines-threshold = 100`.
- `cargo test --all-targets`, Python compile checks, asset tests.

Sources: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml),
[`scripts/check_codelimit.sh`](../../scripts/check_codelimit.sh),
[`clippy.toml`](../../clippy.toml), [`scripts/verify.sh`](../../scripts/verify.sh).

The baseline is already known: 4 Python functions above 60 LOC and 5 at 31–60; 4 Rust functions
above Clippy's 100-line threshold — `src/city.rs:64` (203), `src/presentation.rs:118` (175),
`src/smoke.rs:201` (216), `src/vehicle.rs:137` (245). The changed-file ratchet is what keeps those
from blocking unrelated PRs ([`docs/codelimit-research.md`](./codelimit-research.md)).

### 8.2 Fit of each option

| Option | Fit for `car-game` | Why |
| --- | --- | --- |
| **Clippy only** | Good Rust gate, incomplete replacement | Exact Rust metrics, zero new dependency, already in CI. Cannot cover Python, has no true CCN, and `too_many_lines` is pedantic/allow-by-default. |
| **Lizard** | **Best overall fit** | One tool for Rust + Python, explicit length/CCN/param gates, non-zero exit, whitelist and inline forgiveness, changed-file ratchet via `-f`. Rust CCN is heuristic, so calibrate. |
| **rust-code-analysis** | Poor gate, good reporting | Accurate Tree-sitter metrics but no thresholds; would require a custom JSON gate and a build-from-master/old-release decision. |
| **scc** | Reporting only | File-level complexity and repository stats; no function-level gate. |
| **cargo-crap** | Optional deeper Rust gate | Excellent baseline/regression model and structural signals, but needs LCOV coverage and is very new. Bevy test/coverage cost is the blocker. |
| **Keep CodeLimit** | No | No Rust support; false green on `.rs`; hard-coded thresholds; small project. |

### 8.3 Recommendation

**Adopt lizard as the cross-language function-limit gate; keep Clippy as the Rust-native
precision gate; retire CodeLimit.**

Concretely:

1. **Lizard** enforces function length, parameter count, and a calibrated CCN on changed Rust and
   Python files, replacing codelimit's Python-only role and adding the Rust coverage CodeLimit
   lacks.
2. **Clippy** remains the blocking Rust lint gate (`-D warnings`) and keeps `too_many_lines` as a
   second, AST-accurate length check. This redundancy is cheap because Clippy is already installed
   and already runs.
3. **Do not** use `clippy::cognitive_complexity` as a gate; if you want a complexity signal beyond
   lizard CCN, enable `excessive_nesting` only after measuring the baseline and setting a threshold
   in `clippy.toml`.
4. **Do not** replace Clippy with lizard. Lizard is the cross-language policy tool; Clippy is the
   Rust correctness/precision tool. They answer different questions.

### 8.4 Concrete setup

**Install (pin the version; 1.24.2 is current as of this note):**

```bash
pipx install 'lizard==1.24.2'
# or: python3 -m pip install --user 'lizard==1.24.2'
```

**Create `whitelizard.txt` (empty initially, or grandfather specific functions):**

```text
# whitelizard.txt — car-game
# Exact paths as lizard prints them. Prefer inline selective forgiveness for one-off exceptions.
# src/vehicle.rs:update_vehicle
# tools/prepare_fleet.py:build_fleet
```

**Add `scripts/check_limits.sh` (derived from the existing script):**

```bash
#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

base="${1:-origin/main}"
changed_files() {
  { git diff --name-only --diff-filter=ACM "$base" --cached
    git diff --name-only --diff-filter=ACM "$base"
    git ls-files --others --exclude-standard
  } | sort -u
}

mapfile -t py_files < <(changed_files | grep '\.py$' || true)
mapfile -t rs_files < <(changed_files | grep '\.rs$' || true)

# Python: preserve CodeLimit's >60 code-line policy.
if [ "${#py_files[@]}" -gt 0 ]; then
  echo "== lizard (Python, changed vs $base) =="
  lizard -l python -T nloc=60 -C 15 -a 7 -W whitelizard.txt -w "${py_files[@]}"
fi

# Rust: preserve Clippy's 100 code-line policy; calibrate CCN before tightening.
if [ "${#rs_files[@]}" -gt 0 ]; then
  echo "== lizard (Rust, changed vs $base) =="
  lizard -l rust -T nloc=100 -C 20 -a 7 -W whitelizard.txt -w "${rs_files[@]}"
fi

echo "== lizard gates passed =="
```

Notes:

- `-T nloc=…` is the CodeLimit/Clippy-comparable code-line metric; `-L` would count comments and
  blanks and is deliberately not used.
- `-C 20` is a **starting point**, not a calibrated value. Lizard counts `where` and `?`, which are
  common in Bevy systems, so measure first. Run this once to print every function sorted by CCN
  (`-C 0` makes every function a "warning"; the command exits 1, which is expected during
  calibration):
  ```bash
  lizard -l rust -C 0 -L 1000 -a 100 -s cyclomatic_complexity -w src
  ```
  Then set `-C` at the desired percentile and ratchet down. If CCN noise is unacceptable, gate
  only `nloc`/`parameter_count` with lizard and leave complexity to Clippy's structural lints.
- `-a 7` matches Clippy's `too_many_arguments` default. Bevy systems with many `Query`/`Res`
  parameters may need a deliberate exception via `// #lizard forgives(parameter_count)` or a
  whitelist entry; do not raise the global threshold just for systems.
- The whitelist path must match lizard's output path exactly; the script runs from the repo root,
  so relative paths should be stable. Prefer inline forgiveness for one-offs and prune the
  whitelist quarterly.
- If the changed-file list is huge, write it to a file and pass `-f changed.txt` instead of
  expanding it on the command line.

**CI change (replace the codelimit install/step):**

```yaml
- name: Install lizard
  run: python3 -m pip install --user 'lizard==1.24.2'

- name: Function/complexity gates
  run: scripts/check_limits.sh "origin/${{ github.base_ref }}"
```

Keep the existing Clippy step. If you want CI annotations, add
`--checkstyle -o artifacts/logs/lizard.xml` to the lizard invocations and upload the artifact.

**Clippy configuration to keep:**

```toml
# clippy.toml — car-game
too-many-lines-threshold = 100
too-many-arguments-threshold = 7
# type-complexity-threshold = 250     # default; complexity group is warn-by-default
# excessive-nesting-threshold = 4     # only after measuring the baseline
```

Run:

```bash
cargo clippy --all-targets -- -D warnings -W clippy::too_many_lines
```

This is essentially what `scripts/check_codelimit.sh` already does for Rust, so the Clippy half
needs no new policy.

### 8.5 Migration steps

1. Add `whitelizard.txt` at the repo root (empty or with explicit grandfather entries).
2. Add `scripts/check_limits.sh`; keep the existing changed-file collection logic.
3. Update `.github/workflows/ci.yml` to install pinned lizard and call the new script.
4. Update `scripts/verify.sh` to call the new script.
5. Remove `.codelimit.yml`, `.codelimit_cache/` from `.gitignore`, and the codelimit references from
   the CI/verify scripts after one green run. Keep `docs/codelimit-research.md` as historical
   context or add a short "superseded by" note.
6. Calibrate `-C` on the current `src/` and `tools/` trees; record the baseline and the ratchet
   target in `docs/`.
7. Do not tighten Clippy's `too-many-lines-threshold` below 100 until the four known Rust
   violations (`city.rs:64`, `presentation.rs:118`, `smoke.rs:201`, `vehicle.rs:137`) are
   refactored or explicitly grandfathered.

### 8.6 Risks and mitigations

| Risk | Mitigation |
| --- | --- |
| Lizard Rust CCN over-counts `where`/`?`, causing noisy failures | Calibrate with `-C 0` first; start at 20; use selective forgiveness; or gate only NLOC/params and rely on Clippy for structural complexity. |
| Lizard's partial parser misreads macro-heavy Bevy code | Treat lizard as a policy heuristic, not a compiler; keep Clippy as the authoritative Rust check. Spot-check warnings before enabling the gate. |
| Whitelist drift / stale entries | Prefer inline `#lizard forgives(metric)` with a reason; review `whitelizard.txt` quarterly; names are not checked for existence, so stale entries silently do nothing. |
| Python threshold regression when moving off codelimit | Run lizard with `-T nloc=60` on Python; compare the first report with the codelimit baseline (4 functions >60, 5 at 31–60). |
| `pip` supply-chain / version drift | Pin `lizard==1.24.2` and install with `pipx` or a hashed requirements file; the MIT license is permissive. |
| Bevy systems legitimately exceed 7 params | Use a documented per-function forgiveness comment rather than raising the global threshold; or set `-a 10` for Rust only if the team deliberately chooses a Bevy-specific policy. |

---

## 9. Executive summary (end)

- **Most mature CodeLimit-like tool with Rust support: `terryyin/lizard`.** Dedicated Rust reader,
  CCN/NLOC/params/length/nesting metrics, optional cognitive complexity, explicit `-C`/`-T`/`-a`
  thresholds, non-zero exit codes, `.gitignore` integration, whitelist and inline forgiveness, and
  CI-friendly output formats. It is a token/state-machine analyzer, not an AST parser, so its Rust
  CCN is heuristic: `where` and `?` count, `loop` does not.
- **`mozilla/rust-code-analysis` is not a gate.** It has excellent Tree-sitter metrics and
  JSON/TOML/YAML/CBOR export, but no threshold flags and no metric-based exit code; gating requires
  a custom wrapper. Its latest GitHub release is from 2023 even though master still moves.
- **Clippy is the best Rust-native gate.** `too_many_lines` (pedantic, default 100),
  `too_many_arguments` (complexity, default 7), `type_complexity` (complexity, default 250), and
  optional `excessive_nesting` give compiler-accurate checks with zero new dependencies. Do not
  gate on `cognitive_complexity`; Clippy's own docs say it is not a real measurement.
- **scc and CodeLimit do not meet the bar.** scc is file-level/report-only; CodeLimit has no Rust
  support and hard-coded thresholds. `cargo-crap` is a strong optional Rust ratchet if you are
  willing to run LCOV coverage.
- **Recommended for `car-game`:** keep `cargo clippy --all-targets -- -D warnings` and
  `too_many_lines = 100`; replace CodeLimit with pinned lizard 1.24.2 in the changed-file gate;
  use `-T nloc=60 -C 15 -a 7` for Python and `-T nloc=100 -C 20 -a 7` for Rust, then calibrate
  `-C` against the repo; use `whitelizard.txt`/inline forgiveness for exceptions; keep the
  changed-file ratchet so the four known long Rust functions and the existing Python debt do not
  block unrelated work.

---

## 10. Sources

**lizard**

- Repository: <https://github.com/terryyin/lizard>
- README (metrics, exit code, `.gitignore`, limitations): <https://github.com/terryyin/lizard/blob/master/README.rst>
- CLI reference: <https://github.com/terryyin/lizard/blob/master/docs/cli.rst>
- Extensions/warnings (cognitive, whitelist, forgiveness): <https://github.com/terryyin/lizard/blob/master/docs/extensions-and-warnings.rst>
- Rust reader: <https://github.com/terryyin/lizard/blob/master/lizard_languages/rust.py>
- Go-like state machine: <https://github.com/terryyin/lizard/blob/master/lizard_languages/golike.py>
- Tokenizer condition base: <https://github.com/terryyin/lizard/blob/master/lizard_languages/code_reader.py>
- Main CLI/whitelist/exit logic: <https://github.com/terryyin/lizard/blob/master/lizard.py>
- Release 1.24.2: <https://github.com/terryyin/lizard/releases/tag/1.24.2>
- PyPI metadata: <https://pypi.org/pypi/lizard/json>
- License: <https://github.com/terryyin/lizard/blob/master/LICENSE.txt>

**rust-code-analysis**

- Repository: <https://github.com/mozilla/rust-code-analysis>
- README: <https://github.com/mozilla/rust-code-analysis/blob/master/README.md>
- Metrics: <https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-book/src/metrics.md>
- Languages: <https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-book/src/languages.md>
- CLI options and exit behavior: <https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-cli/src/main.rs>
- Output formats: <https://github.com/mozilla/rust-code-analysis/blob/master/rust-code-analysis-cli/src/formats.rs>
- Latest release: <https://github.com/mozilla/rust-code-analysis/releases/tag/v0.0.25>

**rust-clippy**

- Repository: <https://github.com/rust-lang/rust-clippy>
- Lint list (anchors for each lint): <https://rust-lang.github.io/rust-clippy/master/index.html>
- `too_many_lines`: <https://rust-lang.github.io/rust-clippy/master/index.html#too_many_lines>
- `too_many_arguments`: <https://rust-lang.github.io/rust-clippy/master/index.html#too_many_arguments>
- `type_complexity`: <https://rust-lang.github.io/rust-clippy/master/index.html#type_complexity>
- `excessive_nesting`: <https://rust-lang.github.io/rust-clippy/master/index.html#excessive_nesting>
- `cognitive_complexity`: <https://rust-lang.github.io/rust-clippy/master/index.html#cognitive_complexity>
- Lint configuration: <https://doc.rust-lang.org/clippy/lint_configuration.html>
- Usage / `-D warnings`: <https://doc.rust-lang.org/clippy/usage.html>
- `too_many_lines` implementation: <https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/functions/too_many_lines.rs>
- `too_many_arguments` implementation: <https://github.com/rust-lang/rust-clippy/blob/master/clippy_lints/src/functions/too_many_arguments.rs>
- License: <https://github.com/rust-lang/rust-clippy/blob/master/Cargo.toml>

**scc**

- Repository: <https://github.com/boyter/scc>
- README (complexity estimates, help, install): <https://github.com/boyter/scc/blob/master/README.md>

**cargo-crap**

- Repository: <https://github.com/minikin/cargo-crap>
- README (CRAP formula, gates, exit codes): <https://github.com/minikin/cargo-crap/blob/main/README.md>
- CLI reference: <https://github.com/minikin/cargo-crap/blob/main/docs/reference/cli.md>
- Structural signals: <https://github.com/minikin/cargo-crap/blob/main/docs/explanation/signals.md>
- Version/license: <https://github.com/minikin/cargo-crap/blob/main/Cargo.toml>

**CodeLimit / car-game**

- Repository: <https://github.com/getcodelimit/codelimit>
- Languages (no Rust): <https://github.com/getcodelimit/codelimit/tree/main/codelimit/languages>
- Release v0.21.0: <https://github.com/getcodelimit/codelimit/releases/tag/v0.21.0>
- This repo's prior research: [`docs/codelimit-research.md`](./codelimit-research.md)
- This repo's current config: [`clippy.toml`](../../clippy.toml), [`.codelimit.yml`](../../.codelimit.yml)
- This repo's current gate: [`scripts/check_codelimit.sh`](../../scripts/check_codelimit.sh)
- This repo's CI: [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml)
