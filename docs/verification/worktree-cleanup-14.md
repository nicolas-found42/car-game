# Worktree cleanup checkpoint — issue #14

The user stopped P1 implementation to recover local disk space. This branch preserves the worktree before removal. Its PR targets `main`; no PR is merged and the issue remains open.

Original head: `8f7df517f25043004cfda2a6ebe8655e2882ac84`. Branch: `codex/issue-14-controls`.

## Work preserved

Per-chassis controls/configuration, exclusive controller routing, targeted one-shot commands, secondary visuals, reserved player actor alias, original command-intent evidence, shared reset policy and wheel pose helpers.

## Verification and limits

Recorded earlier two-car scenario showed independent opposite motion. Latest implementation commit is 8f7df51. Verification was interrupted for this storage cleanup; overlapping shared-target runs are not authoritative. See issue-14.md.

## Continuation

Reverify latest helper changes, bind a fresh executable, run the full fleet/Metal/native checks and finalize review. Contains #12 ancestry; #13 integration is handled by #15. This is unfinished work, not acceptance.

All additional local worktrees and local feature branches are being removed. Recreate a checkout from this PR's remote branch to continue. Main remains synced with origin/main. Primary raw evidence remains under `/Users/nicolas/Documents/github/car-game/artifacts/`; recorded binaries formerly held only in target folders were moved to `artifacts/worktree-cleanup/preserved-builds/issue-14/`. Build caches are disposable and are removed. No new tests or gameplay verification were run for this documentation-only preservation step.
