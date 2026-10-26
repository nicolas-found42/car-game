# Worktree cleanup checkpoint — issue #13

The user stopped P1 implementation to recover local disk space. This branch preserves the worktree before removal. Its PR targets `main`; no PR is merged and the issue remains open.

Original head: `c30cf7446e31160bbe51b8a0174bbd5d1ca31906`. Branch: `codex/issue-13-snapshots`.

## Work preserved

Versioned DTOs, staged atomic restore validation, exclusive root locations, new runtime part mappings, fresh-application restoration and historical transaction reference guards.

## Verification and limits

69 serialized Rust tests and 19 invalid restore cases; final positive/negative/Metal assertions and provenance. The 13-car/52-capture run precedes the narrow history guard; its source boundary is documented in issue-13.md and issue-13-receipt.json.

## Continuation

Includes #12 ancestry. Retargeted from the #12 branch to main for this cleanup; the diff now includes the dependency. P1 acceptance and the CI repair remain pending.

All additional local worktrees and local feature branches are being removed. Recreate a checkout from this PR's remote branch to continue. Main remains synced with origin/main. Primary raw evidence remains under `/Users/nicolas/Documents/github/car-game/artifacts/`; recorded binaries formerly held only in target folders were moved to `artifacts/worktree-cleanup/preserved-builds/issue-13/`. Build caches are disposable and are removed. No new tests or gameplay verification were run for this documentation-only preservation step.
