# GT/GT4 major-part editions — issue #16

The five supported editions are `350z`, `gt-rx7`, `gt-viper`, `gt4-m3` and
`gt4-nsx`. `Assembly::major_parts` extends the original 350Z pilot; its existing
VehicleId (`pilot-350z`), PartIds, DefinitionIds and test definitions are retained.
Other supported graphs use `edition-<fleet-id>` VehicleIds and fleet-prefixed
part/definition IDs. The eight remaining editions retain the legacy, explicitly
unbound physics path pending #17/#18.

## Authored ownership

Each manifest declares one residual chassis and four 25 kg wheel fixtures. All
intact editions retain the shared 1320 kg arcade mass, zero local COM and baseline
inertia. The residual's COM and full tensor subtract the four wheel contributions
at that edition's existing mounts. These are not recovered stock calibrations.

| Edition | Body root | FL | FR | RL | RR |
| --- | --- | --- | --- | --- | --- |
| 350z | 0 `body` | 1 + 3 | 2 + 4 | 5 + 7 | 6 + 8 |
| gt-rx7 / gt-viper | 0 `GRID body` (5) | 1 (6) | 2 (7) | 3 (8) | 4 (9) |
| gt4-m3 / gt4-nsx | 0 `GRID body` (5) | 1 (6,8) | 2 (7,9) | 3 (10,12) | 4 (11,13) |

Parenthesized indices are descendants, not additional bindings. Source hashes
are pinned in `src/assembly/editions.rs` and checked against the actual prepared
GLB before use. The validator rejects wrong assets/scenes/hashes/names, ambiguous
names, overlapping subtrees and missing scene nodes. Shared mesh resources are
distinct node instances, not duplicate ownership. No geometry is split.

Rendering consumes the validated binding roots in the original loaded scene.
`PartVisual` identifies their graph owner; installed state controls root visibility
and therefore its complete subtree. Hierarchy, wheel pivots, reflected scale,
authored rotation, textures and visual offset remain intact. Selection parks each
ledger and its runtime anchors; cycling and reset do not repair parts, clear
transactions, rewind revisions or recreate persistent identities. Snapshot restore
still deliberately rebuilds disposable anchors and increments the revision.

## Explicit content gaps

The prepared assets establish exterior body and wheels/tires only. P2 picking,
access geometry, fasteners and genuine separable internals are unavailable. P4
engine internals (including RX-7 rotary topology), starter/electrical/fuel/cooling
networks, detailed suspension and operating curves are unavailable. Names and
exterior art do not establish these topologies. No internal art, spatial splitting,
or invented stock mechanical calibration is substituted for missing content.

## Verification

`tests/edition_manifests.rs` checks exact hashes/indices/descendants, rejects bad
bindings and checks intact COM/tensor plus each corner's removal. Production
selection/cycle/reset/drive/snapshot retention and physics/wheel identities are
covered by `tests/edition_runtime.rs`. Presentation tests exercise subtree
visibility and preservation of reflected wheel transforms. Use the repository's
recorded verification ladder and native Metal captures; unit tests alone are not
visual acceptance. Local execution results and immutable run/build provenance
are retained under `artifacts/issue-16/`.

## Final native acceptance

The full-send continuation observed all five native smoke runs with four meaningful
1920×1080 views each, plus the native production lifecycle with 40/40 assertions
and its required nonblank capture. `artifacts/issue-16/audit_evidence.py` passes.
The matched native contact sheet was inspected: expected body/wheels/materials,
orientation and front/side/rear/driving camera correspondence are visible.

The first NSX attempt failed a black driving capture; it remains retained. An
unchanged-binary retry passed all four views. No thresholds were relaxed.
The recorded binary SHA-256 is
`d1e2f17ed7a1e7b87ca2dd2421923a5bca4e85c7b3b367175d96b2677e3fcbf7`.
The five accepted run identities and capture hashes are in `native-validation.json`;
`native-contact-sheet.jpg` shows those actual captures. Raw evidence remains under
`artifacts/issue-16/` in the implementation checkout.

Sol-loop round 1 (`gpt-6.1-sol`, medium) approved the complete behavior diff with
no actionable findings. Native acceptance was established separately afterward.

Fresh full recorded verification also passed in this continuation:
`CAR_GAME_MUTE=1 python3 <shared-cargo-lease> bash scripts/verify.sh --recorded --base origin/main`.
Log: `artifacts/issue-16/full-send-verification.log`; canonical log
`artifacts/logs/verify-20261027T001620Z.log`. All 91 Rust tests, doc tests,
format/Clippy/complexity, source asset and Python controls passed, followed by
13 fleet drive scripts, 52 captures and run-evidence validation. The recorded
binary digest remained exactly the digest used by the accepted native runs.
