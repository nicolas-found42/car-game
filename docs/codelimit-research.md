# CodeLimit research (for car-game)

> **Research note — 2026-10-22.** Primary sources only: the `getcodelimit/codelimit` repository, the
> docs site source (`getcodelimit/getcodelimit.github.io`), the official GitHub Action repo
> (`getcodelimit/codelimit-action`), and the published release / Homebrew / Docker Hub pages.
> No third-party blog posts were used.
>
> **Why this file lives in `docs/`:** car-game keeps notes under `docs/`, and
> `docs/agents/domain.md` only defines glossary/ADR conventions — there is no research-note
> convention, so `docs/` root is the sensible location.
>
> **Version studied:** codelimit **v0.21.0** — latest release, 2025-10-21
> ([releases/latest](https://github.com/getcodelimit/codelimit/releases/latest),
> [CHANGELOG](https://github.com/getcodelimit/codelimit/blob/main/CHANGELOG.md#v0210-2025-10-21)).
> It is a **Python** CLI (not Rust), GPL-3.0-or-later, maintained by one author.

---

## 1. Overview — what it is, what problem it solves

CodeLimit is "Your Refactoring Alarm": a zero-configuration CLI that measures **one metric — function
length in lines of code** — and tells a developer when it is time to refactor. Its stated design
principle is a "non-controversial code metric" (most developers agree longer functions are harder to
maintain), actionable results, sane defaults, and the ability to check only new/modified code so a
team with existing technical debt can adopt it immediately
([README](https://github.com/getcodelimit/codelimit/blob/main/README.md),
[docs index](https://getcodelimit.github.io/),
[why-function-length](https://getcodelimit.github.io/why-function-length/)). It deliberately does
**not** compute complexity, duplication, or other metrics — the docs position it against tools like
Lizard and Radon that "measure a lot more metrics" and overwhelm developers
([why-function-length](https://getcodelimit.github.io/why-function-length/),
[FAQ](https://getcodelimit.github.io/faq/)).

---

## 2. How it works

### 2.1 Architecture / pipeline

It is a Typer/Click CLI (`codelimit` entry point,
[pyproject.toml#L28](https://github.com/getcodelimit/codelimit/blob/main/pyproject.toml#L28)) with
commands `scan`, `report`, `findings`, `check`, `badge`
([`codelimit/__main__.py#L27-L107`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L27-L107)).
The pipeline for a scan/check is:

1. **Walk** the tree with `os.walk`, skipping dot-files/dot-dirs, applying an exclusion `PathSpec`
   ([`Scanner.py#L60-L78`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L60-L78)).
2. **Pick a lexer** with Pygments `get_lexer_for_filename` and keep the file only if the lexer's class
   name matches a registered `Languages.by_name` entry — otherwise the file is **silently skipped**
   ([`Scanner.py#L67-L71`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L67-L71)).
3. **Lex** the source into tokens with that Pygments lexer
   ([`Scanner.py#L118-L126`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L118-L126)).
4. **Language-specific scope extraction**: each `Language` subclass implements `extract_headers`
   (function signature patterns) and `extract_blocks` (bodies), built from hand-written token
   grammars/predicates — not a real parser
   ([`Language.py#L11-L26`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Language.py#L11-L26),
   [`Python.py#L13-L22`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/languages/Python.py#L13-L22),
   [`scope_utils.py#L24-L38`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/scope/scope_utils.py#L24-L38)).
5. **Measure**: length = number of distinct source lines containing at least one non-whitespace,
   non-comment token inside the scope; the header line counts; **nested function bodies are excluded
   from the parent's length**
   ([`scope_utils.py#L134-L148`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/scope/scope_utils.py#L134-L148),
   [`source_utils.py#L44-L52`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/source_utils.py#L44-L52)).
6. **Persist**: `scan` writes a JSON report to `.codelimit_cache/codelimit.json` (with a
   `CACHEDIR.TAG` and self-ignoring `.gitignore`), cached per file by MD5 checksum so unchanged files
   are not re-analyzed
   ([`scan.py#L13-L29`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/scan.py#L13-L29),
   [`Scanner.py#L88-L106`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L88-L106)).
   `report`/`findings` read that file and render text or Markdown
   ([`report.py`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/report.py),
   [`findings.py`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/findings.py)).

### 2.2 The metric: thresholds, categories, what is enforced

Function length is bucketed into four categories
([docs table](https://getcodelimit.github.io/why-function-length/),
[`common/utils.py#L19-L29`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/utils.py#L19-L29)):

| Category | Lines of code | Effect |
| --- | --- | --- |
| Easy | 1–15 | none |
| Verbose | 16–30 | none |
| Hard-to-maintain ⚠ | 31–60 | reported as a warning; `check` still exits 0 |
| Unmaintainable ✖ | > 60 | `check` **fails** |

Exact code boundaries: the profile uses `<=15`, `<=30`, `<=60`, else
([`utils.py#L22-L29`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/utils.py#L22-L29));
`check` counts `30 < v <= 60` as hard-to-maintain and `v > 60` as unmaintainable
([`CheckResult.py#L17-L20`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/CheckResult.py#L17-L20)),
and only passes measurements with `v > 30` to the report
([`check.py#L66-L71`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/check.py#L66-L71)).
Note a small docs/code discrepancy: the docs table labels "60+ lines" as unmaintainable, while the
code fails only strictly above 60 (a 60-line function is hard-to-maintain).

**Only function length is measured.** There is no file-length rule, no cyclomatic complexity, no
parameter-count rule, no duplication metric; `scan_file` produces exactly one `Measurement` per
function (name, start, end, line count)
([`Scanner.py#L134-L152`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L134-L152)).
Comments and blank lines are not counted
([why-function-length](https://getcodelimit.github.io/why-function-length/),
[`source_utils.py#L44-L52`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/source_utils.py#L44-L52)).

### 2.3 Configuration file: `.codelimit.yml`

The config file is **`.codelimit.yml`** in the root of the scanned tree — not `codelimit.yaml`, and
there is no `.yaml` fallback
([`Configuration.py#L15-L24`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Configuration.py#L15-L24),
[configuration docs](https://getcodelimit.github.io/configuration/)).

| Key | Type | Default | Effect |
| --- | --- | --- | --- |
| `exclude` | list of gitignore-style patterns | `[]` | appended to the built-in exclusion list ([`Configuration.py#L21-L22`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Configuration.py#L21-L22)) |
| `verbose` | bool | `false` | enables INFO logging ([`Configuration.py#L23-L24`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Configuration.py#L23-L24)); not mentioned on the docs configuration page |

There are **no keys for thresholds, rule selection, languages, or baselines** — the 30/60 boundaries
are hard-coded. The loader reads only these two keys and does not validate the file, so unknown keys
are ignored \[INFERENCE from
[`Configuration.py#L19-L24`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Configuration.py#L19-L24)\].
There is no published JSON Schema.

Example (from the docs):

```yaml
exclude:
  - docs/javascripts/asciinema-player.min.js
```

**Config discovery:** the file is looked up only at the path passed to the command (`scan`/`report`/
`findings`/`badge` use their path argument; `check` always loads from the current working directory);
there is no upward search
([`__main__.py#L43,L60,L73,L93,L103`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L43)).

**Built-in default exclusions** (extended by `--exclude`, by `.codelimit.yml`, and by the root
`.gitignore`):

```
.bzr .direnv .eggs .git .git-rewrite .hg .ipynb_checkpoints .mypy_cache .nox .pants.d
.pytest_cache .pytype .ruff_cache .svn .tox .venv .vscode __pypackages__ _build buck-out
build dist node_modules venv test tests acceptancetests integrationtests
```

Source: [`Scanner.py#L170-L199`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L170-L199).
The docs list omits the last two entries (`acceptancetests`, `integrationtests`)
([configuration docs](https://getcodelimit.github.io/configuration/)) — the source is authoritative.
Note `target` is **not** in the default list.

### 2.4 Exclusions / ignore behaviour

- `.codelimit.yml` `exclude` and the repeatable `--exclude` CLI option on `scan`/`check`
  ([`__main__.py#L32-L41,L79-L91`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L32-L41)).
- The **root `.gitignore`** of the scanned tree is read and its lines appended to the exclusion spec
  ([`Scanner.py#L153-L166`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L153-L166)).
  Only that single root file is read — nested `.gitignore` files are not walked.
- `# nocl` (case-insensitive) on the **same line as the function name** excludes that function
  ([configuration docs](https://getcodelimit.github.io/configuration/),
  [`source_utils.py#L57-L65`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/source_utils.py#L57-L65),
  [`scope_utils.py#L92-L95`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/scope/scope_utils.py#L92-L95)).

---

## 3. Install & run

Documented installation paths
([standalone-usage docs](https://getcodelimit.github.io/getting-started/standalone-usage/)):

```shell
brew install codelimit                 # Homebrew (formula exists, v0.21.0)
pipx install codelimit                 # isolated Python env
python -m pip install codelimit        # into current Python
```

Platform binaries (`codelimit-macos`, `codelimit-linux`, `codelimit.exe`, built with PyInstaller) are
attached to each release
([releases](https://github.com/getcodelimit/codelimit/releases/latest),
[release workflow](https://github.com/getcodelimit/codelimit/blob/main/.github/workflows/release.yml),
[binary names in the Action](https://github.com/getcodelimit/codelimit-action/blob/main/src/codelimit.ts#L12-L15)).
A published Docker image exists: `getcodelimit/codelimit:latest` (amd64/arm64, updated 2025-10-21)
([Docker Hub](https://hub.docker.com/r/getcodelimit/codelimit),
[publish step](https://github.com/getcodelimit/codelimit/blob/main/.github/workflows/release.yml#L109-L116));
the image's `ENTRYPOINT` is `codelimit`
([Dockerfile#L13](https://github.com/getcodelimit/codelimit/blob/main/Dockerfile#L13)), so e.g.
`docker run --rm -v "$PWD:/src" getcodelimit/codelimit scan /src` \[command constructed from the
Dockerfile/ENTRYPOINT; not documented verbatim\].

Requirements: Python `>=3.10,<3.16`
([pyproject.toml#L6](https://github.com/getcodelimit/codelimit/blob/main/pyproject.toml#L6)); runtime
deps include `pygments`, `pathspec`, `typer`, `pyyaml`
([pyproject.toml#L9-L20](https://github.com/getcodelimit/codelimit/blob/main/pyproject.toml#L9-L20)).
The Homebrew formula confirms v0.21.0 and GPL-3.0-or-later
([formulae.brew.sh](https://formulae.brew.sh/formula/codelimit)).

Commands:

```shell
codelimit scan path/to/codebase            # analyze + write .codelimit_cache/codelimit.json
codelimit report [path] [--format text|markdown] [--diff other_report.json]
codelimit findings [path] [--full] [--format text|markdown]
codelimit check a.py b.py src/ [--quiet] [--exclude "*.generated.py"]
codelimit badge                            # print + copy badge Markdown to clipboard
```

Pre-commit hook (from the docs / hook definition):

```yaml
-   repo: https://github.com/getcodelimit/codelimit
    rev: 0.6.2
    hooks:
    - id: codelimit
```

([development-workflow docs](https://getcodelimit.github.io/getting-started/development-workflow/),
[`.pre-commit-hooks.yaml#L1-L8`](https://github.com/getcodelimit/codelimit/blob/main/.pre-commit-hooks.yaml#L1-L8)).
Caveat: the hook declares `types: [file, python]` and entry `codelimit check --quiet`, so it only
runs on Python files; the docs' `rev: 0.6.2` example is far behind the current release.

---

## 4. Language support — does it support Rust?

**No.** The registry contains exactly seven languages: C, C++, C#, Java, JavaScript, Python,
TypeScript
([`languages/__init__.py#L10-L22`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/languages/__init__.py#L10-L22),
[FAQ](https://getcodelimit.github.io/faq/)); the `codelimit/languages/` directory has no Rust module
([directory listing](https://github.com/getcodelimit/codelimit/tree/main/codelimit/languages)).
Because the scanner keeps a file only when its Pygments lexer name is in `Languages.by_name`
([`Scanner.py#L67-L71`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L67-L71)),
`.rs` files are **skipped silently** — no warning, no error.

Rust support is an open request: issue **#105 "Add Rust support"** (opened 2026-10-13 by the
maintainer, no description; only an auto-created branch) — no implementation as of this note
([issue #105](https://github.com/getcodelimit/codelimit/issues/105)). Related open issue **#68 "Add
support for tree-sitter grammars"** shows the current architecture is hand-written token matching,
not tree-sitter ([issue #68](https://github.com/getcodelimit/codelimit/issues/68)). Adding Rust would
mean writing a `Rust(Language)` subclass implementing `extract_headers`/`extract_blocks`
([`Language.py#L11-L26`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Language.py#L11-L26)).

Parsers used per language: **Pygments lexers only** for tokenization, plus codelimit's own
grammar/token-predicate machinery for scopes; no external linter is invoked
([`Scanner.py#L27`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L27),
[`Python.py`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/languages/Python.py)).

---

## 5. CI integration (official GitHub Action, ratchet)

There **is** an official Action: `getcodelimit/codelimit-action@v1`
([action repo](https://github.com/getcodelimit/codelimit-action)). It is a Docker action
([`action.yml#L6-L8`](https://github.com/getcodelimit/codelimit-action/blob/main/action.yml#L6-L8))
that downloads the codelimit release binary at run time (default `codelimit_version: latest`) and
executes `scan .`, then `check` on changed files, then updates the report branch
([`action.ts#L130-L152`](https://github.com/getcodelimit/codelimit-action/blob/main/src/action.ts#L130-L152)).

Inputs ([`action.yml#L9-L21`](https://github.com/getcodelimit/codelimit-action/blob/main/action.yml#L9-L21)):

| Input | Default | Purpose |
| --- | --- | --- |
| `token` | `${{ github.token }}` | store results / PR comment |
| `check` | `true` | check changed files |
| `codelimit_version` | `latest` | pin a codelimit release |

Workflow YAML (docs + action README verbatim):

```yaml
name: 'codelimit'

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - name: 'Checkout sources'
        uses: actions/checkout@v4

      - name: 'Run CodeLimit'
        uses: getcodelimit/codelimit-action@v1
```

([development-workflow docs](https://getcodelimit.github.io/getting-started/development-workflow/),
[action README](https://github.com/getcodelimit/codelimit-action#readme)).
Side effects: it pushes `badge.svg`, `report.json`, `codelimit.md` (and a comment-state file) to a
`_codelimit_reports` branch and posts/updates a PR comment
([`action.ts#L49-L84`](https://github.com/getcodelimit/codelimit-action/blob/main/src/action.ts#L49-L84)).

**Baseline / ratchet:** the CLI has **no `--since`/baseline flag** — `check` takes an explicit list
of files ([`__main__.py#L76-L95`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L76-L95)).
The ratchet is implemented by the Action: it calls the GitHub compare API and passes only
`added`/`modified` files to `codelimit check`
([`utils.ts#L4-L23`](https://github.com/getcodelimit/codelimit-action/blob/main/src/utils.ts#L4-L23),
[`action.ts#L86-L95`](https://github.com/getcodelimit/codelimit-action/blob/main/src/action.ts#L86-L95)).
Caveats: if there are no changed files it skips; if the compare API is unavailable or the event is
`workflow_run`/undefined it falls back to `['.']`, i.e. it checks the **entire repo**
([`utils.ts#L5-L6,L12-L13`](https://github.com/getcodelimit/codelimit-action/blob/main/src/utils.ts#L5-L13)).
There is a separate `report --diff <old.json>` option, but it only renders a comparison — it does not
gate CI ([`__main__.py#L53-L61`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L53-L61),
[CHANGELOG v0.18.0](https://github.com/getcodelimit/codelimit/blob/main/CHANGELOG.md#v0180-2025-01-07)).
No Marketplace listing was found at `github.com/marketplace/actions/codelimit` (HTTP 404, checked
2026-10-22); the Action is consumed directly by `uses:`.

---

## 6. Exit codes and output format

Exit codes (from source):

| Command | Code | Condition |
| --- | --- | --- |
| `check` | `1` | at least one checked function is `> 60` LOC ([`check.py#L33,L40`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/check.py#L33-L40)); 31–60 warnings do **not** fail |
| `check` | `0` | otherwise |
| `report` / `findings` | `1` | no `.codelimit_cache/codelimit.json`, or report version mismatch ([`utils.py#L34-L42`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/utils.py#L34-L42)) |
| `badge` | `1` | git remote/branch not resolvable ([`__main__.py#L105-L107`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L105-L107)) |
| `app upload` | `1` | upload failure ([`utils.py#L57-L58`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/utils.py#L57-L58)); command is disabled in the CLI, see §7 |
| `scan` | `0` | always (it reports, it does not gate) ([`scan.py`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/scan.py)) |
| any command | `2` | Click/Typer usage errors, e.g. missing/invalid arguments ([Click exit-code docs](https://click.palletsprojects.com/en/stable/exceptions/)) |

Output: `check` prints one colored line per flagged function in the form
`path:line:column: <loc> <⚠|✖> <name>` plus a summary ("N files checked, M functions need
refactoring." or "Refactoring not necessary, happy coding!")
([`utils.py#L118-L140`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/utils.py#L118-L140),
[`CheckResult.py#L26-L45`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/CheckResult.py#L26-L45)).
`--quiet` suppresses output only when there is nothing to report
([`check.py#L34-L39`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/check.py#L34-L39)).
`report`/`findings` support `--format text|markdown` (Markdown tables of language, files, LOC,
functions, warnings/failures, plus a findings table — see the example report in
[PR #65](https://github.com/getcodelimit/codelimit/pull/65)). The machine-readable artifact is the
JSON report at `.codelimit_cache/codelimit.json`
([`scan.py#L15-L16`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/scan.py#L15-L16)).
There is **no SARIF output and no auto-fix**: the CLI has no fix/format command, and the tool is
described purely as an alarm
([`__main__.py#L27-L107`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L27-L107),
[README](https://github.com/getcodelimit/codelimit/blob/main/README.md)).

---

## 7. Pricing / free tier

No paid tier is documented anywhere I could find as of 2026-10-22:

- No pricing page: `https://getcodelimit.github.io/pricing/` → HTTP 404.
- Docs contain no pricing/plan pages (docs repo contents:
  [getcodelimit.github.io](https://github.com/getcodelimit/getcodelimit.github.io) — index, why-function-length,
  configuration, development, FAQ, getting-started).
- No Marketplace listing (404), so no marketplace billing.
- License is GPL-3.0-or-later
  ([pyproject.toml#L8](https://github.com/getcodelimit/codelimit/blob/main/pyproject.toml#L8),
  [formula](https://formulae.brew.sh/formula/codelimit)).
- The GitHub-App integration is currently **disabled**: `cli.add_typer(app.app, name="app", ...)` is
  commented out ([`__main__.py#L25`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L25)),
  even though the docs usage block still lists `app`
  ([standalone-usage docs](https://getcodelimit.github.io/getting-started/standalone-usage/)). The
  upload target default `https://codelimit.vercel.app/api/upload` returns 404
  ([`app.py#L44-L48`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/app.py#L44-L48)).

So: free, self-hosted, GPL OSS, with a GitHub Action and Docker image; **no org/team/paid features
relevant to a solo dev** (no accounts, no dashboards — results are committed to a
`_codelimit_reports` branch and posted as a PR comment).

---

## 8. Notable behaviour (performance, monorepos, ignore files, discovery)

- **Performance:** no published benchmarks. `scan` caches per-file results by MD5 in
  `.codelimit_cache/codelimit.json` and skips unchanged files; `check` has no cache and re-lexes the
  files it is given ([`Scanner.py#L88-L106`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L88-L106),
  [`check.py#L53-L71`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/check.py#L53-L71)).
  A performance PR landed in v0.17.0 with no numbers
  ([CHANGELOG](https://github.com/getcodelimit/codelimit/blob/main/CHANGELOG.md#v0170-2024-12-29),
  [PR #65](https://github.com/getcodelimit/codelimit/pull/65)).
- **Monorepos:** no workspace/package awareness. One config file, loaded from the single scanned
  root; one report/cache at that root
  ([`Configuration.py#L15-L24`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Configuration.py#L15-L24),
  [`scan.py#L15-L16`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/scan.py#L15-L16)).
  You can scope it with path patterns in `exclude` or by pointing commands at subdirectories.
- **Ignore files:** root `.gitignore` + `exclude` + `--exclude` + `nocl` comments (see §2.4). Only the
  root `.gitignore` is read; nested ignore files are not
  ([`Scanner.py#L153-L166`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L153-L166)).
- **Silent skips:** unsupported extensions and unparseable files are dropped without any diagnostic
  ([`Scanner.py#L66-L78`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/common/Scanner.py#L66-L78),
  [`check.py#L54-L58`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/commands/check.py#L54-L58)),
  which makes "all green" ambiguous on a mixed-language repo.
- **Docs/code drift (v0.21.0):** docs show a global `--exclude` and an `app` command; the code has
  `--exclude` only on `scan`/`check` and the `app` command commented out
  ([docs usage](https://getcodelimit.github.io/getting-started/standalone-usage/),
  [`__main__.py#L23-L41`](https://github.com/getcodelimit/codelimit/blob/main/codelimit/__main__.py#L23-L41)).
- **Project health:** 42 stars, one maintainer, latest release 2025-10-21, Rust-support issue opened
  2026-10-13 with no description
  ([org repos](https://github.com/orgs/getcodelimit/repositories),
  [issue #105](https://github.com/getcodelimit/codelimit/issues/105)).

---

## 9. Fit for car-game

**Repo facts used** (local paths): single Rust package, no `[workspace]` — `Cargo.toml` is a plain
`[package]` with Bevy 0.20; Rust sources are `src/{main,presentation,vehicle,smoke,diagnostics,roster,city}.rs`;
Python helpers live in `tools/*.py` (7 files) with shell scripts in `scripts/`; `.gitignore` contains
`/target/`, `/artifacts/`, `/dist/`, `__pycache__/`; CI is `.github/workflows/ci.yml` (macOS 14:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`, plus
Python compile/asset checks).

**Would codelimit work on this repo? Not for the Rust code.** v0.21.0 has no Rust language support
(§4), so a `codelimit scan .` would analyze only the seven `tools/*.py` helpers and skip every `.rs`
file silently; `codelimit check src/main.rs` would report `0 files checked` and exit `0`. That is a
**false green** for exactly the code we'd want to guard, so it should not be adopted as a Rust
function-length gate today. The pre-commit hook would not help either (`types: [file, python]`,
[`.pre-commit-hooks.yaml#L7`](https://github.com/getcodelimit/codelimit/blob/main/.pre-commit-hooks.yaml#L7)),
and the GitHub Action would pass while checking only changed Python files.

**Config we would start with** — two scenarios:

*If adopting today, for the Python tooling only* (minimal, since `/target/`, `/artifacts/`, `/dist/`
are already in `.gitignore` and are therefore already excluded):

```yaml
# .codelimit.yml — car-game
verbose: false
exclude:
  - target/
  - artifacts/
  - dist/
```

Then run `codelimit scan .` once and `codelimit report` to see the baseline, and gate only changed
Python files (`codelimit check $(git diff --name-only origin/main...HEAD)`), or add the official
Action with a pinned version (`codelimit_version: v0.21.0`) rather than `latest`. If we want to
ignore the Python helpers entirely, use `--exclude "tools/*"` (which would then leave nothing to
check).

*If/when Rust support lands (#105):* keep the same `.codelimit.yml` (thresholds are not
configurable — 30 warn / 60 fail), start in report-only mode for one iteration to see the size
distribution of Bevy systems, then enable `check` on changed files only. Note `tools/` is not
excluded by default (only `test`/`tests` are), so it must be excluded explicitly if we only want Rust
covered.

**Practical alternative today:** car-game already runs `cargo clippy --all-targets -- -D warnings` in
CI; Clippy's `too_many_lines` lint (pedantic, allow-by-default, default threshold 100 lines,
configurable via `too-many-lines-threshold` in `clippy.toml`) gives Rust-native function-length
enforcement without a new dependency
([Clippy lint docs](https://rust-lang.github.io/rust-clippy/master/index.html#too_many_lines)).
Codelimit's 30-line "hard-to-maintain" line is much stricter than Clippy's default 100.

### Open questions

1. **Rust support timeline/design** — issue #105 is open with no description; #68 (tree-sitter) is
   open. Unknown whether Rust will be a hand-written token grammar or tree-sitter-based.
2. **Threshold fit for Bevy systems** — 30/60 are hard-coded; Bevy systems (and `main`/setup
   functions) may routinely exceed 30, making warnings noisy. Unknown until a scan can run on `.rs`.
3. **Scope of adoption** — gate `tools/*.py` now, wait for Rust, or use Clippy `too_many_lines`
   instead? Decision needs a baseline report (blocked on Rust support).
4. **Action robustness** — changed-file detection uses the compare API and falls back to checking the
   *entire* repo (`['.']`) on `workflow_run`/API failure; also defaults to `codelimit_version: latest`
   (unpinned). Needs a deliberate decision on pinning and on failure-mode noise.
5. **No baseline file** — ratchet exists only via the Action's changed-file filtering; there is no
   CLI baseline and no configurable thresholds, so adopting a "fail only above 60" or
   grandfathering policy would require a wrapper script.
6. **Docs drift** — docs describe a global `--exclude` and an `app` command that the v0.21.0 CLI does
   not expose; pin to a version and verify against `codelimit --help`.
7. **Maintenance/licensing risk** — single maintainer, small project (42 stars), last release
   2025-10-21; GPL-3.0-or-later is fine for CI use but matters if we ever vendor or fork it.

---

## 5-line summary

1. CodeLimit v0.21.0 is a free, GPL-3.0, zero-config Python CLI (Typer + Pygments) that measures **only function length in lines of code** (comments/blank lines excluded), warning at 31–60 LOC and failing `check` above 60.
2. It supports exactly **C, C++, C#, Java, JavaScript, Python, TypeScript** — **no Rust**; `.rs` files are skipped silently, and Rust support is only an open request (issue #105, opened 2026-10-13).
3. Config is a single root `.codelimit.yml` with just `exclude` and `verbose`; thresholds are hard-coded and there is no JSON Schema, no `--since`, and no auto-fix.
4. Install via Homebrew / pipx / pip / release binaries / `getcodelimit/codelimit:latest` Docker image; CI via the official `getcodelimit/codelimit-action@v1` (Docker action, ratchet = check changed files only) or a pre-commit hook that is Python-files-only.
5. For car-game: it cannot guard the Rust code today (would report a false green), so either scope it to `tools/*.py` or use Clippy's `too_many_lines` (pedantic, default 100) for Rust function-length enforcement.

---

## 10. Setup applied (2026-10-22)

**Decision:** adopt codelimit for the Python tooling **and** Clippy `too_many_lines` for the Rust
sources. codelimit alone would give a false green on all 3,358 Rust lines; Clippy alone would leave
the 1,370 Python lines unguarded. Together they cover both languages.

**Files added/changed:**

| File | Purpose |
| --- | --- |
| `.codelimit.yml` | codelimit config; excludes `target/`, `artifacts/`, `dist/`, `__pycache__/`, `.scratch/` |
| `clippy.toml` | `too-many-lines-threshold = 100` (Clippy default; ratchet down after refactoring) |
| `scripts/check_codelimit.sh` | ratchet gate: checks only files changed vs a base ref |
| `.github/workflows/ci.yml` | new CI step "Function-length gates" running the script on PRs/pushes |
| `scripts/verify.sh` | local verify chain now includes the ratchet gate |
| `.gitignore` | ignores `/.codelimit_cache/` |

**How the gate works** (`scripts/check_codelimit.sh [base-ref]`, default `origin/main`):

1. Collects changed files vs base (staged + unstaged + untracked).
2. **Python:** `codelimit check <changed .py files>` — fails (exit 1) if any function > 60 LOC.
3. **Rust:** `cargo clippy --all-targets -- -W clippy::too_many_lines` — fails if any
   `too_many_lines` warning points at a changed `.rs` file.

**Baseline (2026-10-22, v0.21.0):** `codelimit scan .` → 7 Python files, 51 functions, 1,002 LOC;
4 functions > 60 LOC (✖), 5 at 31–60 (⚠). Clippy `too_many_lines` at 100: 4 existing violations —
`src/city.rs:64` (203), `src/presentation.rs:118` (175), `src/smoke.rs:201` (216),
`src/vehicle.rs:137` (245). The ratchet means these pre-existing violations do not block CI; only
new/changed code is gated.

**Install for local use:** `brew install codelimit` (or `pipx install codelimit`).

**When codelimit adds Rust support (issue #105):** re-evaluate. If adopted, decide whether to keep
the Clippy gate (redundant but stricter at 100 vs 60) or drop it, and whether to scan `src/` with
codelimit in report-only mode to see the Bevy-system size distribution before enforcing.

**Known limitations:**

- codelimit thresholds (30 warn / 60 fail) are hard-coded; cannot tune per-language.
- codelimit silently skips `.rs` files — a repo-wide `codelimit scan .` looks healthy while ignoring
  all Rust. The `.codelimit.yml` header documents this.
- Clippy gate is file-granular, not function-granular: touching `src/vehicle.rs` for an unrelated fix
  will fail CI until the 245-line `simulate` is refactored. This is intentional ratchet pressure.
- CI step uses `brew install codelimit` (unpinned). Pin to a release when supply-chain policy requires.
