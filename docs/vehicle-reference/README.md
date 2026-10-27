# Real and game vehicle reference corpus — 50 cars

A physics reference corpus built from the existing local `carmodels` archive and source-linked research. It contains **50 distinct year/trim references**, **80 game model counterparts across six games**, and **1,555 real-world facts**. **21** references have counterparts in multiple games; some are exact named year/trim matches and others deliberately remain related variants.

The selection spans kei cars, classic and modern Minis, hot hatches, Japanese sports cars, road AWD cars, a WRC rally car, rear-engine sports cars, muscle cars, pickups, an SUV, luxury performance cars and supercars. Two Corvette generations count as distinct references. Fictional cars and motorcycles are excluded.

## Open the data

| Deliverable | Contents |
|---|---|
| [SQLite database](corpus.sqlite) | Queryable cars, facts, sources, counterparts, raw game parameters, model measurements and unknowns. |
| [Real profiles JSON](real-profiles.json) | All 50 identities, facts, per-fact citations, units, applicability notes and missing fields. |
| [Game profiles JSON](game-profiles.json) | Complete selected catalog records, native game data, static geometry, hashes and limits for 80 assets. |
| [Car index CSV](car.csv) · [real facts CSV](real_fact.csv) · [sources CSV](source.csv) | Flat exports for filtering or analysis; JSON cells retain ranges and vectors. |
| [Counterparts CSV](counterpart.csv) · [game parameters CSV](game_fact.csv) · [model measurements CSV](model_measurement.csv) | Game evidence remains separate from manufacturer data. |
| [Data dictionary](DATA-DICTIONARY.md) | Units, mass conventions, game decoding boundaries and relationship semantics. |
| [Calibration guide](CALIBRATION.md) · [SQL examples](queries.sql) | How to turn the corpus into controlled physics checks without mixing incompatible references. |
| [Validation](validation.json) · [byte/export checks](evidence/corpus-checks.json) | Counts, relationships, model/native file hashes and export consistency checks. |

## What was collected

Real profiles include dimensions, wheelbase/track, mass and its basis, engine configuration and output/RPM, drivetrain/differentials, transmission and ratios, tires, suspension, brakes, steering, weight distribution, aero and performance observations **where retrieved evidence supports the specific configuration**. Manufacturer brochures, manuals, technical papers, press material, FIA homologation and original instrumented tests are linked at the individual fact. Rehosted factory documents are identified as such. Some profiles have fewer published fields than others.

Peak output is not a torque curve. Dry weight is not curb weight. 0–60 mph is not 0–100 km/h. AWD is not one universal differential behavior. Adjacent model years, alternate transmissions and tested trims stay labeled. The exact scope and evidence gaps are part of each profile, rather than silently filled with another car's numbers.

### Game coverage

| Source game | Selected model assets | Native data coverage |
|---|---:|---|
| Gran Turismo 4 | 34 | Model-code/default-part joins; chassis, engine curve/display values, gearing, brakes, suspension, steering, aids, tire references and retained raw rows. |
| Gran Turismo | 6 | Exact native SPEC-code base specifications and provenance. |
| Midnight Club 3: DUB Edition Remix | 13 | Default-named customization records, including duplicate archive entries; runtime player physics semantics unresolved. |
| Midnight Club: Los Angeles | 14 | Seventeen typed tuning sections per asset, including base/mods simulation and grip sections; many physical field meanings unresolved. |
| Redline | 11 | Active authored `.car` scalar/wheel/gear settings with line references; add-ons distinguished from manufacturer specifications. |
| Ford Racing 2 | 2 | Catalog values/ratings and model geometry; native runtime physics formulas not decoded. |

The SQL export contains **99,166 native value/row records** and **465 model measurements**. Many native records are encoded parameters, raw row bytes or typed unknown offsets; that count is not a claim of 99,166 known physical constants. All 80 model files exist and are hash-pinned. The native build/index identities are in [game-builds.json](evidence/game-builds.json).

### Exact matches versus related models

The 1999 Skyline GT-R V-spec appears with the same explicit year/trim name in GT4 and MCLA, and the 1987 GNX has a named cross-game pair. Other multi-game groups include the 350Z, Supra, MR2, S2000, R32, Elise, SL55, Viper, 300C and CTS-V, with year/market/trim/ambient-role distinctions preserved. A family match supports investigating art and game interpretation; it does not authorize inheriting every physical specification.

A concrete finding is the Redline 911's **2,330 mm** wheel-pivot spacing versus **2,272 mm** in the cited 1982 European handbook. This is an art/configuration discrepancy to investigate, separate from an assertion about the original game's handling. See the [911 profile](profiles/e01.md) and [calibration guide](CALIBRATION.md).

## The 50 profiles

| ID | Real reference | Year / market | Source games | Real facts |
|---|---|---|---|---:|
| a01 | [Ford Mustang GT — 4.6L GT coupe, manual reference; automatic test retained separately](profiles/a01.md) | 2005 / United States | Gran Turismo 4 | 28 |
| a02 | [Ford Mustang SVT Cobra R — 5.4L Cobra R coupe](profiles/a02.md) | 2000 / United States | Gran Turismo 4 | 26 |
| a03 | [Ford Shelby GT500 — Coupe, stock supercharged 5.4L](profiles/a03.md) | 2008 / United States | Redline | 32 |
| a04 | [Ford SVT F-150 Lightning — Regular Cab Flareside RWD](profiles/a04.md) | 2003 / United States | Ford Racing 2, Gran Turismo 4 | 34 |
| a05 | [Ford F-150 FX4 — 5.4L FX4 4x4; cab/bed unresolved](profiles/a05.md) | 2004 / United States | Ford Racing 2 | 22 |
| a06 | [Ford SVT Focus — US 2.0L SVT hatchback](profiles/a06.md) | 2004 / United States | Midnight Club: Los Angeles | 38 |
| a07 | [Ford GT — 5.4L supercharged coupe](profiles/a07.md) | 2005 / United States | Gran Turismo 4 | 29 |
| a08 | [Chevrolet Corvette Z06 — C5 fixed-roof LS6 6-speed](profiles/a08.md) | 2003 / United States | Midnight Club 3 Remix | 31 |
| a09 | [Chevrolet Corvette Z06 — 2007 C6 LS7 stock coupe](profiles/a09.md) | 2007 / United States | Midnight Club: Los Angeles, Redline | 27 |
| a10 | [Dodge Viper SRT10 — 8.3L roadster, 6-speed manual](profiles/a10.md) | 2003 / United States | Gran Turismo 4, Midnight Club 3 Remix | 27 |
| a11 | [Dodge Ram 1500 Laramie Hemi Quad Cab — 5.7L Laramie Quad Cab; bed/drivetrain unresolved](profiles/a11.md) | 2004 / United States | Gran Turismo 4 | 17 |
| a12 | [Chevrolet Silverado SS — AWD SS Extended Cab](profiles/a12.md) | 2004 / United States | Midnight Club 3 Remix, Midnight Club: Los Angeles | 22 |
| a13 | [Hummer H2 — 6.0L 2003 SUV, full-time 4WD; launch specification estimates](profiles/a13.md) | 2003 / United States | Midnight Club 3 Remix, Redline | 27 |
| a14 | [Cadillac CTS-V — LS6 six-speed sedan](profiles/a14.md) | 2004 / United States | Midnight Club 3 Remix, Midnight Club: Los Angeles | 28 |
| a15 | [Chrysler 300C — 5.7L HEMI RWD sedan, AWD alternate separately recorded](profiles/a15.md) | 2005 / United States | Gran Turismo 4, Midnight Club 3 Remix, Midnight Club: Los Angeles | 27 |
| a16 | [Buick GNX — productionGNX identity; prototype measurement envelope](profiles/a16.md) | 1987 / United States | Gran Turismo 4, Midnight Club: Los Angeles | 24 |
| e01 | [Porsche 911 SC 3.0 — G-series coupe; European 204 PS reference, market attribution provisional](profiles/e01.md) | 1982 / Europe | Redline | 33 |
| e02 | [Ferrari Testarossa — Testarossa F 110 coupe; original model, not 512 TR](profiles/e02.md) | 1991 / Unresolved | Redline | 37 |
| e03 | [Lamborghini Gallardo — first-series 5.0 V 10 coupe](profiles/e03.md) | 2004 / Europe | Midnight Club 3 Remix | 37 |
| e04 | [Lamborghini Murcielago — 6.2 V 12 coupe; first series](profiles/e04.md) | 2004 / Europe | Midnight Club 3 Remix | 47 |
| e05 | [Lamborghini Miura P400 SV — SV production coupe](profiles/e05.md) | 1971 / Europe | Midnight Club: Los Angeles | 12 |
| e06 | [BMW M3 CSL — E 46 coupe SMG](profiles/e06.md) | 2003 / Europe | Gran Turismo 4 | 36 |
| e07 | [BMW M5 — E 60 saloon SMGIII](profiles/e07.md) | 2005 / Europe | Gran Turismo 4 | 39 |
| e08 | [MINI Cooper S — R 53 three-door supercharged hatch manual](profiles/e08.md) | 2002 / Europe | Gran Turismo 4 | 24 |
| e09 | [Morris Mini Cooper S 1275 — 1275 S road model; exact model year unresolved](profiles/e09.md) | None / UK reference; assetmarket unresolved | Redline | 32 |
| e10 | [Volkswagen Golf IV R32 — Mk 4 VR 6 4 MOTION; EU 2003 reference versus US 2004 asset](profiles/e10.md) | 2003 / Europe | Gran Turismo 4, Midnight Club 3 Remix, Midnight Club: Los Angeles | 43 |
| e11 | [Audi R8 4.2 FSI quattro — Type 42 coupe;USmanual reference](profiles/e11.md) | 2008 / USA | Midnight Club: Los Angeles | 26 |
| e12 | [Audi RS4 — B 7 saloon 4.2 FSI quattro sixspeedmanual](profiles/e12.md) | 2008 / USA | Midnight Club: Los Angeles | 28 |
| e13 | [Lotus Elise 111R — S 2 Toyota 2 ZZ-GE UK 111 R](profiles/e13.md) | 2004 / Europe | Gran Turismo 4, Midnight Club 3 Remix | 36 |
| e14 | [Mercedes-Benz SL55 AMG — R 230 retractablehardtop;GT 42002 and MC 32004](profiles/e14.md) | 2002 / Europe | Gran Turismo 4, Midnight Club 3 Remix | 34 |
| e15 | [Aston Martin DB9 Coupe — launchshowcar/initial 450 bhpmodel;production 2004 on](profiles/e15.md) | 2003 / Europe | Gran Turismo 4 | 30 |
| e16 | [Jaguar XJ220 — production twin-turbo V6 road coupe](profiles/e16.md) | 1992 / Europe | Gran Turismo 4, Redline | 21 |
| e17 | [Pagani Zonda C12S 7.3 — 7.3 LM 120 AMGcoupe](profiles/e17.md) | 2002 / Europe | Gran Turismo 4 | 26 |
| j01 | [Nissan 350Z — Z33 coupe, US manual reference](profiles/j01.md) | 2003 / United States | Gran Turismo 4, Midnight Club 3 Remix, Midnight Club: Los Angeles | 38 |
| j02 | [Nissan Skyline GT-R V-spec — GF-BNR34 V-spec](profiles/j02.md) | 1999 / Japan | Gran Turismo 4, Midnight Club: Los Angeles | 34 |
| j03 | [Toyota Supra RZ — E-JZA80 RZ, August1997 VVT-i revision](profiles/j03.md) | 1997 / Japan | Gran Turismo, Gran Turismo 4, Midnight Club 3 Remix, Redline | 33 |
| j04 | [Mazda RX-7 Spirit R Type A — GF-FD3S Spirit R Type A](profiles/j04.md) | 2002 / Japan | Gran Turismo 4 | 39 |
| j05 | [Mazda RX-8 Type S — Japan Type S6MT](profiles/j05.md) | 2003 / Japan | Gran Turismo 4 | 32 |
| j06 | [Honda NSX Type R — LA-NA2 NSX-R3.2L6MT](profiles/j06.md) | 2002 / Japan | Gran Turismo 4 | 40 |
| j07 | [Honda S2000 — GH-AP1 Japan 6MT](profiles/j07.md) | 1999 / Japan | Gran Turismo 4, Redline | 36 |
| j08 | [Mazda MX-5 V-Special Type II — Japan EunosRoadster E-NA8C1.85MT](profiles/j08.md) | 1993 / Japan | Gran Turismo 4, Redline | 38 |
| j09 | [Mitsubishi Lancer Evolution IX — US standardEvolutionIX5MT reference](profiles/j09.md) | 2006 / United States | Midnight Club: Los Angeles | 46 |
| j10 | [Mitsubishi 3000GT VR-4 — US3000GTVR4 sixspeed](profiles/j10.md) | 1999 / United States | Gran Turismo 4, Midnight Club: Los Angeles | 14 |
| j11 | [Subaru Impreza Sedan WRX STi — GH-GDB GDType II,November2002 regularSTi](profiles/j11.md) | 2002 / Japan | Gran Turismo, Gran Turismo 4 | 36 |
| j12 | [Subaru Impreza Rally Car — Subaru ImprezaWRC2003,FIA A5652 extension02/01WR](profiles/j12.md) | 2003 / Japan | Gran Turismo 4 | 16 |
| j13 | [Subaru Impreza 22B-STi Version — E-GC8modified22B5MT](profiles/j13.md) | 1998 / Japan | Gran Turismo 4 | 38 |
| j14 | [Toyota MR2 GT-S — E-SW20 GT-S December1997 hardtop](profiles/j14.md) | 1997 / Japan | Gran Turismo, Gran Turismo 4 | 33 |
| j15 | [Toyota MR2 V Edition — Japan MR-S TA-ZZW30 VEdition6MT August2002](profiles/j15.md) | 2002 / Japan | Gran Turismo 4, Redline | 33 |
| j16 | [Suzuki Cappuccino — E-EA21R K6A5MT](profiles/j16.md) | 1995 / Japan | Gran Turismo 4 | 36 |
| j17 | [Honda Civic Type R — E-EK9 B16B5MT](profiles/j17.md) | 1997 / Japan | Gran Turismo 4 | 33 |

## Reproduce and verify

From the repository root (Python standard library; existing archive decoder modules required):

```sh
python3 docs/vehicle-reference/scripts/build_corpus.py --selection-only
python3 docs/vehicle-reference/scripts/collect_game_profiles.py \
  --archive /Users/nicolas/Documents/github/carmodels \
  --selection docs/vehicle-reference/selection.json \
  --output docs/vehicle-reference/game-profiles.json
python3 docs/vehicle-reference/scripts/build_corpus.py
python3 docs/vehicle-reference/scripts/verify_corpus.py \
  --archive /Users/nicolas/Documents/github/carmodels
sqlite3 docs/vehicle-reference/corpus.sqlite < docs/vehicle-reference/queries.sql
```

The research inputs under `research/` are reviewed facts, not an automated web-scraping cache. Rebuilding exports preserves those facts; updating them requires checking their sources and applicability. Re-extraction reads the archive without modifying models. Source file hashes, decoder/map identity, model hashes and SQL foreign keys are checked. Hash checks establish byte identity and export fidelity; they do not replace interpreting the source qualifications.

Validation also included the repository format, strict Clippy, complexity and all-target Rust test ladder. No new 50-car driving experiment is claimed; this task compiles the references needed to design one.

### Historical extractor identity

Implementation source hashes identify the code that produced this corpus. If a
current helper has since changed, verification checks the exact retained copy
under `evidence/source/` rather than reinterpreting an old hash as the current
implementation. Those snapshots preserve provenance; they do not establish that
later helper versions reproduce the same outputs. The staged-asset recovery
separately re-prepares all12 original additional editions byte-for-byte.
