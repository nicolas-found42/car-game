# Additional physics profiles for the 40-car fleet

This audit contributes 27 explicit profiles to the existing 13-car fleet. The machine-readable `profile-expansion.json` contains every runtime parameter, field-level evidence status, original source value/unit, and source-field identifier. It uses the existing reference corpus as its authoritative source and does not claim to recover original game handling. Source claims below refer to the corpus extracts; no new web claims are introduced.

The existing 13 profiles and the corpus are unchanged. These additions are provisional calibration inputs, not declarations that all 40 cars reproduce published road tests. Authored mesh wheelbase, track, wheel radius, and body bounds remain separate geometry inputs. Tire friction, CG heights, effective wheel frequencies, damper ratios, anti-roll fractions, steering, losses, most differential splits, and most operating masses without exact references are estimates. The simulation constructs approximate box inertia from authored dimensions and estimated CG; no measured inertia tensors were recovered.

## Selection and applicability

Each addition selects the first corpus asset code. This deliberately selects GT4 Ford Lightning 2003 rather than FR2 Lightning2002, Redline classic Mini rather than an unspecified alternate game asset, and GT4 Subaru WRC2003. Focus and both Minis provide front-wheel-drive cases; pickups, Hummer, rear/mid-engine cars, small kei sports cars, road AWD and WRC provide distinct mass/layout envelopes.

| Runtime ID | Car | Mass kg | kW | Front drive | Estimate fields | Mass basis / key qualification |
|---|---|---:|---:|---:|---:|---|
| ref-a01 | Ford Mustang GT | 1596.6 | 223.7 | 0.00 | 15 | related_reference: tested_curb_mass |
| ref-a02 | Ford Mustang SVT Cobra R | 1628.4 | 287.1 | 0.00 | 18 | reference: published_curb_mass |
| ref-a04 | Ford SVT F-150 Lightning | 2118.3 | 283.4 | 0.00 | 13 | reference: curb_mass |
| ref-a05 | Ford F-150 FX4 | 2096.0 | 223.7 | 0.50 | 17 | game_proxy: catalog.kg |
| ref-a06 | Ford SVT Focus | 1247.4 | 126.8 | 1.00 | 16 | reference: curb_mass |
| ref-a07 | Ford GT | 1538.0 | 410.1 | 0.00 | 15 | estimate: 1538kg operating-mass estimate; native GT4catalog1451kg is retained in corpus but not assumed wet real-life mass. |
| ref-a09 | Chevrolet Corvette Z06 | 1421.0 | 376.6 | 0.00 | 17 | reference: curb_mass |
| ref-a11 | Dodge Ram 1500 Laramie Hemi Quad Cab | 2390.0 | 257.3 | 0.50 | 21 | game_proxy: catalog.kg |
| ref-a12 | Chevrolet Silverado SS | 2410.0 | 257.3 | 0.40 | 18 | estimate: Runtime operating mass estimate; exact sourced wet curb mass/configuration unavailable. |
| ref-a13 | Hummer H2 | 2903.0 | 242.4 | 0.40 | 16 | reference: curb_mass |
| ref-a14 | Cadillac CTS-V | 1746.0 | 298.3 | 0.00 | 16 | reference: curb_mass |
| ref-a15 | Chrysler 300C | 1844.0 | 253.5 | 0.00 | 15 | reference: curb_mass_rwd |
| ref-a16 | Buick GNX | 1600.0 | 205.8 | 0.00 | 17 | game_proxy: catalog.kg |
| ref-e02 | Ferrari Testarossa | 1506.0 | 286.8 | 0.00 | 14 | related_reference: mass |
| ref-e03 | Lamborghini Gallardo | 1520.0 | 367.0 | 0.30 | 18 | estimate: 1520kg operating mass estimate = published1430kg dry/no-fuel plus90kg estimated fluids/fuel. No source calls1520kg measured curb mass. |
| ref-e04 | Lamborghini Murcielago | 1740.0 | 426.0 | 0.30 | 17 | estimate: 1740kg operating mass estimate above related launch1650kg unspecified-basis mass; no exact2004wet-curb figure verified. |
| ref-e08 | MINI Cooper S | 1180.0 | 120.0 | 1.00 | 17 | estimate: 1180kg operating-mass estimate; native GT4catalog1120kg is not asserted to be a wet curb measurement. |
| ref-e09 | Morris Mini Cooper S 1275 | 640.0 | 55.9 | 1.00 | 16 | related_reference: mass_rubber_suspension |
| ref-e10 | Volkswagen Golf IV R32 | 1546.0 | 177.0 | 0.50 | 15 | related_reference: reference_2004US_mass |
| ref-e11 | Audi R8 4.2 FSI quattro | 1635.2 | 313.2 | 0.10 | 15 | reference: mass_manual |
| ref-e12 | Audi RS4 | 1650.0 | 313.2 | 0.40 | 18 | related_reference: reference_2006EU_mass |
| ref-e13 | Lotus Elise 111R | 860.0 | 141.0 | 0.00 | 15 | reference: mass |
| ref-j04 | Mazda RX-7 Spirit R Type A | 1270.0 | 206.0 | 0.00 | 15 | reference: vehicle_mass |
| ref-j07 | Honda S2000 | 1240.0 | 183.9 | 0.00 | 14 | reference: vehicle_mass |
| ref-j11 | Subaru Impreza Sedan WRX STi | 1440.0 | 206.0 | 0.50 | 16 | reference: vehicle_mass |
| ref-j12 | Subaru Impreza Rally Car | 1230.0 | 224.0 | 0.50 | 22 | game_proxy: catalog.kg |
| ref-j16 | Suzuki Cappuccino | 690.0 | 47.1 | 0.00 | 14 | reference: vehicle_mass |

## Important source and representation decisions

- Focus publishes rounded overall reductions `[12.7,7.7,5.7,4.6,3.8,3.1]`. The gearbox has two different final drives, so the runtime stores these overall values in `gears` and sets the multiplier `final_drive=1`. It does not multiply by a second final drive. [am_focus_manual](https://www.fordservicecontent.com/Ford_Content/catalog/owner_guides/04focsv1e.pdf), [am_focus_2004_press](https://terminator-cobra.com/SVT%20New%20for%202004.pdf).
- MINI Cooper S likewise uses related 2003 US overall reductions `[11.4,7.1,5.3,4.4,3.6,2.9]` with identity final multiplier; the asset is a 2002 car with unresolved market. [e08-2003-us](https://www.press.bmwgroup.com/usa/article/attachment/T0018282EN_US/36018).
- Testarossa uses the handbook complete reductions `[10.091,6.473,4.906,3.752,2.813]` and identity final multiplier. These already incorporate the reduction train; this is a related 1988 European reference for a 1991 asset. [e02-eu1988](https://cdn.made-in-maranello.fr/documents/testarossa_1988.pdf).
- S2000 uses original published gearbox ratios and effective final multiplier `4.1 * 1.16 = 4.756`, preserving the separately documented primary reduction. Its 250PS rating converts to183.8747kW, not250 mechanical horsepower. [j07-honda-spec](https://www.honda.co.jp/factbook/auto/s2000/199904/050.html).
- Gallardo operating mass1520kg is an explicit estimate above the published1430kg dry/no-fuel mass; no source is labeled as measuring1520kg wet. Murcielago1740kg is likewise an estimate above a related launch1650kg unspecified-basis reference. [e03-launch](https://lamborghiniclubwest.com/GALLARDO_ENG_DAT.pdf), [e04-launch](https://www.lamborghiniclubwest.com/murcielago.pdf).
- Golf2003EU retains177kW source rating but uses qualified related2004US1546kg mass, axle loads and gearing. These are not claimed to be exact2003EU measurements. [e10-history](https://www.volkswagen-newsroom.com/en/publications/more/50-years-or-the-sporty-golf-2718/download), [e10-us2004](https://billswebspace.com/R32TechnicalSpecifications.pdf).
- GNX uses the reported production advertised276hp/360lb-ft ratings. Its curve peak RPMs are separately labeled prototype related references; its1600kg mass is a GT4 game proxy. It does not silently combine the prototype300hp rating with a production276hp claim. [am_gnx_visit](https://www.motortrend.com/news/1403-inside-gnx-001-1987-buick-regal), [am_gnx_test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/).
- Classic Mini chooses the approximately640kg rubber-suspension family reference rather than averaging it with698kg Hydrolastic. Actual mesh year/suspension and the75hp rating standard remain unresolved. [e09-bmc](https://www.manualslib.com/manual/3119843/Bmw-Mini-Cooper-1968.html), [e09-archive](https://www.bmwgroup-classic.com/content/grpw/websites/bmwgroup-classic_com/en/modelle/mini-klassiker/product-description-page.ad-1101-1.morris-mini-cooper-s-1275.content.html).
- Cappuccino51%front is explicitly a two-occupant claim and is only a related reference for static distribution; it is not mislabeled as a measured curb split. Runtime mass690kg remains the separate standard5MT source mass. [j16-suzuki-mechanism](https://images.jdm-catalogues.com/suzuki/cappuccino/suzuki_cappuccino_ea21r/images/l/page.4.webp), [j16-suzuki-brochure](https://images.jdm-catalogues.com/suzuki/cappuccino/suzuki_cappuccino_ea21r/images/l/page.5.webp).
- WRC2003 homologation establishes competition identity and front/rear drive, not event mass, engine map, gearbox, differential control, or tires. The1230kg runtime mass is explicitly the game catalog proxy;224kW/550Nm and other missing fields are estimates for a provisional dry-asphalt tarmac exercise. No road-STi powertrain or loose-surface tire model is claimed. [j12-fia-homologation](https://historicdb.fia.com/sites/default/files/car_attachment/1601041801/homologation_form_number_5652_group_a.pdf).
- R8 nominal10%front is sourced, but source maximum35% and viscous coupling behavior do not by themselves reconstruct its transfer controller. All other nominal AWD allocations here are estimates unless explicitly qualified in the profile. [e11-brochure](https://xr793.com/wp-content/uploads/2024/01/2008-Audi-R8.pdf).
- Cobra R spring rates800/750lb/in are coil rates. They do not establish effective wheel ride frequencies without motion ratios and unsprung mass; the runtime frequencies therefore remain estimates. [am_cobra_press](https://terminator-cobra.com/2000%20SVT%20Mustang%20Cobra%20R%20Overview.pdf).

## Available performance facts for subsequent comparisons

These facts remain source measurements or manufacturer claims with their original units and caveats. They have not been used to force-fit the runtime coefficients. Different test protocols, launch rollout, occupancy, market, transmission and surface make some comparisons contextual rather than acceptance thresholds. The entire performance record remains in `real-profiles.json`.

| Corpus | Fact | Value | Unit | Source | Qualification |
|---|---|---|---|---|---|
| a01 | tested_0_60_mph | 5.1 | s | [am_mustang_test](https://www.motortrend.com/reviews/2005-ford-mustang-gt) | MotorTrend automatic test car; brake-torque launch just over 2000rpm, 200-ft skidpad. Not manual-transmission performance. |
| a01 | tested_quarter_mile | 13.6 | s | [am_mustang_test](https://www.motortrend.com/reviews/2005-ford-mustang-gt) | MotorTrend automatic test car; brake-torque launch just over 2000rpm, 200-ft skidpad. Not manual-transmission performance. |
| a01 | tested_quarter_mile_trap | 99.9 | mph | [am_mustang_test](https://www.motortrend.com/reviews/2005-ford-mustang-gt) | MotorTrend automatic test car; brake-torque launch just over 2000rpm, 200-ft skidpad. Not manual-transmission performance. |
| a01 | tested_60_0_mph | 120 | ft | [am_mustang_test](https://www.motortrend.com/reviews/2005-ford-mustang-gt) | MotorTrend automatic test car; brake-torque launch just over 2000rpm, 200-ft skidpad. Not manual-transmission performance. |
| a01 | tested_skidpad | 0.84 | g | [am_mustang_test](https://www.motortrend.com/reviews/2005-ford-mustang-gt) | MotorTrend automatic test car; brake-torque launch just over 2000rpm, 200-ft skidpad. Not manual-transmission performance. |
| a02 | claimed_skidpad_lower_bound | 1.0 | g | [am_cobra_press](https://terminator-cobra.com/2000%20SVT%20Mustang%20Cobra%20R%20Overview.pdf) | Ford launch press text; power estimated, top speed >170mph and skidpad >1g on 100-ft skidpad. Spring rates are spring rates, not wheel rates. |
| a14 | tested_0_60_mph | 5.1 | s | [am_ctsv_test](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive) | MotorTrend instrumented results; not GMclaimed4.6seconds. Tire/conditions vary; use as scenario envelope. |
| a14 | tested_0_100_mph | 12.3 | s | [am_ctsv_test](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive) | MotorTrend instrumented results; not GMclaimed4.6seconds. Tire/conditions vary; use as scenario envelope. |
| a14 | tested_quarter_mile | 13.48 | s | [am_ctsv_test](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive) | MotorTrend instrumented results; not GMclaimed4.6seconds. Tire/conditions vary; use as scenario envelope. |
| a14 | tested_quarter_mile_trap | 105.33 | mph | [am_ctsv_test](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive) | MotorTrend instrumented results; not GMclaimed4.6seconds. Tire/conditions vary; use as scenario envelope. |
| a14 | tested_60_0_mph | 121 | ft | [am_ctsv_test](https://www.motortrend.com/reviews/comparison-2004-cadillac-cts-v-vs-jaguar-s-type-r-archive) | MotorTrend instrumented results; not GMclaimed4.6seconds. Tire/conditions vary; use as scenario envelope. |
| a16 | tested_0_60_mph | 4.7 | s | [am_gnx_test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/) | C/D prototype test; not verified production-car baseline. Numericresultsonly, weather/rolloutnotreportedintheaccessibletext. |
| a16 | tested_quarter_mile | 13.5 | s | [am_gnx_test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/) | C/D prototype test; not verified production-car baseline. Numericresultsonly, weather/rolloutnotreportedintheaccessibletext. |
| a16 | tested_quarter_mile_trap | 102 | mph | [am_gnx_test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/) | C/D prototype test; not verified production-car baseline. Numericresultsonly, weather/rolloutnotreportedintheaccessibletext. |
| a16 | tested_70_0_mph | 186 | ft | [am_gnx_test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/) | C/D prototype test; not verified production-car baseline. Numericresultsonly, weather/rolloutnotreportedintheaccessibletext. |
| a16 | tested_skidpad_300ft | 0.8 | g | [am_gnx_test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/) | C/D prototype test; not verified production-car baseline. Numericresultsonly, weather/rolloutnotreportedintheaccessibletext. |
| e04 | acceleration_0_100_kmh | 3.8 | s | [e04-brochure](https://www.luxosonline.pt/empresas/catalogos/4_lamborghini_int_murcielago.pdf) | Manufacturer brochure claim |
| e08 | acceleration_0_100_kmh | 7.4 | s | [e08-launch](https://www.press.bmwgroup.com/united-kingdom/article/detail/T0016013EN_GB/mini-cooper-s?language=en_GB) | Source 0–62 mph UK 2002 launch or explicitly dated US reference; asset market is unresolved. |
| e09 | acceleration_0_100_kmh_upper_bound | 11 | s | [e09-archive](https://www.bmwgroup-classic.com/content/grpw/websites/bmwgroup-classic_com/en/modelle/mini-klassiker/product-description-page.ad-1101-1.morris-mini-cooper-s-1275.content.html) | Archive claim less than  11 seconds Classic 1275 S family reference; exact asset year and suspension variant unresolved. |
| e10 | acceleration_0_100_kmh | 6.6 | s | [e10-history](https://www.volkswagen-newsroom.com/en/publications/more/50-years-or-the-sporty-golf-2718/download) | Manufacturerclaim |
| e10 | reference_2004US_acceleration_0_60_mph | 6.4 | s | [e10-us2004](https://billswebspace.com/R32TechnicalSpecifications.pdf) | 2004 US reference; keep separate from 2003 European GT 4 identity. |
| e11 | acceleration_0_60_mph | 4.4 | s | [e11-brochure](https://xr793.com/wp-content/uploads/2024/01/2008-Audi-R8.pdf) | Manufacturerclaimedbothtransmissions |
| e11 | acceleration_0_125_mph | 15.1 | s | [e11-brochure](https://xr793.com/wp-content/uploads/2024/01/2008-Audi-R8.pdf) |  |
| e11 | quarter_mile_time | 12.7 | s | [e11-brochure](https://xr793.com/wp-content/uploads/2024/01/2008-Audi-R8.pdf) |  |
| e12 | reference_2006EU_acceleration_0_100_kmh | 4.8 | s | [e12-eu2006](https://id.scribd.com/document/1027717492/Audi-Rs4-Owners-Manual) | Adjacent 2006 European reference, not assumed exact 2008 USmatch. |
| e13 | acceleration_0_60_mph | 4.9 | s | [e13-brochure](https://kurumacatalog.com/wp-content/uploads/2021/01/2005_Lotus_Elise_catalog.pdf) | Manufacturerclaim |
| e13 | acceleration_0_100_mph | 13 | s | [e13-brochure](https://kurumacatalog.com/wp-content/uploads/2021/01/2005_Lotus_Elise_catalog.pdf) |  |
| e13 | acceleration_0_100_kmh | 5.2 | s | [e13-2004](https://www.lotusdriversguide.com/Downloads/Brochures/Elise_111R_2004.pdf) |  |

## Validation of this contribution

The generator checked exactly27 unique IDs and exact parameter/evidence-key equality with the current production profile schema. Every non-estimate has source and basis. Every original-source identifier resolves in the corpus source registry; game proxies resolve to the selected catalog asset in `game-profiles.json`. Numeric conversions use1hp=0.7456998715822702kW,1PS=0.73549875kW,1lb=0.45359237kg,1lb-ft=1.3558179483314004Nm, and1kgf-m=9.80665Nm. Original source values and units accompany converted runtime parameters. These checks validate this contribution’s structure and traceability, not vehicle behavior; the parent fleet scenario/physics verification supplies runtime evidence.

## Source registry

Each source below is retained from the existing corpus; mirrors preserve manufacturer ownership separately from hosting. Instrumented tests are primary evidence of their tested cars, with applicability caveats retained.
- [am_300c_press](https://s3.amazonaws.com/chryslermedia.iconicweb.com/mediasite/specs/2005300Specs.pdf) — DaimlerChrysler2005 Chrysler300 product information specifications, pp.12–15 (manufacturer_press_release).
- [am_c6_specs](https://www.corvetteactioncenter.com/specs/c6/2007/2007specs.html) — GM Media 2007 Corvette specifications, release August 1 2006 (manufacturer_press_release_mirror).
- [am_cobra_press](https://terminator-cobra.com/2000%20SVT%20Mustang%20Cobra%20R%20Overview.pdf) — Ford SVT 2000 Mustang Cobra R Overview (manufacturer press text mirror) (manufacturer_document_mirror).
- [am_cobra_test](https://www.motortrend.com/reviews/supercar-top-speed-test-2000-america-versus-the-world) — MotorTrend November 2000 supercar shootout, republished May 2025 (instrumented_test).
- [am_ctsv_order](https://www.cadillacvnet.com/tech-center/cts-v/2004/2004_cts-v_rpo.pdf) — GM2004 Cadillac CTS-V Vehicle Order Guide, April9 2004 pp.2,12–13 (manufacturer_document_mirror).
- [am_f150_preview](https://xr793.com/wp-content/uploads/2024/02/2004-Ford-F-150-Early-Preview.pdf) — Ford 2004 F-150 Early Preview sales training brochure (manufacturer_document_mirror).
- [am_f150_specs](https://www.fordf150.net/specs/04f150.php) — Ford Motor Company 2004 F-150 specifications (credited reproduction) (manufacturer_specification_mirror).
- [am_focus_2004_press](https://terminator-cobra.com/SVT%20New%20for%202004.pdf) — Ford 2004 SVT Product Information, pp.85–87 (manufacturer_document_mirror).
- [am_focus_manual](https://www.fordservicecontent.com/Ford_Content/catalog/owner_guides/04focsv1e.pdf) — Ford 2004 SVT Focus Owner Guide Supplement, pp.4,10–11 (manufacturer_document).
- [am_fordgt_brochure](https://www.auto-brochures.com/makes/Ford/GT/Ford_US%20GT_2006.pdf) — Ford GT US brochure: internal table explicitly headed 2005 SPECIFICATIONS, p.2 (manufacturer_document_mirror).
- [am_gnx_test](https://www.caranddriver.com/reviews/a33502193/tested-1987-buick-gnx/) — Car and Driver May1987 GNX prototype instrumented test (instrumented_test).
- [am_gnx_visit](https://www.motortrend.com/news/1403-inside-gnx-001-1987-buick-regal) — RickJensen firsthandGMwarehouse visit toGNX001, March31 2014 (first_hand_vehicle_review).
- [am_h2_brief](https://www.rockcrawler.com/reviews/hummer/h2/2003_tech_briefing/specs.asp) — HUMMER2003 H2 launch technical briefing preliminary specifications reproduction (manufacturer_technical_briefing_mirror).
- [am_lightning_manual](https://www.fordservicecontent.com/ford_content/catalog/owner_guides/03f12sv1e.pdf) — Ford 2003 SVT F-150 Lightning Owner Guide Supplement, pp.4,7–8 (manufacturer_document).
- [am_lightning_specs](https://www.ford-trucks.com/specs/2003-2/2003-special-vehicle-team-svt-f-150-lightning-specifications/) — 2003 SVT F-150 Lightning Ford specification table mirror (manufacturer_specification_mirror).
- [am_mustang_brochure](https://www.auto-brochures.com/makes/Ford/Mustang/Ford_US%20Mustang_2005.pdf) — Ford 2005 Mustang US sales brochure, pp. 8, 13–15 (manufacturer_document).
- [am_mustang_test](https://www.motortrend.com/reviews/2005-ford-mustang-gt) — MotorTrend 2005 Mustang GT instrumented automatic test, October 13 2004 (instrumented_test).
- [am_ram_brochure](https://xr793.com/wp-content/uploads/2021/11/2004-Dodge-Ram-1500.pdf) — DaimlerChrysler 2004 Ram1500 US brochure, six pages (manufacturer_document_mirror).
- [am_ram_roadreview](https://drivechicago.com/reviews/article/245/ram-ahead) — Ira Siegel first-hand2004Ram1500HEMIQuadCabLaramiepackage road review (first_hand_vehicle_review).
- [am_silverado_2004_kit](https://news.chevrolet.com/content/dam/company/no_search/heritage-archive-docs/vehicle-information-kits/chevrolet/2004-Chevrolet-Silverado.pdf) — GM2004ChevroletSilveradoRestorationKit, printedpp.3,10 (manufacturer_document).
- [am_silverado_manual](https://assets.gm.com/manuals/chevrolet/2004_chevrolet_silverado_owners.pdf) — GM2004 Chevrolet Silverado Owner Manual SS towing row (manufacturer_document).
- [am_silverado_sheet](https://xr793.com/wp-content/uploads/2021/11/2004-Chevrolet-Silverado-Spec-Sheet.pdf) — Chevrolet2004 Silverado US spec sheet 04CHESILSPE01 (manufacturer_document_mirror).
- [e02-eu1988](https://cdn.made-in-maranello.fr/documents/testarossa_1988.pdf) — Ferrari Testarossa 1988 European owner handbook, printed pages 10–16 (manufacturer publication).
- [e03-launch](https://lamborghiniclubwest.com/GALLARDO_ENG_DAT.pdf) — Lamborghini Gallardo 2003 launch press kit and technical sheet (manufacturer publication).
- [e04-brochure](https://www.luxosonline.pt/empresas/catalogos/4_lamborghini_int_murcielago.pdf) — Lamborghini London/Manchester Murcielago dealer brochure (manufacturer publication).
- [e04-launch](https://www.lamborghiniclubwest.com/murcielago.pdf) — Lamborghini Murcielago original 6.2 launch press dossier, technical sheet page 16 (manufacturer publication).
- [e08-2003-us](https://www.press.bmwgroup.com/usa/article/attachment/T0018282EN_US/36018) — MINICooper S 2003 USproductinformation (manufacturer publication).
- [e08-launch](https://www.press.bmwgroup.com/united-kingdom/article/detail/T0016013EN_GB/mini-cooper-s?language=en_GB) — MINI Cooper S UK 2002 launchrelease (manufacturer publication).
- [e09-archive](https://www.bmwgroup-classic.com/content/grpw/websites/bmwgroup-classic_com/en/modelle/mini-klassiker/product-description-page.ad-1101-1.morris-mini-cooper-s-1275.content.html) — BMWGroup Classic Morris Mini Cooper S 1275 (manufacturer publication).
- [e09-bmc](https://www.manualslib.com/manual/3119843/Bmw-Mini-Cooper-1968.html) — BMC Mini workshop manual, Cooper S General Data pages 14, 16, 17 (manufacturer publication).
- [e10-history](https://www.volkswagen-newsroom.com/en/publications/more/50-years-or-the-sporty-golf-2718/download) — Volkswagen 50 yearssporty Golf vehicle 18 Golf IVR 32 (manufacturer publication).
- [e10-us2004](https://billswebspace.com/R32TechnicalSpecifications.pdf) — Volkswagen of America 2004 U.S.R 32 Technical Specifications (manufacturer publication).
- [e11-brochure](https://xr793.com/wp-content/uploads/2024/01/2008-Audi-R8.pdf) — Audi of America 2008 R 8 brochure C-R 8-08-01 US (manufacturer publication).
- [e12-brochure](https://xr793.com/wp-content/uploads/2019/08/2008-Audi-Ful-Line.pdf) — Audi of America 2008 full-linebrochure (manufacturer publication).
- [e12-eu2006](https://id.scribd.com/document/1027717492/Audi-Rs4-Owners-Manual) — Audi RS 4 European 2006 salesbrochure (misnamed Owners Manualbyhost) (manufacturer publication).
- [e12-us2008-release](https://www.audiworld.com/model/rs4/08/RS4_Cab_PR.pdf) — Audi of America 2008 RS 4 model announcement June 6,2007 (manufacturer publication).
- [e13-2004](https://www.lotusdriversguide.com/Downloads/Brochures/Elise_111R_2004.pdf) — Lotus Elise 111 R 2004 factory technical leaflet (manufacturer publication).
- [e13-brochure](https://kurumacatalog.com/wp-content/uploads/2021/01/2005_Lotus_Elise_catalog.pdf) — Lotus The Elisein Brief LOTV 505062 (manufacturer publication).
- [j04-mazda-brochure](https://kurumacatalog.com/wp-content/uploads/2023/08/2002_Mazda_RX-7_catalog_jp.pdf) — Mazda RX-7 Japanese brochure March2002 technicaldata PDF21 (manufacturer_brochure_archived).
- [j07-honda-layout](https://www.honda.co.jp/factbook/auto/s2000/199904/004.html) — Honda S2000 behindaxle layout (manufacturer_primary).
- [j07-honda-spec](https://www.honda.co.jp/factbook/auto/s2000/199904/050.html) — Honda S2000 April1999 factbook specifications (manufacturer_primary).
- [j11-subaru-catalog](https://ucar.subaru.jp/php/catalog/grade.php?cat_id=4502753) — Subaruofficial usedcar catalogNovember2002WRXSTi (manufacturer_hosted_catalog).
- [j16-suzuki-brochure](https://images.jdm-catalogues.com/suzuki/cappuccino/suzuki_cappuccino_ea21r/images/l/page.5.webp) — SuzukiCappuccino May1995brochure,finalspecificationpanel (manufacturer_brochure_archived).
- [j16-suzuki-mechanism](https://images.jdm-catalogues.com/suzuki/cappuccino/suzuki_cappuccino_ea21r/images/l/page.4.webp) — Suzuki1995Cappuccino brochure EngineandMechanism pages5–6 (manufacturer_brochure_archived).
