# Issue 96: CI recorder deadline repair

Work in progress; no acceptance claim or PR yet.

Base: `bdd2bee`. Branch: `codex/issue-96-ci-recorder`.

The current main and PR #94/#95 jobs all fail the same four production-process
`recorder_shutdown` tests. Original 8-second readiness and 10-second shutdown
bounds remain unchanged. Preserved original logs and SHA-256 receipts live in
`artifacts/issue-96/controls` and `artifacts/issue-96/CHECKPOINT.json` in the primary
checkout.

## Controlled experiment

[Dispatch 39887029794](https://github.com/nicolas-found42/car-game/actions/runs/39887029794)
at `91e67c8` runs on `macos-14`. Temporary workflow instrumentation builds the
production subprocess with the existing CI dependency profile and explicitly
sets `profile.ci.package.sha2.opt-level=0`, then exercises both serial and default
concurrent shutdown controls. It captures failed-child health/logs and partial
object sizes before running the full normal CI suite with only SHA-256 changed
to opt-level 3. No simulation, production deadline, assertion, test concurrency
or provenance policy changes are proposed. Causal interpretation awaits results.

Ranked hypotheses: (1) concurrent cold provenance capture contention,
(2) unoptimized SHA-256 work even for a single process, (3) simulation readiness.
Production `build_app` captures provenance before app construction. Capture starts
by hashing and copying the executable, followed by prepared assets; this is the
candidate boundary to distinguish from simulation readiness.

## Continuation

Do not ship the temporary workflow instrumentation. Restore `.github/workflows/ci.yml`
to the base version, preserve probe artifacts, and obtain a green unmodified CI
workflow on the final branch. Complete the documented verification ladder under
the primary checkout's whole-command Cargo lease. No cold local CI rebuild is
required; remote measurements use the actual CI profile/workload. See
`artifacts/issue-96/CHECKPOINT.json` for exact remaining steps. Issue #96 and its
native dependency on #19 remain open until human merge.
