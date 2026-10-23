# Worktree cleanup checkpoint — issue #12

The user stopped P1 implementation to recover local disk space. This branch preserves the worktree before removal. Its PR targets `main`; no PR is merged and the issue remains open.

Original head: `adf9fed09288b15bd148bee843b4eed63e6bcf6a`. Branch: `codex/issue-12-assembly`.

## Work preserved

Stable vehicle/part/definition identities, explicit 350Z bindings, atomic assembly changes, mass/inertia and wheel capability derivation, inspection, and canonical location provenance.

## Verification and limits

Authoritative serialized 66 Rust tests; 17 Python provenance tests; recorded fleet 13 cars/52 captures; assembly assertions and actual native driving/reset/pause/quit. See issue-12.md and issue-12-evidence.json.

## Continuation

CI shutdown failures predate this work and are tracked in #96. Full P1 acceptance remains #19. Native surface-readback failures are preserved as failures.

All additional local worktrees and local feature branches are being removed. Recreate a checkout from this PR's remote branch to continue. Main remains synced with origin/main. Primary raw evidence remains under `/Users/nicolas/Documents/github/car-game/artifacts/`; recorded binaries formerly held only in target folders were moved to `artifacts/worktree-cleanup/preserved-builds/issue-12/`. Build caches are disposable and are removed. No new tests or gameplay verification were run for this documentation-only preservation step.
