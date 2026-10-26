# Primary handling and performance targets

Research date: 2026-10-25. The structured companion is [handling-targets.json](handling-targets.json). Baseline values come from the completed geometry/drivetrain phase's `all-57-cars.csv`; the comparisons below use its **after** columns. Real tests are observations with different tires, surfaces, loads, and launch methods. Unknown rollout is not silently converted to the simulation's no-rollout times.

## Highest-confidence priorities

- **RS4:** current 300 ft result 1.0676 g exceeds the related 2007 US sedan test's 0.91 g by 17.3%; 0–100 mph is 9.417 s versus 11.1 s. The OEM US sedan curb mass is **1795 kg**, rather than the current related European 1650 kg. This is a concrete mass-basis correction before delivery or tire calibration. [Audi technical sheet](https://www.audiworld.com/model/rs4/07/07AudiRS4_TechSpecs.pdf), [C/D original instrumented test](https://www.caranddriver.com/reviews/a18200959/2007-audi-rs-4-quattro-road-test-review/).
- **CTS-V:** current 200 ft result 1.0344 g exceeds Autoweek's exact 2004 road test 0.88 g by 17.5%. Midrange is also too fast: 0–100 mph 10.917 s versus MotorTrend's 12.3 s, despite a much smaller launch difference. These support independent lateral-grip and power-delivery investigations. [Autoweek original test](https://www.autoweek.com/news/a2086961/2004-cadillac-cts-v-flying-v-cadillacs-cts-v-simply-shreds/), [MotorTrend April 2004 original comparison, republished](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive).
- **Early NSX:** current 300 ft result 0.9862 g exceeds C/D's early US car 0.86 g by 14.7%; braking 46.761 m is close to its 157 ft (47.854 m). This is a diagnostic discrepancy only. The article labels the car 1990, matching the catalog, but early/preproduction status remains unresolved; defer tuning until a production configuration is verified. [C/D 1990 comparison](https://www.caranddriver.com/reviews/comparison-test/a15140663/porsche-911-carrera-4-cabriolet-vs-acura-nsx-chevrolet-corvette-zr-1-ferrari-348ts-lotus-esprit-turbo-se-archived-comparison-test/).

## Source qualifications that matter

The US Audi document is explicitly **2007 RS 4 Sedan**, not Avant or Cabriolet. Its six ratios and final drive are recorded in JSON. The 2008 US announcement describes sedan equipment changes and a shared powertrain, supporting a related-year comparison; it does not establish exact 2008 sedan curb mass. Direct PDF retrieval returned 403; the web search index exposed the original tables. Keep that retrieval limitation visible. Jev browser confirmed the C/D table and its weather context: 97°F at 2000 ft, weather-corrected; a separate race-track session used different P Zero Corsa tires. Its engine-unit/torque-rpm labels conflict with the later release, so the table should not automatically replace the existing engine curve. [Audi 2008 announcement](https://www.audiworld.com/model/rs4/08/RS4_Cab_PR.pdf).

Autoweek describes production Goodyear Eagle F1 tires, a 200-foot skidpad, 5.35 s to 60 mph, and 112 ft braking from 60 mph. Its accessible text omits exact tire size and test payload. MotorTrend identifies 245/45WR18 Eagle F1 tires, 3850 lb and 54/46 distribution; it reports launch axle hop. Their braking spread is evidence against fitting one universal exact result. C/D's separate 0.90 g CTS-V result has a documented leaking front tire and no accessible circle diameter, so it is corroboration rather than the primary target. [C/D test caveat](https://www.caranddriver.com/reviews/a15133698/cadillac-cts-v-road-test/).

## Do not tune these from mismatched evidence

**M3 CSL:** ordinary M3 skidpad values do not describe the CSL's special tire package. No matched primary numeric skidpad was retrieved. BMW's historical technical table gives 4.9 s to 100 km/h, while another retrospective quotes 4.8 s; manufacturer claims are a qualified envelope. The current 1.102 g skidpad alone is insufficient reason to reduce CSL grip. [BMW historical technical table](https://www.press.bmwgroup.com/austria/article/attachment/T0082318DE/125287).

**Early NSX:** no matched production 1991 US 3.0-litre manual 300 ft test was retrieved. The later 1994 C/D result uses different factory tire/wheel equipment; the 1997 NSX-T changes engine, gearbox and body. The 1990 article records ambient temperatures above 100°F and cautions that weather correction may be incomplete. Its 0.86 g remains a diagnostic observation, not a calibration target.

**GNX:** the existing C/D 4.7-second result belongs to a prototype. R&T's original report also used a well-used prototype for informal timing and separately quoted Buick's 5.4-second claim. No verified production-car instrumented target was retrieved. Exclude this apparent large discrepancy from production calibration ranking. [R&T original first look, archived](https://www.roadandtrack.com/new-cars/road-tests/a25623/first-look-flashback-1987-buick-gnx/).

Jev browser open/read independently confirmed the original RS4, CTS-V Autoweek, and NSX comparison pages.

The JSON preserves one supporting excerpt per source, each below 25 words, plus units, source identifiers, circle diameters, baseline values, and calculated relative errors. No production files or reference corpus were changed by this research.
