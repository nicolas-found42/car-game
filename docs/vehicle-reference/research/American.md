# American vehicle profiles

Sixteen profiles selected from the existing local game-model catalog, with 439 facts and 33 sources. Manufacturer documents lead; launch estimates, archival copies, firsthand test results, and prototype measurements remain labeled. No center-of-gravity height, inertia tensor, damper curve, or tire force curve is invented.

| ID | Reference counterpart | Facts | Identity status |
|---|---|---:|---|
| a01 | 2005 Ford Mustang GT | 28 | catalog_exact_name |
| a02 | 2000 Ford Mustang SVT Cobra R | 26 | catalog_exact_name |
| a03 | 2008 Ford Shelby GT500 | 32 | catalog_exact_name |
| a04 | 2003 Ford SVT F-150 Lightning | 34 | catalog_exact_name |
| a05 | 2004 Ford F-150 FX4 | 22 | trim_year_known_body_configuration_unresolved |
| a06 | 2004 Ford SVT Focus | 38 | catalog_exact_name |
| a07 | 2005 Ford GT | 29 | catalog_exact_name |
| a08 | 2003 Chevrolet Corvette Z06 | 31 | catalog_exact_name |
| a09 | 2007 Chevrolet Corvette Z06 | 27 | catalog_exact_name |
| a10 | 2003 Dodge Viper SRT10 | 27 | catalog_exact_name |
| a11 | 2004 Dodge Ram 1500 Laramie Hemi Quad Cab | 17 | trim_year_known_body_configuration_unresolved |
| a12 | 2004 Chevrolet Silverado SS | 22 | catalog_exact_name |
| a13 | 2003 Hummer H2 | 27 | catalog_exact_name |
| a14 | 2004 Cadillac CTS-V | 28 | catalog_exact_name |
| a15 | 2005 Chrysler 300C | 27 | catalog_exact_name |
| a16 | 1987 Buick GNX | 24 | catalog_exact_name |

The machine-readable source is [american.json](american.json). Each fact identifies its source and original unit. Conditional and tested values have distinct field names.

Repeated-game identities include 2003 Viper SRT10 (GT4/MC3), 1987 GNX (GT4/MCLA), 2005 Chrysler 300C (GT4/MC3/MCLA ambient), and 2004 CTS-V (MC3/MCLA ambient). Related assets with unresolved trim or year are recorded separately.

## a01 — 2005 Ford Mustang GT

MCLA names a 2005 Mustang GT convertible, a different body variant. Do not apply coupe mass or roll calibration to the convertible.

Assets: `GT4_FORD0011`.

Sources:

- [Ford 2005 Mustang US sales brochure, pp. 8, 13–15](https://www.auto-brochures.com/makes/Ford/Mustang/Ford_US%20Mustang_2005.pdf) — manufacturer_document.
- [MotorTrend 2005 Mustang GT instrumented automatic test, October 13 2004](https://www.motortrend.com/reviews/2005-ford-mustang-gt) — instrumented_test.
- [Ford 2005 Mustang dealer product guide: Key Product Specifications](https://shnack.com/docs/05_mustang_product_guide.pdf) — manufacturer_document_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a02 — 2000 Ford Mustang SVT Cobra R

Ford Racing 2 COBRA is labeled SVT COBRA 2000 without R; it is not verified as this trim. The Ford overview alternates between 20- and 21-gallon fuel cells, so fuel capacity is omitted.

Assets: `GT4_FORD0008`.

Sources:

- [Ford SVT 2000 Mustang Cobra R Overview (manufacturer press text mirror)](https://terminator-cobra.com/2000%20SVT%20Mustang%20Cobra%20R%20Overview.pdf) — manufacturer_document_mirror.
- [MotorTrend November 2000 supercar shootout, republished May 2025](https://www.motortrend.com/reviews/supercar-top-speed-test-2000-america-versus-the-world) — instrumented_test.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a03 — 2008 Ford Shelby GT500

The Redline addon explicitly names a 2008 GT500. Roush Stage 3 and FR500 variants are excluded. Coupe spring and stabilizer settings differ from convertible settings.

Assets: `REDLINE_ADDON_080_MUSTANG_S197_PACK_ZIP_MUSTANG_NNB_GT500_867D18EE049F_18D1CC5DE2FE0CDE`.

Sources:

- [Ford 2008 Shelby GT500 Owner Guide Supplement, pp.4,11,13](https://www.mustangspecs.com/wp-content/uploads/2020/05/2008-Ford-Shelby-GT500-Supplement.pdf) — manufacturer_document_mirror.
- [Ford 2008 Shelby GT500 eSourceBook, June 2007, p.52](https://themustangsource.com/forums/attachments/f637/46859d1187387886-2008-mustang-shelby-e-source-books-better-than-order-guide-08-20shelby-20sb.pdf) — manufacturer_document_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a04 — 2003 Ford SVT F-150 Lightning

GT4 explicitly names the 2003 Lightning. Ford Racing 2 LIGHTNING_SVT names 2002 and is retained as a related model year, without inheriting 2003 facts.

Assets: `GT4_FORD0012`.

Sources:

- [Ford 2003 SVT F-150 Lightning Owner Guide Supplement, pp.4,7–8](https://www.fordservicecontent.com/ford_content/catalog/owner_guides/03f12sv1e.pdf) — manufacturer_document.
- [2003 SVT F-150 Lightning Ford specification table mirror](https://www.ford-trucks.com/specs/2003-2/2003-special-vehicle-team-svt-f-150-lightning-specifications/) — manufacturer_specification_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a05 — 2004 Ford F-150 FX4

Ford Racing 2 names the 2004 FX4, but cab and bed configurations remain unresolved. Body-dependent dimensions are alternatives rather than one fitted geometry.

Assets: `F150_2004`.

Sources:

- [Ford Motor Company 2004 F-150 specifications (credited reproduction)](https://www.fordf150.net/specs/04f150.php) — manufacturer_specification_mirror.
- [Ford 2004 F-150 Early Preview sales training brochure](https://xr793.com/wp-content/uploads/2024/02/2004-Ford-F-150-Early-Preview.pdf) — manufacturer_document_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.
- Cab/bed, curb mass, axle option, tires and engine configuration must be reconciled against the FR2 mesh and native vehicle definition.

## a06 — 2004 Ford SVT Focus

MCLA explicitly names the 2004 SVT Focus. Ford Racing 2 SVT Focus 2002 and GT4 ST170 2003 are related vehicles with different year or market identities.

Assets: `MCLA_VP_FRD_FOCUS_04`.

Sources:

- [Ford 2004 SVT Focus Owner Guide Supplement, pp.4,10–11](https://www.fordservicecontent.com/Ford_Content/catalog/owner_guides/04focsv1e.pdf) — manufacturer_document.
- [Ford 2004 SVT Product Information, pp.85–87](https://terminator-cobra.com/SVT%20New%20for%202004.pdf) — manufacturer_document_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.
- Resolve minor spring-rate differences between2004press kit and Owner Supplement; exact3-door/5-door/EAPmesh configuration remains unverified.

## a07 — 2005 Ford GT

Both GT4 codes explicitly name Ford GT 2005 and represent one real-life counterpart. Ford Racing 2 FORD_GT is labeled 2004 and may represent preproduction specification.

Assets: `GT4_FORD0006`, `GT4_FORD0015`.

Sources:

- [Ford GT US brochure: internal table explicitly headed 2005 SPECIFICATIONS, p.2](https://www.auto-brochures.com/makes/Ford/GT/Ford_US%20GT_2006.pdf) — manufacturer_document_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a08 — 2003 Chevrolet Corvette Z06

MC3 explicitly names the 2003 Z06. GT4 names Z06 2000, so it is not merged with this 405-horsepower 2003 profile. A period specification table has inconsistent brake unit conversions; inches are preserved.

Assets: `MC3_VP_CORVETTEZ06_03`.

Sources:

- [Chevrolet 2003 Corvette US specification sheet 03CHECORSPE01](https://xr793.com/wp-content/uploads/2020/04/2003-Chevrolet-Corvette-Spec-Sheet.pdf) — manufacturer_document_mirror.
- [2003 Corvette LS6/Z06 period specifications table](https://www.corvetteactioncenter.com/specs/c5/2003/2003specs.html) — period_specification_reproduction.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a09 — 2007 Chevrolet Corvette Z06

MCLA explicitly names the 2007 Z06. The Redline C6 Z06 addon is related by generation and trim, with its model year unresolved. C6.R, Z06.R and ZR1 variants are excluded.

Assets: `MCLA_VP_CHV_CORVETTEZ06_07`.

Sources:

- [GM Media 2007 Corvette specifications, release August 1 2006](https://www.corvetteactioncenter.com/specs/c6/2007/2007specs.html) — manufacturer_press_release_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a10 — 2003 Dodge Viper SRT10

GT4 names the 2003 SRT10 and the MC3 source code names Viper 2003. The Redline base Viper lacks a year and trim; it is not merged with this roadster or a later coupe.

Assets: `GT4_DODG0005`, `MC3_VP_VIPER_03`.

Sources:

- [DaimlerChrysler 2003 Dodge Viper SRT10 Overview, August 19 2002](https://s3.amazonaws.com/chryslermedia.iconicweb.com/mediasite/attachments/08192002_VIPER_OVERVIEW.pdf) — manufacturer_press_release.
- [MotorTrend 2003 Viper SRT10 first-hand road review, July 9 2003](https://www.motortrend.com/reviews/2003-dodge-viper-srt-10) — first_hand_vehicle_review.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a11 — 2004 Dodge Ram 1500 Laramie Hemi Quad Cab

GT4 names the 2004 Laramie Hemi Quad Cab, without a bed or drivetrain specification. Do not select a generic Ram curb mass until its exact configuration is established.

Assets: `GT4_DODG0009`.

Sources:

- [DaimlerChrysler 2004 Ram1500 US brochure, six pages](https://xr793.com/wp-content/uploads/2021/11/2004-Dodge-Ram-1500.pdf) — manufacturer_document_mirror.
- [DaimlerChrysler 2004 DR Ram Parts Catalog model code definitions](https://mopar1973man.com/external-media/pdf/2004Ramparts.pdf) — manufacturer_parts_catalog_mirror.
- [Ira Siegel first-hand2004Ram1500HEMIQuadCabLaramiepackage road review](https://drivechicago.com/reviews/article/245/ram-ahead) — first_hand_vehicle_review.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.
- Curb mass, exact bed/drivetrain, torque and RPM, tires, ratios, track, length and height still require exact2004 manufacturer specification table. No2005values silently backfilled.

## a12 — 2004 Chevrolet Silverado SS

MC3 explicitly names Silverado SS 2004. The MCLA ambient Silverado 2004 filename omits SS and remains a related family asset with unverified trim and drivetrain.

Assets: `MC3_VP_SILVERADO_SS_04`.

Sources:

- [Chevrolet2004 Silverado US spec sheet 04CHESILSPE01](https://xr793.com/wp-content/uploads/2021/11/2004-Chevrolet-Silverado-Spec-Sheet.pdf) — manufacturer_document_mirror.
- [GM2004 Chevrolet Silverado Owner Manual SS towing row](https://assets.gm.com/manuals/chevrolet/2004_chevrolet_silverado_owners.pdf) — manufacturer_document.
- [GM2004ChevroletSilveradoRestorationKit, printedpp.3,10](https://news.chevrolet.com/content/dam/company/no_search/heritage-archive-docs/vehicle-information-kits/chevrolet/2004-Chevrolet-Silverado.pdf) — manufacturer_document.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.
- SS-specific curb mass, geometry, tire sizes and individual gear ratios not recovered from original specification source yet; do not use generic1500payload/GVWR figures.
- GM2004restorationkit has a suspect4L85Etransmissionlabel. Generic1500dimensions/masses inthiskitare notSS-specific; do notinherit them.

## a13 — 2003 Hummer H2

MC3 H2_03 explicitly identifies the H2 and 2003. The separate MC3 hummer_02 asset is excluded. The Redline H2 addon lacks a model year and remains a related model.

Assets: `MC3_VP_H2_03`.

Sources:

- [HUMMER2003 H2 launch technical briefing preliminary specifications reproduction](https://www.rockcrawler.com/reviews/hummer/h2/2003_tech_briefing/specs.asp) — manufacturer_technical_briefing_mirror.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a14 — 2004 Cadillac CTS-V

MC3 and MCLA filenames explicitly identify CTS-V 2004. The MCLA asset is ambient; its geometry identity does not imply playable physics or shared native tuning.

Assets: `MC3_VP_CTSV_04`, `MCLA_VA_CAD_CTSV_04`.

Sources:

- [GM2004 Cadillac CTS-V Vehicle Order Guide, April9 2004 pp.2,12–13](https://www.cadillacvnet.com/tech-center/cts-v/2004/2004_cts-v_rpo.pdf) — manufacturer_document_mirror.
- [MotorTrend April2004 CTS-V/S-TypeR instrumented comparison, republished2026](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive) — instrumented_test.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a15 — 2005 Chrysler 300C

GT4, MC3 and MCLA ambient names identify 300C 2005. The playable MCLA SRT8 has a different engine and year and is excluded. The asset drivetrain must still be reconciled with RWD and AWD alternatives.

Assets: `GT4_CHRY0009`, `MC3_VP_CHRYSLER300C_05`, `MCLA_VA_CHR_300C_05`.

Sources:

- [DaimlerChrysler2005 Chrysler300 product information specifications, pp.12–15](https://s3.amazonaws.com/chryslermedia.iconicweb.com/mediasite/specs/2005300Specs.pdf) — manufacturer_press_release.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.

## a16 — 1987 Buick GNX

GT4 and MCLA explicitly identify GNX 1987. The period Car and Driver measurements describe a prototype and quote 300 hp and 380 lb-ft; they remain separate from the advertised production figures reported by a later firsthand GM collection visit.

Assets: `GT4_BUIC0001`, `MCLA_VP_BUK_GRANDNATIONAL_87`.

Sources:

- [GM GNX project history by MarkReuss, February17 2025](https://news.gm.com/home.detail.html/Pages/topic/us/en/2025/feb/0217-buick.html) — manufacturer_history.
- [Car and Driver May1987 GNX prototype instrumented test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/) — instrumented_test.
- [GNX creators interviews: identifies1987testcars asprototypes](https://www.caranddriver.com/features/a23739443/buick-regal-grand-national-gnx-history/) — first_hand_creator_interviews.
- [RickJensen firsthandGMwarehouse visit toGNX001, March31 2014](https://www.motortrend.com/news/1403-inside-gnx-001-1987-buick-regal) — first_hand_vehicle_review.

Remaining uncertainties:

- Center of gravity height, inertia tensor, tire force curves and damper curves not published in retrieved sources.
- Factory production rating, final curb mass, tire specifications, gear ratios and exact production dimensions need original GNX-specific manufacturer documents; prototype table is clearly segregated.

## Conflicts that affect calibration

- The Focus uses two final drives: 2.88 for gears 1, 2, 5 and 6; 4.25 for gears 3, 4 and reverse. Its owner supplement supplies overall ratios, which must not be multiplied by another final drive.
- Focus spring rates differ slightly between the owner supplement and press kit. The press kit also prints a rear brake diameter inconsistent with the owner supplement. Preserve the conflict instead of averaging.
- GT500 coupe and convertible springs and stabilizers differ substantially. The selected profile uses coupe settings.
- The GM Silverado kit labels its SS transmission 4L85-E while reporting a 3.06 first gear. That model designation is questionable and is not imported as an authoritative gearbox identity. Generic 1500 mass and geometry rows are not automatically SS specifications.
- Several manufacturer documents have inconsistent customary-to-metric torque or mass conversions. Original customary values are retained where the printed pair conflicts.
- GNX prototype test results are an envelope, not a production calibration baseline.
- A generic 2004 Ram brochure URL returned a 2009 document; it was rejected. The linked six-page DaimlerChrysler 2004 brochure is correctly dated.

No simulation settings or model archive files were changed.
