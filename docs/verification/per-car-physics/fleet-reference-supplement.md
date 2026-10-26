# Current fleet reference supplement

These references resolve the current fleet against the50-car corpus. Every proxy stays explicitly separate from exact real identity. JSON is the machine-readable evidence; null/missing facts are unresolved, not zeros.

## Identity and calibration decisions

| Fleet | Reference status | Decision |
|---|---|---|
| fr2-cobra | unresolved | Do not join to corpus a02 CobraR. Catalog380bhp1718kg differs from both CobraR385hp1628kg and ordinary1999/2001 Cobra320hp. Exact real counterpart cannot be proved from catalog. |
| fr2-fr500 | catalog_and_manufacturer_name_match | 1999 experimental concept, not2005+ FR500C/GT racecar. Native mass1630kg is a game value; OEM release does not publish mass. |
| gt-rx7 | native_exact_real_variant_unverified | Native spec is direct game evidence only. No authentic A-spec OEM/tuner parameter sheet located. Do not borrow corpusj04 SpiritR2002 or generic stockFD data and label exact. |
| gt-viper | native_name_exact_year_unresolved | GT native year unresolved; use1997 GTS first-hand test as relatedyear reference. Not corpus a10 SRT10 2003. Native1470kg is lighter than tested3375–3410lb. |
| gt4-nsx | catalog_exact_year_and_oem_mass_match | GT41365kg matches OEM5MT, automatic1405kg252hp separate. Not NSX-R2002. |
| mc3-supra | related_variant_proxy | Modelcode suggests1998, catalog does not decode retailyear/trim/market. Choose1998 US Turbo6MT hardtop only as explicit proxy; do not mix JDMRZ1510kg280PS with US320hp mass. |
| mc3-skyline | related_year_and_trim_proxy | Sourcecode02 suggests2002 butcatalog doesnotdecodeyear/trim. Inherit1999JVspec corpusj02 as qualifiedproxy only; do notcall2002Nur orMspec exact. |
| 350z | existing_corpus_reference | GT4 asset has no US equipment grade. Service-manual dimensions apply Z33; mass and grade-specific brakes must not be assumed. MC3/MCLA350Z04 are related different-year counterparts. |
| gt4-m3 | existing_corpus_reference |  |
| redline-911 | existing_corpus_reference | Native Redline year 1982 and 267 Nm identify a late SC. European market is inferred from the author parameters; US engine specifications are excluded. |
| redline-gt500 | existing_corpus_reference | The Redline addon explicitly names a 2008 GT500. Roush Stage 3 and FR500 variants are excluded. Coupe spring and stabilizer settings differ from convertible settings. |
| mcla-evo | existing_corpus_reference | MCLA asset name has no RS/MR grade;standard IX selected as reference. RS1460kg andMR1490kg are separate trims, not interchangeable. |
| mcla-skyline | existing_corpus_reference |  |

## Directly usable facts

FR500: archived Ford1999 release gives415hp at6800rpm,4.10final,T56six-speed,50:50balance and106.3in wheelbase; claims4.6s0–60 andabout175mph onstreettires.1630kg is only gamecatalog mass, torque andactual gearratiosunresolved. [Ford release](https://www.terminator-cobra.com/FR500%20is%20Fords%20Ultimate%20Performance%20Parts%20Car.pdf)

NSX: OEM1991manual gives1365kg,270hp7100rpm,210lb-ft5300rpm,42:58balance,5MT[3.071,1.727,1.230,.967,.771],4.062final andCdA0.57m². Use this instead of2002NSX-R data. [Honda presskit](https://hondanews.com/en-US/releases/release-cc39196e84787a20ec3cb086fb095286-1991-acura-nsx)

Viper: first-hand1997test gives450hp5200rpm,490lb-ft3700rpm,3375lb andRWD. Another1997instrumentedtest records4.0s0–60,8.8s0–100mph,177ft70–0mph and0.95g skidpad. GT1470kg differs fromthetestedcurbmass; sourceyear is unresolved. [MotorTrend](https://www.motortrend.com/reviews/1997-dodge-viper-gts), [Car and Driver](https://www.caranddriver.com/reviews/a35728264/1997-dodge-viper-gts-vs-porsche-911-turbo-s-vs-acura-nsx-t-comparison-test/)

Supra: US1998brochuretranscription givesTurbo320hp/315lb-ft,3445lb hardtop6MT or3505lbSportRoof,53:47balance. CatalogMC3 code alone doesnotproveUSmarket orTurbo. The1993USTurbo4.6s0–60 and160ft70–0test is relatedyear only. [Toyota brochure transcription](https://www.mkiv.com/specifications/98_brochure/index.html), [1993 test](https://www.caranddriver.com/reviews/a15142755/toyota-supra-turbo-instrumented-test/)

MC3 Skyline: use corpusj02 Nissan1999Vspec brochure only as relatedyear/trim proxy. Nativecode02doesnotidentify2002Nur. Nissanhistorical release confirmsR34ATTESA4WD and392Nm4400rpm; do notinterpret as fixed50:50AWD. [Nissan history](https://uk.nissannews.com/en-GB/releases/2017-nissan-gt-r-press-pack?origin=channel-8a0d2f3353d7723c4c3fcf8e15008b5a)

RX-7 A spec:1220kg,265rawPS6500rpm,30kgf-m5000rpm andFR are directnativeGTspecfacts. They are suitable game-derived calibrationproxies, not independently verified realMazdaspeedfacts. Exacttuner sheet andturn/brake groundtruth remainmissing.

COBRA: exactcatalogyear2000/nameSVTCOBRA/380bhp1718kg retained. Noevidencejustifies assigningCobraR385hp1628kg. Use nativegameproxy withunresolvedrealtrim, or resolveartist/sourceidentity beforeclaimingrealvalidation.

## Comparison boundaries

- 0–60mph and0–100km/h differ; periodtest rollout andlaunch procedures are notuniform.
- Existing fleet6sfullthrottle/brake scenario is not a standardized0–100test.
- Compare braking using exactentryspeed andnormalizeda=v²/(2d); notrawdistance whereentryspeedsdiffer.
- Tiremu, CGheight, inertia, wheelrates, damperrates and most weightdistributions are explicit engineeringestimates, notfacts.
- No real matched constantsteering turntest covers current scenario; yaw/roll ranking has no directlyvalidated OEMgroundtruth.

The JSON retains all six existingcorpus profiles and their full originalsources; no50-carcorpus sourcefact was rewritten. Geometry remainsin roster/modelreceipts. No Rust or analysis tools were edited by this research task.

## Bounded performance follow-up

| Fleet reference | Published acceleration | Published braking | Applicability |
|---|---|---|---|
| 1991 Acura NSX, 270 hp, 5MT | 0–60 mph **5.2 s** | 70–0 mph **157 ft / 47.8536 m** | Original September 1990 C/D test calls the launch car 1990; C/D explicitly reproduces those results as 1991 NSX in its 2019 comparison. Stock Yokohama A-022; do not substitute the modern tires photographed in 2019. |
| 2008 Shelby GT500 coupe, 500 hp, 6MT | 0–60 mph **4.3 s** | 60–0 mph **118 ft / 35.9664 m** | Exact July 2008 MotorTrend test, republished in 2026. Skilled launch, measured 3,887 lb, standard Goodyear Eagle F1 Supercar tires. |
| 1982 European 911 SC coupe, 204 PS, manual | 0–100 km/h **6.8 s**, qualified official dealer claim | Not found | Porsche-hosted listing identifies German version and February 1982 registration. This is manufacturer-hosted dealer data, weaker than a period factory publication. |
| 1999 Skyline GT-R V-spec | Unresolved | Unresolved | No original instrumented test or verified factory claim retrieved in bounded search. Do not promote unsourced 4.4/4.6/4.8-second values from aggregators into measured ground truth. |

Primary test sources: [original C/D September 1990 comparison](https://www.caranddriver.com/reviews/comparison-test/a15140663/porsche-911-carrera-4-cabriolet-vs-acura-nsx-chevrolet-corvette-zr-1-ferrari-348ts-lotus-esprit-turbo-se-archived-comparison-test/), [C/D historical result confirmation](https://www.caranddriver.com/reviews/comparison-test/a28930414/2019-honda-civic-type-r-vs-1991-acura-nsx/), [MotorTrend July 2008 comparison](https://www.motortrend.com/reviews/2008-dodge-challenger-srt8-vs-ford-shelby-gt500-comparison-test-archive).

Qualified 911 source: [Porsche Finder, 1982 German-version 911 SC coupe, vehicle 72834](https://finder.porsche.com/ch/it-CH/details/porsche-911-sc-coupe-usata-G028QN?sortBy=price.value%5C). The [1982 French handbook, printed page 72 / online page 76](https://www.manualslib.fr/manual/1040031/Porsche-911-Sc-1982.html?page=76) contains a factory 0–100 row and a DIN empty-plus-half-payload footnote, but the retrieved OCR omits the acceleration number. No numeric handbook claim is inferred from that missing cell.

Use exact speed endpoints for comparisons. 0–60 mph differs from 0–100 km/h, 60–0 differs from 70–0, and rollout, launch technique, tires and road conditions remain material uncertainties. The fleet uses automated inputs rather than the testers' optimized launches.

## Gearing attribution audit

The original cited sources do **not** support the numerical gearing: the [1997 MotorTrend Viper test](https://www.motortrend.com/reviews/1997-dodge-viper-gts) says only six-speed manual, and the [1998 Supra brochure transcription](https://www.mkiv.com/specifications/98_brochure/index.html) also omits numerical gears and final drive. Neither should be cited as proof of those numbers.

- **Supra:** The [Toyota 1997 New Car Features document](https://www.scribd.com/document/23325338/1997-New-Car-Features), Appendix printed125 and differential printed26, supports US/Canada Turbo V160 gears **[3.827, 2.360, 1.685, 1.312, 1.000, 0.793]** and final **3.133**. Those are a **related-year estimate** for the chosen 1998 US proxy; the MC3 market and trim remain unresolved. Its six ratios and final drive are not established by the 1998 sales brochure.
- **Viper:** The [Dodge 2005 SRT10 OEM specification sheet](https://s3.amazonaws.com/chryslermedia.iconicweb.com/mediasite/attachments/ViperSpecifications.pdf), printed9/PDF1, supports **[2.66, 1.78, 1.30, 1.00, 0.74, 0.50]**, final **3.07** for that later SRT10. Applying them to 1996/97 GTS remains a **related-generation estimate** until period OEM support is retrieved. The [1997 OEM parts catalog](https://www.viperclub.org/downloads/1997_Viper_Parts_Listing.pdf), PDF6/Figure3-110, labels differential04763897AB only “3.0 Ratio”; that rounded label does not prove precise3.07.

Secondary specification sites corroborate the Viper numbers but were not promoted to primary evidence. No runtime profile, asset or benchmark input was edited during this audit. Supplement facts now explicitly record both uncertainty and corrected source attribution.

## Remaining reference audit

A read-only audit found no further numerical contradictions between the current non-estimate runtime parameters and their selected corpus profiles. M3 CSL, NSX, GT500, Evo IX, 911 SC, 350Z and R34 values match within rounding and unit conversion. This does not establish identical real counterparts where trim or market is unresolved.

The FR500 1,630 kg mass is correctly marked game-derived, but its source attribution must point to the native Ford Racing2 FR500 catalog, **not the Ford1999 press release**, which omits mass. Native catalog source locators and their existing SHA256 are now explicit in the supplement. MC3 Supra values belong to the selected1998US Turbo6MT proxy; the asset's market and trim are unresolved. The 911 European market and Evo standard US IX grade also retain their documented qualifications.

Mass conventions remain heterogeneous: the CSL source labels1,385kg with an EU/DIN-plus75kg footnote; the911 uses DIN empty mass, while others use curb or measured test mass. No additional driver mass or cross-convention equivalence is claimed. No runtime JSON, source code or benchmark inputs were changed by either audit.
