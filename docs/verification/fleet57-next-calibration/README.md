# Qualified remaining-reference corrections

Two supported issues were corrected: excessive 350Z midrange delivery and excessive early-NSX lateral grip. All 57 geometries and the other 55 profiles remain exact. The GT500, C5 Z06, RS4 and CTS-V keep their accepted profiles, benchmark rows, skidpads and all 48 original scenario trajectories each. The same 24 scenarios, seed 17, 120 Hz timestep and course definitions were rerun twice for every car.

## What changed and why

The 350Z’s estimated effective delivery of 0.88 made its 60–100 mph interval 7.350 s, compared with 8.500 s from the original C/D Track 6MT test. The existing mass, nominal engine output and six-speed gearing already match that selected related configuration. Isolated delivery probes at 0.82, 0.81 and 0.80 establish the response without altering tire friction or nominal output. An effective delivery estimate of 0.80 gives the table below. This is a reduced-order calibration coefficient, not a measured drivetrain efficiency. Exact GT4 grade, rollout and payload remain uncertain.

| 350Z measure | Before | After | Related Track 6MT test |
|---|---:|---:|---:|
| zero_60_mph_s | 5.200 s | 5.442 s | 5.400 s |
| zero_100_mph_s | 12.550 s | 13.617 s | 13.900 s |
| sixty_to_hundred_mph_s | 7.350 s | 8.175 s | 8.500 s |

Mean absolute percentage error across these three selected calibration observations falls 8.98% → 2.21%. The interval is derived from the same acceleration test and is correlated with the other two measures; this score is descriptive, not independent or held-out validation. Brakes and lateral tire parameters are unchanged. Autoweek’s same-tire Track launch is 5.66 s and its quoted mass differs by 97 lb; that disagreement stays visible. [C/D original test](https://www.caranddriver.com/reviews/a15136959/nissan-350z-road-test/), [Autoweek original test](https://www.autoweek.com/news/a2115721/2003-nissan-350z-its-z-lightful-z-back-long-live-z/).

The early NSX period tests establish Yokohama A-022 equipment rather than modern replacement tires. The C/D retrospective explicitly reproduces the September 1990 .86 g, 300 ft result; the modern RE71R tires belong to the later road car, not the old measurement table. Original R&T reports .93 g on a makeshift circle with bumps/seams and an unknown diameter. Production build/VIN and weather corrections remain unresolved. These sources justify reducing the estimated lateral coefficient from 1.04 to 0.98, with a conservative 0.84–0.95 g regression envelope, rather than fitting a precise .86 g target. Sustained grip changes from 0.985/0.986 g to 0.930/0.932 g on 200/300 ft circles. Acceleration, engine delivery, longitudinal tire grip and braking parameters are unchanged. This is a qualified period-grip estimate, not exact matched-circle production validation. [C/D retrospective](https://www.caranddriver.com/reviews/comparison-test/a28930414/2019-honda-civic-type-r-vs-1991-acura-nsx/), [original R&T transcription](https://rednsx.org/articles/rt90.html).

## Resolved comparisons and deferred uncertainty

- Supra: an original 1997 MotorTrend test gives 5.1 s/12.3 s to 60/100 mph on Michelins. The existing MC3 edition launches in about 5.06 s. The earlier 1993 C/D car’s 4.6 s launch, different RE020 tires and different mass do not justify increasing output.
- Viper GTS: the current 0–100 mph result already agrees closely with the period test. A slower launch alone does not justify more power and an over-fast midrange.
- GNX: the 4.7 s test is a prototype. A firsthand production GNX report corroborates advertised output but quotes uninstrumented timing and rounded mass. No exact production acceleration or mass fit is applied.
- Elise 111R: UK full-tank/no-occupant weight and RE040 equipment are established. US Sport Pack A048 tires and prototype timing are different configurations; the UK 13 s manufacturer claim has no matched measured-load basis. No new delivery fit is applied.
- 911: exact market and published performance load remain unresolved. Known DIN mass and the accepted geometry/braking corrections remain.
- CSL: Cup equipment is established, but a track-session peak is not a matched sustained skidpad. Existing grip and the manufacturer braking upper bound remain.

`remaining-source-notes.md` and `remaining-targets.json` retain source identities, equipment, units, retrieval limits and every decision. Unknowns are not silently converted into exact targets.

## Matched verification and preservation

All 2,736 executions passed, with 7,638 assertions and 912 route completions. All 1,368 repeat pairs have identical trajectories. There are no unexpected resets. The 55 untouched editions retain all 2,640 baseline trajectories exactly. Timed effective controls match at each scenario-ready epoch; route feedback may adapt to response, while controller/course definitions are unchanged.

GT500 launch remains 4.616667 s, its 60–100 mph interval remains 5.200001 s, and its full 150 s trace is exact. C5 launch remains 4.666667 s with corrected geometry and driven-radius averaging preserved. RS4 and CTS-V 0–100 mph remain 11.083334 s and 12.141667 s. Both circles and all scenario trajectories for these four cars are exact.

The full verification ladder and native 57-car check passed, including wheel animations and 228 captured views. Seven overview sheets and the comparison chart were visually inspected; existing source-model material artifacts remain. The red/green regression logs prove both new guards fail before tuning and pass after it. Comparing immutable compiled source snapshots finds only `assets/cars/physics.json` changed for this calibration. Later batch-runner changes have separate frozen source/build evidence.

The canonical 2,736-run matrix contains complete runtime-material and rebuild evidence for every execution. All raw exports are bound to immutable redacted objects and rehashed. Preparation initially put new unselected models in the active asset folder, producing 143 explicitly incomplete evidence attempts. Those attempts remain in the retained attempt index and were rerun with identical physics, binary and scenarios after moving assets to separate staging. A retry coordinator’s relative-path indexing fault was also corrected; completed passing bundles were recovered explicitly, interrupted tasks were rerun, and unavailable elapsed times remain null. No behavioral failure was removed by either correction.

See `validation.json`, `comparison.json`, `all-57-cars.csv`, `input-comparison.json`, `evidence-audit.jsonl`, `export-binding.jsonl` and the frozen executable/evidence under `artifacts/vehicle-reference/fleet57-next-calibration/`. The 300-car expansion and common 5,024-case matrix are documented separately in `../fleet-300/method.md`.
