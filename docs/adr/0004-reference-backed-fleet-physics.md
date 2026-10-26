# Reference-backed fleet physics with explicit estimates

Replace ADR-0003's shared arcade mass/force tuning with explicit profiles for the existing 13 editions. Prepared GLBs, receipts, wheel pivots and roster remain unchanged. `assets/cars/physics.json` binds every runtime parameter to evidence status and applicability; no car silently falls back to another profile.

Use the corpus and the current-fleet reference supplement. Exact manufacturer data, instrumented tests, related variants and native-game proxies remain distinct. The FR2 Cobra identity is unresolved, and the GT1 RX-7 A spec lacks verified real tuner specifications. Those profiles explicitly retain game-derived mass/output proxies. Both Skyline editions share the same qualified reference/estimates because the archive does not establish a physical trim difference.

Rapier chassis mass and COM implement each profile's declared mass/front static weight split. CG height and box-derived inertia are estimates. Suspension wheel rates derive from estimated ride frequency and axle mass; damping derives from an estimated damping ratio. Tire friction/cornering response, anti-roll settings, rolling resistance, drivetrain efficiency and shift timing remain declared estimates. A piecewise torque shape respects peak torque and peak power; speed, tire radius, gear ratios and final drive set engine RPM. Automatic shifts approximate a manually shifted test and are not manufacturer gearbox software. Rear-drive cars deliver no engine force to front tires; the Evo uses a simplified nominal50:50 split and Skylines a rear-biased demand-based split. Neither reproduces the original active differential algorithm or a full rotational tire model.

Keep load-following service brakes and the per-tire combined force limit. Replace the former equal-roll compensation with profile suspension rates so narrow-track or rear-heavy cars need not copy the 350Z's roll response. No generic artificial yaw torque/downforce is used to make unequal cars follow the same trajectory. The low-speed reverse control remains a gameplay convenience.

The 350Z assembly pilot decomposes the selected stock mass/COM/inertia into a residual body and four estimated25kg wheel assemblies. Removing/reinstalling a wheel must still change aggregate properties atomically and restore the stock profile exactly; changing vehicle must replace both collider and mass properties.

Capture the compiled profile JSON independently of the runtime asset, check equality, include it in recorded build inputs and retain profile metadata in run events. Per-wheel requested and applied longitudinal forces allow checking that actual production AWD/RWD force paths match the declared layout. Historical evidence bundles without profiles remain verifiable under their older scope.

Validate with the same130 matched city runs, retaining their limitations when speeds change. Add flat-road standardized entry-speed braking and acceleration measurements through the same VehiclePlugin to compare with applicable real references. Exact test speed, mass basis, related trims, rollout uncertainty and unvalidated turn/tilt targets must remain visible; a successful trace is not proof of real-car fidelity.

## C5 geometry and driven-wheel gearing correction

The expanded fleet's MC3 2003 C5 Z06 edition uses an explicit stock-reference
correction in `tools/reference_geometry.py`: 2.6543 m wheelbase, 1.58496/1.59004 m
tracks, and 0.3219/0.33185 m nominal unloaded tire radii. Source pivots and mesh
bounds agreed, so this was authored geometry rather than a reparenting error.
Wheel meshes scale isotropically around their original hubs and move to the
corrected pivots. The shell receives an affine correction to the published body
envelope; its source wheel-arch styling remains approximate. Collider bounds and
inertia follow the corrected shell, and both render placement and suspension use
the same corrected wheel metadata. No source vertex, triangle or texture bytes
change. The receipt records both authored geometry and correction; the integrity
checker derives the correction from the original archive and fixed reference
constants instead of trusting matrices supplied by the receipt.

Engine coupling uses a nominal driven-wheel kinematic radius:
`1/r = f*(1/rFL + 1/rFR)/2 + (1-f)*(1/rRL + 1/rRR)/2`, where `f` is the
profile's nominal front drive fraction. This averages driven wheel RPM at a given
road speed and applies the same radius to torque-to-force conversion. Undriven
tires cannot alter gearing. AWD remains a nominal center-coupling approximation;
its existing load-sensitive force allocation is separate and does not change
engine RPM as weight transfers. This is not a model of active differentials or
measured loaded rolling circumference. Suspension rest-height calculations still
use all four wheels. Profiles, including the GT500's grip and .82 driveline
efficiency, remain unchanged.

## Qualified US sedan calibration

The next57-car comparison corrects the RS4's related European mass/estimated
gearing using Audi's2007 US sedan technical sheet:1795kg,58% front weight,
its six ratios/final drive andCd×frontal area. These remain related-year
references for the2008US edition, not exact-year measurements. Conflicting
engine-rpm labels do not replace the existing2008 engine reference.

Separate lateral grip from longitudinal traction when calibrating RS4 and CTS-V
against original period skidpads of the matching diameter. Effective drivetrain
efficiency and service-brake deceleration remain model estimates calibrated to
qualified test envelopes, not measurements of drivetrain loss or tire friction.
The CTS-V midrange interval avoids assigning an unknown rollout to engine delivery.
No nominal engine outputs or other editions are tuned to force a ranking. GNX
prototype tests, unresolved earlyNSX configuration and unmatched CSL tires remain
explicit limitations. Source qualifications and observations are retained in
`docs/vehicle-reference/calibration-supplement-20261025.json`.

Keep the GT500 and C5 profiles and every prepared geometry unchanged. Require
identical trajectories for the55 unchanged editions against the accepted
driven-radius baseline, together with repeated production acceleration, braking
and skidpad fixtures. Closed-loop routes use the same courses/controller;
timed-control comparisons align to the recorded scenario-ready boundary.
