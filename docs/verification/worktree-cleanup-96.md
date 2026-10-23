# Worktree cleanup checkpoint — issue #96

Historical checkpoint: `2e7c368` preserved the original issue #96 worktree before
its removal to recover disk space. It retained the temporary diagnostic workflow
and candidate SHA-256 optimization. No verification was performed by that
cleanup commit.

Work resumed from the preserved branch in PR #102. The completed causal probe,
workflow restoration and current verification evidence are documented in
[issue-96.md](issue-96.md). This historical checkpoint does not describe the
current merge readiness; consult that evidence and the final-head CI checks.

Original raw evidence remains in the primary checkout's `artifacts/issue-96/`.
Recorded binaries formerly held only in disposable target folders were retained
under `artifacts/worktree-cleanup/preserved-builds/issue-96/`. Issue #96 remains
open pending human merge.
