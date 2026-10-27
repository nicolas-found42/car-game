# Preservation checkpoint recovery

The immutable `codex/preserve-fleet-progress-20261026` checkpoint (`d75f950`)
contains unique work from before the independent-control and synchronized-physics
integration. It is a recovery source, not a merge candidate. `inventory.json`
assigns every one of its 1,068 changed paths to a focused recovery slice and pins
the exact checkpoint bytes. Shared glue files need hunk-level separation.

The first slice recovers the vehicle-reference corpus only. Its original research
qualifications and manufacturer/game distinctions remain authoritative. Structural
verification rechecked all 50 references, 80 counterpart assets, 280 source file
hashes, 99,166 exported native rows/values and 1,555 cited facts. It does not
independently certify the truth of every historical external claim.

Remaining slices separate asset preparation/catalog data, diagnostic recording
and provenance, scenario courses/fast runner, vehicle profiles/force integration,
and study tooling. Large historical result matrices remain recoverable at the
checkpoint; their old passing runs do not certify current-main integration.
Do not restore whole old runtime files over the newer per-vehicle controls,
assembly retention, synchronized query/force schedule or completed-step records.

Recovery command for any original file:

```sh
git show codex/preserve-fleet-progress-20261026:path/from/inventory
```


## Recovery disposition

| Slice | Result |
| --- | --- |
| Reference corpus (82 checkpoint paths) | Recovered in [#108](https://github.com/nicolas-found42/car-game/pull/108), with source/export verification. |
| Fixture retention | Isolated cleanup merged in [#109](https://github.com/nicolas-found42/car-game/pull/109). |
| Fleet assets and preparation (582 paths) | [#110](https://github.com/nicolas-found42/car-game/pull/110) preserves the 287 additional editions outside the active asset root; 13 remain active. |
| Diagnostics and provenance (25 paths) | CLI recovered in [#112](https://github.com/nicolas-found42/car-game/pull/112); profile material binding in [#113](https://github.com/nicolas-found42/car-game/pull/113); automatic windows recovered separately with per-actor ownership. Existing current-main recorder guarantees remain authoritative. |
| Profiles and forces (20 paths) | Adapted to current assemblies/controls in [#113](https://github.com/nicolas-found42/car-game/pull/113), behind `--reference-physics`. Default arcade tuning retained. |
| Verification/study tooling (37 paths) | Current verification, diagnosis, profile and asset checks integrated. Old study runners remain quarantined because they assume the old actor pipeline, roster and courses. |
| Scenario courses/fast runner (8 paths) | Kept at checkpoint. They change the driving scene and clock behavior and require their own acceptance; not part of restoring completed work. |
| Historical study evidence (314 paths) | Kept byte-for-byte at checkpoint. Old matrices are historical evidence, not current-main acceptance or original-game fidelity certification. |

The duplicate GT experiment is retired after [#107](https://github.com/nicolas-found42/car-game/pull/107) fulfilled #16 with retained identities. Its unique commit and ignored evidence were saved before removal. Claude's #86 verifier recovery is [#111](https://github.com/nicolas-found42/car-game/pull/111). No historical runtime file was copied wholesale over the accepted controls/physics implementation.

The preservation branch is deliberately retained as an immutable recovery source,
not an active merge branch. No stashes or unrelated dirty work were discarded.
Deferred recovery prompts, if those features are wanted next:

```text
Create codex/recover-scenario-courses from origin/main. Recover only the eight
scenario-courses-and-fast-runner paths listed in preservation-recovery/inventory.json
from d75f950. Integrate them with current per-actor commands and completed steps;
keep the default driving scene unchanged. Prove opt-in courses and clock behavior
through production scenarios, run the recorded verification ladder, Sol review,
and open one focused PR. Do not overwrite current runtime owners.
```

```text
After course recovery, create codex/recover-calibration-studies from origin/main.
Recover the quarantined study runners and metrics from d75f950. Explicitly select
reference physics; never silently enable staged cars or compare historical and
current runs as equivalent. Adapt actor selection and provenance, reproduce a
small matched control plus a deliberate mismatch, and open a separate tooling PR.
Keep historical bulk matrices pinned at the preservation checkpoint.
```
