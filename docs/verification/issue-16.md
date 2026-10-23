# Issue 16 preflight — waiting for verified issue 15

Status: preflight only. No implementation, build, GPU check, or acceptance claim. Native dependency API confirms #16 blocked by open #15. Worktree: `/Users/nicolas/.codex/worktrees/issue-16-gt-manifests/car-game`; branch `codex/issue-16-gt-manifests`. Continue only after coordinator provides verified #15 integration. #12 immutable handoff: `ed31f39`.

## Authored binding plan

Extend #12's Assembly pilot and Definition.visual/collision, Mount.wheel_corner and compiled wheelmask interfaces. Retain 350z existing PartIds, DefinitionIds, VehicleId and owner; do not recreate the pilot. Other cars require separate stable IDs prefixed by their existing fleet IDs and asset-specific prepared hashes. Bind complete authored body to residual chassis and four explicit wheel assemblies in FL/FR/RL/RR order. This is the existing P1 residual fixture, not genuine internal engine ownership. Never split geometry by spatial bounds.

| Fleet ID | Residual body node | FL nodes | FR nodes | RL nodes | RR nodes |
| --- | --- | --- | --- | --- | --- |
| 350z | 0 body | 1 wheel + 3 tire | 2 wheel + 4 tire | 5 wheel + 7 tire | 6 wheel + 8 tire |
| gt-rx7 | 0 GRID body (child 5) | 1 pivot (child 6) | 2 pivot (child 7) | 3 pivot (child 8) | 4 pivot (child 9) |
| gt-viper | 0 GRID body (child 5) | 1 pivot (child 6) | 2 pivot (child 7) | 3 pivot (child 8) | 4 pivot (child 9) |
| gt4-m3 | 0 GRID body (child 5) | 1 pivot (6,8) | 2 pivot (7,9) | 3 pivot (10,12) | 4 pivot (11,13) |
| gt4-nsx | 0 GRID body (child 5) | 1 pivot (6,8) | 2 pivot (7,9) | 3 pivot (10,12) | 4 pivot (11,13) |

Node names, indices, meshes, complete local matrices, normalization, source correspondences, prepared/source hashes, and roster geometry are preserved in primary `artifacts/issue-16/preflight-bindings.json`. Hashes match all five receipts. Bind container roots recursively or exact leaf nodes without double ownership; 350z uses direct wheel/tire scene roots while other editions use explicit parent pivots. Preserve reflected left-wheel transforms and existing visual_offset.

Retain baseline mass = 1320 kg: four 25 kg wheel approximations, 1220 kg residual with center and full inertia derived by subtracting wheel contributions from declared baseline inertia. Recompute residual per car's existing mounts so every intact assembled car preserves the current shared physics tuning. Do not claim stock calibration.

Missing/unrepresented internal assets/topologies: the prepared documents expose body and wheels/tires only. They do not establish engine internals, piston/rotary topology, starter/electrical/fuel/cooling systems, suspension subparts, fasteners, picking/access geometry or mechanical operating curves. Keep these explicit P2–P4 gaps. GT RX-7 name is not proof of an authored rotary engine manifest.

## Continuation and verification

1. Rebase/integrate immutable verified #15 handoff with #12 pilot API; coordinate common fleet constructor so #17/#18 reuse it.
2. Add exact edition manifests and behavior tests: complete stable binding coverage, invalid hash/node/name rejection, aggregate baseline mass/inertia, each wheel remove/reinstall affecting correct corner while retaining IDs.
3. Exercise production select/drive/cycle/reset for all five IDs; preserve meaningful matched front/side/rear/driving 1920x1080 captures and graph/mass/revision telemetry. Use baseline fixture evidence; no copied or blank acceptance captures.
4. Run cargo fmt, warning-free all-target Clippy, scripts/check_codelimit.sh HEAD, cargo test --all-targets, tooling/asset integrity, full offline fleet and applicable Metal/live ladder. Fix review findings, preserve raw logs/captures under primary artifacts/issue-16, commit durable verification summary, create reviewable PR only after implementation finishes. Stop before merge/close.

No new product decision or missing required P1 asset found. Missing deeper internals are declared deferred content, not an invented implementation.
