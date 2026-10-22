# Recorder provenance and portable evidence

The production recorder captures provenance before the application loads its assets.
`manifest.json` has an additive reference to `provenance/capture.json`; the existing
`evidence-manifest.json` is version 2 with provenance version 1. These inventories
contain no independent simulation or save DTOs.

Each included entry identifies a logical path, role, byte count and SHA-256 object.
Objects are copied while hashing, and existing objects are checked before reuse.
Capture identity is created once. After final trace flush and recorder finish, the
index also freezes sanitized run metadata, numeric traces, reports and captures.
Subsequent inspection never substitutes current checkout assets for historical ones.

## Building with source provenance

```sh
# Use this checkout's target; shared external target directories are rejected.
python3 tools/record_build.py
CAR_GAME_MUTE=1 target/debug/car-game.recorded --smoke --offscreen
```

The wrapper snapshots Cargo manifests/lock, Rust sources, optional toolchain/build
files and the embedded fleet catalog. It includes a binary Git diff and all allowed
untracked Rust files, records compiler identity, resolved dependency features and
actual compiler-artifact feature sets, then checks inputs again after compiling.
Its source/configuration fingerprint is embedded by the compiler; both wrapper and
runtime check that binding. It publishes `car-game.recorded` and the matching
`car-game.recorded.provenance` sidecar beside Cargo's normal output. The macOS
bundle script copies this pair. Build outputs must belong to this checkout; separate
worktrees need separate target directories even when sharing dependency caches.

An ordinary Cargo build still captures the exact running executable. Without a
matching compiler-bound sidecar, compiler/features/flags/rebuild identity are
explicitly unavailable. The runtime workspace snapshot is an observation of current
files, never claimed as the source of an older executable. Its comparison with the
binary build snapshot reports `source_build_match` or `source_build_mismatch`.

## Export and verification

```sh
python3 tools/evidence_manifest.py --action export --run RUN_DIR --output /tmp/grid-evidence
cd /
python3 /PATH/TO/car-game/tools/evidence_manifest.py --action inspect --run /tmp/grid-evidence
python3 /PATH/TO/car-game/tools/evidence_manifest.py --action verify --run /tmp/grid-evidence
```

Verification needs only the bundle and Python standard library; no checkout assets,
original working directory or network are consulted. `inspect` returns the index
only after validating its objects. Keep `evidence-manifest.sha256` or the reported
index digest separately when comparing retained copies. Hashes detect accidental
mutation; they do not authenticate an index against an attacker who rewrites all
hashes.

Failures are JSON with a nonzero exit status. Codes distinguish missing objects,
index/version/path errors, executable/config/content/source hash mismatches and
unavailable replay prerequisites. `--require-replay` separately requires replay
completeness. Valid content integrity does not establish deterministic behavior or
availability of a compatible OS, SDK, dynamic libraries, GPU/audio drivers or device.
A failed behavioral run can have a valid provenance inventory.

## Export policy

Policy `grid-portable-allowlist-v1` allows known gameplay argument/value pairs,
known effective configuration and the mute/log/backend environment settings. Input
paths become logical content references; output destinations and unknown arguments
are redacted. Unknown required inputs make runtime material incomplete. Runtime
asset paths are explicitly derived from the embedded catalog plus the three audio
files and four facade textures; extra files are omitted with an explicit reason.

Portable metadata redacts legacy args/environment, private absolute paths, raw log
messages/backtraces and source-root observations. Raw unstructured logs are excluded.
Source material is limited to the declared build inputs; credentials, `.env`, Git
configuration, arbitrary untracked files and build internals are excluded. Original
model-authoring inputs are optional regeneration prerequisites; prepared assets and
receipts are included for runtime replay.

Legacy evidence remains unchanged. Its portable export contains sanitized derived
metadata and explicitly unavailable historical build/assets. The fleet verifier
requires captured catalog and receipt objects for historical asset verification;
it does not open today's roster to repair a historical record.

## Binding and completeness validation

Build binding `grid-build-inputs-v1` is SHA-256 over compact UTF-8 JSON with sorted
object keys, preserving array order. Both wrapper and runtime reconstruct that
input from the source inventory, invocation, flags, dependency graph, compiler hash
and bound declarations. Declarations include compiler/Cargo versions, pre-build Git
observation, omissions and rebuild prerequisites. Rebuild completeness is derived
from validated required sources and empty bound omissions; changing a standalone
boolean cannot upgrade an incomplete build.

The portable verifier validates index/entry types, unique logical and singleton
role bindings, object-name/hash agreement, capture-to-index closure, catalog and
receipt dependencies, effective configuration/input references and build bindings.
Complete claims require their material; explicitly incomplete inventories require
omission reasons. Canonical OS/arch/library and GPU/audio prerequisites remain required and external
in this version; packaged platform proofs are unsupported. Other included required
prerequisites need validated content bindings. These checks do
not validate simulation trace outcomes.

Earlier binding formats cannot establish the stronger rebuild claim. Preserve such
historical indexes unchanged and record their unsupported claim separately; new
captures supersede them rather than rewriting their hashes.
