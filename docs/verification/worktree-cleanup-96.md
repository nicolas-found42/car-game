# Worktree cleanup checkpoint — issue #96

The user stopped P1 implementation to recover local disk space. This branch preserves the worktree before removal. Its PR targets `main`; no PR is merged and the issue remains open.

Original head: `390bed420b740d8b7b419c57857ef86d312f622a`. Branch: `codex/issue-96-ci-recorder`.

## Work preserved

Candidate CI-only sha2 optimization and a temporary controlled diagnostic workflow comparing baseline serial/default-concurrency shutdown tests with the optimized full suite.

## Verification and limits

Probe run 39887029794 completed successfully: https://github.com/nicolas-found42/car-game/actions/runs/39887029794 . Detailed original failures and checkpoint are in issue-96.md and artifacts/issue-96/CHECKPOINT.json. Individual baseline outcomes still need review.

## Continuation

Inspect/download the causal controls, remove the temporary diagnostic workflow changes by restoring ci.yml from bdd2bee, complete local serialized gates and a clean final CI run, then review. Deadlines were not relaxed. Experimental checkpoint; not ready to merge.

All additional local worktrees and local feature branches are being removed. Recreate a checkout from this PR's remote branch to continue. Main remains synced with origin/main. Primary raw evidence remains under `/Users/nicolas/Documents/github/car-game/artifacts/`; recorded binaries formerly held only in target folders were moved to `artifacts/worktree-cleanup/preserved-builds/issue-96/`. Build caches are disposable and are removed. No new tests or gameplay verification were run for this documentation-only preservation step.
