# car-game

## Agent skills

### Issue tracker

Issues and specs are tracked as GitHub issues in this repo, driven by the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The canonical triage roles map 1:1 to `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `GLOSSARY.md` and `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### Verification and gates

Run the verification ladder before completing work or opening PRs: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `scripts/check_codelimit.sh HEAD`, and `cargo test --all-targets`. Full command ladder, complexity thresholds, and grandfathering in `whitelizard.txt`: see `docs/agents/verification.md`.
