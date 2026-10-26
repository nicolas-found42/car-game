# Using this corpus to tune and validate physics

The corpus supports building per-vehicle physical profiles and checking plausible behavior. It does not establish a single handling ranking. Tires, road surface, load, grade, gearing, aids and test procedure need controlled definitions before comparisons become acceptance tests.

## Decisions the evidence supports

| Question | Useful evidence | Suggested controlled check |
|---|---|---|
| Does an AWD rally car actually deliver torque through the intended wheels? | Road STi, 22B and WRC profiles distinguish center/rear differentials and road versus homologated race specifications. Native GT4 drivetrain/tire rows remain inspectable. | Inspect individual wheel drive force, longitudinal slip and normal load during identical launches on defined dry and loose surfaces. Compare the configured differential behavior, not just an AWD label. |
| Can a low-power Mini outrun a supercar because all vehicles share the same drive force? | Profiles preserve engine output, mass, gearing and available acceleration observations. | Compare acceleration through specified speed intervals using each car's stock configuration and its own applicable reference. Do not compare 0–60 mph against 0–100 km/h or a modified Mini against a stock Ferrari. |
| Does braking use the load and grip available at each axle? | Manufacturer mass/distribution/tire/brake facts; Redline per-wheel brake settings; GT4 front/rear encoded brake parameters. | Brake from the same measured entry speed on the same surface. Record axle load transfer, saturation, ABS behavior and stopping distance. Real distance targets require compatible tires, load and measurement procedure. |
| Is high roll/tilt caused by geometry or by shared suspension calibration? | Real wheelbase/track and static model pivots are separate. Actual spring/bar data exists for some Ford profiles; many cars lack force curves. | Compare steady lateral acceleration with roll angle and wheel loads; distinguish pitch from roll. Identify estimated spring/damping values explicitly until measured data is available. |
| Is a discrepancy caused by the game model or the chosen real-world trim? | Cross-game asset matches, identity notes, source-year qualifications, hashed current model geometry. | Apply one verified physical reference to genuinely matching counterparts, then examine geometry/assembly differences. Related-year/trim assets stay outside an exact controlled pair. |

## A concrete geometry check

The [1982 European 911 SC reference](profiles/e01.md) has a factory-handbook wheelbase of **2,272 mm**. The selected Redline model/configuration places wheel pivots **2,330 mm** apart. That is a **58 mm / 2.55%** discrepancy. The geometry measurement is reproducible from the model and authored wheel positions; it is not a manufacturer value. The related brake/tilt investigation demonstrates why such differences can matter, but this corpus makes no new claim that the 50 cars have all been simulated or tuned.

## Parameter priority

Start with identity, dimensions, wheel locations, mass basis, engine torque and gearing, drive layout, differential behavior and tire sizes. Keep engine and gearbox alternatives together as complete configurations. Then add weight distribution and brakes. Finally calibrate tire force behavior, CG/inertia, spring/damper curves and aero against trustworthy tests or engineering data.

Many sources do not publish CG height, an inertia tensor, tire force curves or damper force/velocity curves. They are explicit gaps, not permission to silently reuse the same constants for all cars. Estimates can be useful, but belong in a separate layer carrying the estimation method and validation evidence.

Source conflicts remain visible: the [Focus](profiles/a06.md) has differing factory document figures, the [RX-8](profiles/j05.md) has a technical-paper torque discrepancy, the [Cappuccino](profiles/j16.md) quotes weight distribution with two occupants, and the [Testarossa](profiles/e02.md) uses an adjacent-year factory handbook. Those distinctions should propagate into benchmark eligibility.

The [SQL examples](queries.sql) expose coverage, cross-game counterparts, geometry discrepancies and sourced performance observations. They deliberately avoid creating a universal leaderboard from incompatible tests.
