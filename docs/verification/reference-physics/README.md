# Explicit reference-physics calibration

`--reference-physics` selects the recovered reference/estimate model for the
active 13 editions. The existing shared arcade model remains the default. Both
use the current per-vehicle controls, retained assemblies and synchronized
query/force/completed-state pipeline. The 287 additional prepared editions remain
staged separately; their 300-entry profile corpus is preserved but does not
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
same residual body and four estimated 25kg wheels. Default-mode graphs retain
their 1320kg baseline. Switching/resetting retains the selected model and the
existing ledger/part identities. Profiles are captured as both compiled and
runtime material; substitution cannot verify as the original recorded tuning.

Headless integration checks drive all 13 active editions with their own profiles,
verify mass and control/force identity, compare configured graph aggregates with
profile properties and reproduce the NSX wide-ratio shift reversal boundary.
The ordinary repository regression suite continues to exercise default mode.
Native/recorded acceptance for this recovery is recorded separately below;
historical300-car matrices remain checkpoint evidence, not current acceptance.

```sh
CAR_GAME_MUTE=1 cargo run --release -- --reference-physics
```

Final recorded binary `08f0e503c135c7c78629cd3277ae315a3a0b1834efde3205b1703d2213525936`
passed the complete default-mode ladder and 13-car/52-view fleet. The same
binary passed bounded reference-mode production scenarios for the 350Z, AWD
Evo and GT500, each with a validated offscreen capture. The 350Z capture was
visually inspected. Two native captures failed while the host was locked and remain retained as
failures. After the user unlocked it, the unchanged recorded binary passed the
same native scenario with a validated capture, which was visually inspected. `results.json` retains the original summaries and capture hashes.

A real brake-to-reverse trace reproduced `gear=-1, decisions.gear_after=1` before
the telemetry fix; all 120 reverse samples now agree. The all-edition integration
test includes this reverse assertion for every active profile. Sol reviewed the
recovery in two rounds and approved the final behavior.
