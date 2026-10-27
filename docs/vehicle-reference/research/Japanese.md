# Japanese vehicle research

The structured companion is [japanese.json](japanese.json): 17 distinct real configurations, each linked to exact local catalog codes. Every fact cites a source in that car’s `sources` list. Named-game identities were checked against `/Users/nicolas/Documents/github/carmodels/dealership/dealership/catalog.json`; archive files were not edited.

Manufacturer specification tables, manufacturer engineering papers, FIA homologation forms and contemporary manufacturer brochures are the foundation. Historical brochure scans live on independent archives; the source metadata identifies those mirrors and includes hashes for successfully retrieved PDFs. Toyota GAZOO and Subaru SUGDAS are explicitly classified as manufacturer-hosted catalogs. Instrumented Car and Driver observations are primary measurements, kept separate from manufacturer specifications and from differently equipped trims.

| ID | Real reference | Exact local primary asset | Identity boundary |
|---|---|---|---|
| j01 | 2003 Nissan 350Z coupe, US manual reference | GT4_NISS0004 | Equipment grade unresolved. MC3/MCLA 2004 versions are related. |
| j02 | 1999 Nissan Skyline GT-R V-spec | GT4_NISS0046; MCLA_VP_NSN_SKYLINE_99 | Exact name/year/trim cross-game pair. |
| j03 | 1997 Toyota Supra RZ, August Japanese revision | GT4_TOYT0006 | Calendar year alone does not settle the VVT-i revision. GT1 RZ has unresolved year; MC3 is 1998. |
| j04 | 2002 Mazda RX-7 Spirit R Type A | GT4_MAZD0007 | Two-seat manual Type A; do not mix Type B/C. |
| j05 | 2003 Mazda RX-8 Type S, Japan | GT4_MAZD0008 | High-power six-speed Japan version; not US/European rating. |
| j06 | 2002 Honda NSX Type R | GT4_HOND0031 | LA-NA2, 3.2L six-speed; air-conditioning changes mass. |
| j07 | 1999 Honda S2000, Japan | GT4_HOND0049 | AP1 launch configuration; Redline community addons lack matching exact identities. |
| j08 | 1993 Mazda MX-5 V-Special Type II | GT4_MAZD0046 | Japanese Eunos Roadster NA8C 1.8L; not the earlier 1.6L V-Special. |
| j09 | 2006 Mitsubishi Lancer Evolution IX, US standard five-speed reference | MCLA_VP_MIT_EVO_06 | Asset omits grade. RS and MR remain separate configurations. |
| j10 | 1999 Mitsubishi 3000GT VR-4, US | MCLA_VP_MIT_3000GT_99 | GT4’s Japanese 1998 VR-4 Turbo is a related configuration. |
| j11 | 2002 Subaru Impreza Sedan WRX STi GD Type II | GT4_SUBR0001 | November Japanese facelift, regular STi; not Spec C or US 2004 STi. |
| j12 | 2003 Subaru Impreza World Rally Car | GT4_SUBR0003 | A5652/02-01WR competition variant; no road-STi parameter inheritance. |
| j13 | 1998 Subaru Impreza 22B-STi Version | GT4_SUBR0019 | Road-going 2.2L coupe, not a WRC car. |
| j14 | 1997 Toyota MR2 GT-S | GT4_TOYT0026 | December hardtop reference. GT1 GT-S has unresolved year. |
| j15 | 2002 Toyota MR2 V Edition / Japanese MR-S | GT4_TOYT0071 | August six-speed reference; Redline’s 2005 US Spyder and modified 2ZZ addon differ. |
| j16 | 1995 Suzuki Cappuccino EA21R | GT4_SUZK0007 | K6A engine; do not transplant EA11R/F6A figures. |
| j17 | 1997 Honda Civic Type R EK9 | GT4_HOND0082 | Option-dependent mass; standard and race-base equipment differ. |

## Physics distinctions supported by the evidence

The [1999 Nissan brochure](https://images.jdm-catalogues.com/nissan/skyline/r34/nissan_skyline_r34_gtr/catalogue.pdf), printed specification page35, describes the V-spec’s ATTESA E-TS PRO and active limited-slip differential. Its 1,560kg vehicle mass, 2,665mm wheelbase and 1,480/1,490mm tracks belong to this exact trim. The later V-spec II heritage page is not substituted for the 1999 brochure. This gives the GT4/MCLA pair a useful identity-controlled comparison.

The [Subaru 22B launch specification](https://www.subaru.co.jp/news/archives/98_1_3/3_16_1.html) documents a 35:65 front/rear driving-force split when its driver-controlled center differential is free, with locking available. It also documents Bilstein/Eibach suspension and 235/40ZR17 Pirelli tires. The [November 2002 STi release](https://www.subaru.co.jp/news/archives/02_10_12/02_11_01.htm/) instead makes automatic DCCD optional on regular STi and standard on the 17-inch Spec C. “AWD” alone is insufficient to choose a split or differential implementation.

The [FIA WRC2003 extension](https://historicdb.fia.com/sites/default/files/car_attachment/1601041801/homologation_form_number_5652_group_a.pdf), PDF54–70, confirms the rally variant’s 1,994.3cc engine and four driven wheels. Its recorded engine internals and injection system are useful engineering evidence, but do not provide an event-specific tire compound, gearset, minimum running mass or torque curve. Fields marked `X X X` in the form remain unknown. The base GDA form and later WRC extensions are not silently inherited.

The [S2000 launch factbook](https://www.honda.co.jp/factbook/auto/s2000/199904/050.html) lists both a 1.160 primary reduction and 4.100 secondary reduction. Omitting the first stage would materially change wheel torque and speed. Its [layout explanation](https://www.honda.co.jp/factbook/auto/s2000/199904/004.html) explicitly puts the engine behind the front axle and claims nominal 50:50 balance. The factbook’s Japan rating is 250PS, not the 240hp US specification.

The [Mazda RX-7 Type A brochure](https://kurumacatalog.com/wp-content/uploads/2023/08/2002_Mazda_RX-7_catalog_jp.pdf), PDF21, supplies exact track, gearing, staggered tires, damper construction and stabilizer dimensions. The [launch release](https://newsroom.mazda.com/en/publicity/release/2002/200203/0325e.html) independently confirms the Type A’s two-seat/manual identity and brake/damper equipment. The two-rotor displacement remains the manufacturer’s `654×2cc` notation; it is not treated as an ordinary reciprocating-engine swept-capacity equivalent.

The [Mazda Technical Review](https://www.mazda.com/content/dam/mazda/corporate/mazda-com/en/pdf/innovation/monozukuri/technology/tech-review/2003/giho_all2003.pdf), printed82/PDF88, supplies the Japanese RX-8 six-speed ratios and 4.444 final drive. Printed52/PDF58 claims 5% lower yaw inertia than RX-7 while retaining nominal 50:50 distribution. That is a comparative engineering claim, not an absolute inertia tensor. Printed39/PDF45 has an inconsistency: its narrative gives 212Nm at7,500rpm, while its table and [launch release](https://newsroom.mazda.com/en/publicity/release/2003/200304/0409e.html) give 216Nm at5,500rpm. The corpus uses the corroborated table value and records the conflict.

The [1993 Roadster brochure](https://www.atelier-nii.com/cars/rs_catalog/jp/1993_08jp_Roadster_brochure.pdf), printed25/PDF14, confirms the selected V-Special Type II is the 1.8L BP version. The option chart distinguishes base manual, automatic, ABS and airbag-package masses. The [1995 Suzuki brochure](https://images.jdm-catalogues.com/suzuki/cappuccino/suzuki_cappuccino_ea21r/catalogue.pdf) likewise separates 690kg manual from 700kg automatic/option cases. Its [51:49 balance statement](https://images.jdm-catalogues.com/suzuki/cappuccino/suzuki_cappuccino_ea21r/images/l/page.4.webp) explicitly assumes two occupants; it must not become a curb-condition center-of-mass target.

The [NSX-R specification](https://www.honda.co.jp/factbook/auto/nsx-r/200205/11.html) separates 1,270kg standard, 1,290kg air-conditioned and 1,300kg air-conditioned/airbag configurations. Its [launch explanation](https://global.honda/en/newsroom/news/2002/4020523-nsx-eng.html) describes negative lift, stronger roll resistance, a preloaded LSD and purpose-developed RE070 tires. This is positive evidence that these details should differ from a generic NSX, but the document does not supply spring rates or lift coefficients.

## Performance observations and limits

[Car and Driver’s instrumented 2003 350Z Track test](https://www.caranddriver.com/reviews/a15136959/nissan-350z-road-test/) reports 5.4s to60mph, 164ft from70mph to rest, 0.88g on a300ft skidpad and156mph drag-limited top speed. Its tested mass is3,322lb. These are observations of a Track six-speed; they do not establish an unspecified GT4 grade’s factory curb mass.

The [instrumented Evolution IX MR test](https://www.caranddriver.com/reviews/a18202877/2006-mitsubishi-lancer-evolution-ix-first-drive-review/) used a six-speed MR at Mitsubishi’s Okazaki track. Its observations are stored under `related_mr_test_*`; they are not acceptance thresholds for the five-speed standard IX reference. [Contemporary manufacturer-format technical material](https://www.theautochannel.com/news/2005/08/11/140100.html) distinguishes those transmissions, masses and dampers explicitly.

For the [1999 US 3000GT brochure](https://importarchive.com/brochure/mitsubishi3000gt1999_01), the publicly retrieved specification image is only400pixels wide. Legible power, torque, drivetrain, tire and chassis facts are transcribed, but ambiguous small-print dimensions, mass and ratios remain unknown. Earlier-year service-manual specifications and 1998 Japanese GTO ratings are not used as substitutes.

No profile treats unknown tire curves, center-of-gravity height, inertia, brake bias, spring rates or damping curves as zero. Factory nominal figures and instrumented observations are separate evidence types. A performance expectation should be conditional on exact trim, mass basis, tires, road surface and test method.

## High-leverage comparisons

1. Does changing from rear drive to the documented AWD/differential configuration improve launch traction without incorrectly increasing cornering friction?
2. Does the GT4/MCLA Skyline pair produce matching results once both use the same real specification, exposing geometry/import differences?
3. Do the two reduction stages of the S2000 reproduce engine-speed versus road-speed traces without artificial engine torque compensation?
4. Do the documented tires, mass and gearing separate a lightweight Cappuccino/Roadster from a Supra or 3000GT while allowing distinct braking and rotation behavior?
5. Can the RX-8 retain the published lower yaw-inertia relationship without assuming that equal front/rear mass balance implies equal inertia?
6. Does a WRC car use an explicitly chosen rally setup rather than inherited road-STi friction, gearing and center-differential defaults?
7. Do trim and option changes explain discrepancies before changing tire grip or engine power globally?
