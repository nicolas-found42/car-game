# Explicit reference-physics calibration

`--reference-physics` selects the recovered reference/estimate model for the
active13 editions. The existing shared arcade model remains the default. Both
use the current per-vehicle controls, retained assemblies and synchronized
query/force/completed-state pipeline. The287 additional prepared editions remain
staged separately; their300-entry profile corpus is preserved but does not
expand the active roster.

The opt-in model uses declared mass/COM/inertia, axle-frequency suspension,
load-following brakes, separate longitudinal/lateral friction limits, explicit
driven-axle force shares, estimated torque curves and ratio-aware shift
hysteresis. It emits requested/applied longitudinal force and shift/contact
decision observations. These are approximations with field-level qualification
in `assets/cars/physics.json`, not a recovered original-game engine or stock
performance certification. Prepared geometry and current identity/control
ownership stay unchanged.

Bound edition graphs decompose their selected declared mass/COM/tensor into the
same residual body and four estimated25kg wheels. Default-mode graphs retain
their1320kg baseline. Switching/resetting retains the selected model and the
existing ledger/part identities. Profiles are captured as both compiled and
runtime material; substitution cannot verify as the original recorded tuning.

Headless integration checks drive all13 active editions with their own profiles,
verify mass and control/force identity, compare configured graph aggregates with
profile properties and reproduce the NSX wide-ratio shift reversal boundary.
The ordinary repository regression suite continues to exercise default mode.
Native/recorded acceptance for this recovery is recorded separately below;
historical300-car matrices remain checkpoint evidence, not current acceptance.

```sh
CAR_GAME_MUTE=1 cargo run --release -- --reference-physics
```
