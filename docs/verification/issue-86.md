# Issue #86 — evidence integrity

Status: preflight complete; implementation awaits verified #15 handoff. This is not an acceptance claim.

## Checkout and dependency

- Owner: issue86 subagent; isolated managed worktree `/Users/nicolas/.codex/worktrees/issue-86-integrity/car-game`.
- Branch: `codex/issue-86-integrity`; preflight base `bdd2bee117fb78c50fa1a18dafbb6a471e614420`.
- Current #9 and #86 fetched with `gh issue view`; native `issues/86/dependencies/blocked_by` confirms #15, itself blocked by #13/#14. Native dependencies remain unchanged. No merge, close, or PR during preflight.
- Read root instructions, verification/domain/issue-tracker docs, glossary and dynamics ADR. No code changes, builds or GPU work performed.

## Existing implementation and preserved evidence

`tools/verify_run.py` is the sole public verifier and must remain so. It invokes `recorder_health.inspect_health` first, then performs shallow lifecycle/report checks. Gameplay checks currently do not read input/physics records, check command/state correlation, hash capture contents, or verify versioned provenance objects. They unconditionally open `report.json`, which is inappropriate for interactive and scenario modes. Exceptions/assertions can escape without a structured record location.

`tools/recorder_health.py` already checks explicit recorder losses, acknowledged finalization, durable/completed tick agreement and export commit. Preserve `--health-only` as a health inspection, not full simulation acceptance. Its fixture deliberately contains empty traces; existing health-only tests must continue to distinguish the two contracts.

Immutable audit source in the primary checkout: `artifacts/issue-9-baseline/empty-trace-probe/probe.json`. A copied completed fleet run with empty input/physics returned exit 0 and `RUN EVIDENCE PASS` under verifier SHA `c80afb1500fdd28463c4913b921a81af03184cf039b4b8dbec7c809b131618fa`. Run ID `20261022T20274547116Z-b5b59a8-fleet-342-1792700865489841000`. Do not rewrite the source artifact. The current health precheck instead returns legacy health unknown for this old run; this does not demonstrate semantic validation of modern completed-state evidence.

Current manifest schema is 1, common envelope schema_version is separately 1, provenance index schema is 2 with provenance_version 1, health schema is 1. These independent versions must not be conflated. Historical unsupported records remain readable with explicit validation limits and cannot become verified complete simulations by guessing missing fields.

Relevant seams: `src/telemetry.rs::Envelope`; `src/runtime.rs::complete_tick`, command ingress/outcome; `src/diagnostics.rs::InputSample/PhysicsSample`; `src/vehicle.rs` pre-step sample; `tools/provenance_bundle.py::verify/load_index/captured_entry` and existing safe-path/schema validation. Reuse provenance verification rather than invent another hash/index validator.

## #15 handoff contract to settle before implementation

Require explicit schema and units for post-Writeback completed-step record, stable actor/body IDs, outer canonical tick, phase/boundary, actor population or declared unavailable actor, applied controls/one-shots including reset, geometry/assembly revisions, bounded wheel evidence and phase durations. Existing `PhysicsSample` is pre-step and currently excludes reset-boundary samples; never silently reinterpret it as completed state. Input lacks actor_id today; #14/#15 need explicit per-actor linkage for two vehicles. Completed-tick event population counts permit missing actor detection. Confirm whether paused/readiness boundaries execute outer ticks and whether command ingress can occur after final physics boundary, then model their actual declared semantics.

## Implementation plan after handoff

Extend the existing verifier with small shared parsing/semantic helpers. Emit versioned JSON result with status `verified`, `failed`, `incomplete`, `unsupported` or `unknown`, structured error code and file/line/tick/actor/command location where available. Catch malformed types/JSON/UTF-8/truncated tails without traceback. Missing exit is unknown/incomplete termination, not proof of crash. Zero-tick startup failure remains a recorded failure, never simulation success.

Validate expected streams by mode, nonempty completed simulation evidence, schema and finite numeric values, units, monotonic/unique contiguous ticks and per-actor coverage, command ingress/outcome identity and ordering, effective input versus completed state including reset counter transitions, capture frame/tick/actor references and content digests, scenario assertions, lifecycle and finalization. Interactive mode does not require smoke reports or GPU captures; fleet retains 13-car/52-capture requirements. Unknown future schemas fail unsupported with stated limits.

Run immutable provenance verification through existing module and report unavailable replay prerequisites explicitly; exact replay is not implied by inventory integrity. Ensure archived content-addressed bundles use existing resolver instead of silently consulting the current checkout.

## Acceptance evidence matrix

Retain a production-app valid completed run and copied mutations: both traces empty, dropped tick/actor, duplicate tick/command, out-of-order tick, missing reset command/effect/state evidence, invalid numeric/schema/units, corrupt JSON tail, capture hash/link mismatch, incomplete finalization and unknown historical schema. Each invalid control must fail nonzero with the specific reason/location; preserve original run read-only. Make semantic negative controls coherent with health/provenance declarations as needed so they actually exercise semantic validation instead of merely tripping an unrelated earlier hash check. Preserve raw invocation, source binary/build/run identity, stdout/stderr, exit codes and control mutation description.

Before completion/PR: run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `scripts/check_codelimit.sh HEAD`, `cargo test --all-targets`, Python compileall and asset checks; applicable production/offscreen evidence coordinated with root to avoid disk/GPU contention. Add focused Python verifier regressions and retain actual command logs. Seek review and fix findings before PR. Jev judgment tools are unavailable in this session; do not fabricate a Jev review/gate result.

## Continuation

1. Receive verified #15 commit/branch, schema examples and completed production run from coordinator.
2. Rebase/merge this isolated branch onto the verified stack; record exact upstream commits here.
3. Implement and verify the matrix above; preserve raw evidence separately from source.
4. Commit, push and create reviewable PR citing #86 after work finishes; attach PR to task. Stop before merge.
