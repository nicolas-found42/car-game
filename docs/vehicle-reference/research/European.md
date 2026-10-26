# European vehicle reference research

Accessed 23 October 2026. This file covers 17 real vehicles backed by catalog assets. The companion `european.json` preserves atomic facts, original units, source IDs, variant scope and missing data. Facts are manufacturer specifications or explicitly marked primary firsthand reporting; they are not independently measured validation results.

Mass definitions remain separate: dry, DIN unladen, EU with driver/luggage, US curb and unspecified weight are not interchangeable. Source horsepower labels are retained, including inconsistent published kW/hp pairs. Adjacent years and markets are called out on every affected fact. Numeric arrays preserve gear order; an overall drive ratio must not be multiplied by a final drive again.

## e01: Porsche 911 SC 3.0

The matching French 1982 handbook supplies geometry, DIN unladen mass, engine ratings, tooth counts, tire options and performance load basis. Its wheelbase is the real reference for auditing the game mesh. The native author market is still inferred.

Sources: [Porsche Classic: Model 911 SC](https://www.porsche.com/australia/accessoriesandservice/classic/models/911-g/911-sc/); [Porsche 911 SC 1982 French owner handbook, printed pages 67–72](https://www.manualslib.fr/manual/1040031/Porsche-911-Sc-1982.html).

Collected 33 atomic facts. Identity: `model_year_confirmed_market_inferred`.

## e02: Ferrari Testarossa

The usable scanned handbook is European 1988, while the asset is 1991. It provides extensive engine, transmission, overall reduction, geometry, tire, suspension and brake data, but every fact remains an adjacent-year reference. The English curb-weight label and Italian empty-weight label do not define fuel or occupants.

Sources: [Ferrari Testarossa US owner manual (1988)](https://www.manualslib.com/manual/466982/Ferrari-Testarossa.html); [Ferrari Testarossa 1988 European owner handbook, printed pages 10–16](https://cdn.made-in-maranello.fr/documents/testarossa_1988.pdf).

Collected 37 atomic facts. Identity: `model_year_confirmed_market_unresolved`.

## e03: Lamborghini Gallardo

The original Gallardo launch dossier contains a full technical sheet, including dry mass, axle distribution, suspension, brake sizes and claimed braking deceleration. Its printed mass excludes fuel; it is not curb mass.

Sources: [Lamborghini Gallardo 2003 launch press kit and technical sheet](https://lamborghiniclubwest.com/GALLARDO_ENG_DAT.pdf).

Collected 37 atomic facts. Identity: `catalog_exact_model_variant`.

## e04: Lamborghini Murcielago

The 6.2 launch press dossier adds geometry, gear ratios, axle distribution, active wing schedule and suspension details. Its unqualified weight and year scope remain explicit. The original power-peak speed conflicts with the later manufacturer retrospective.

Sources: [Lamborghini London/Manchester Murcielago dealer brochure](https://www.luxosonline.pt/empresas/catalogos/4_lamborghini_int_murcielago.pdf); [Lamborghini Murcielago: first V 12 ofnewmillennium](https://www.lamborghini.com/en-en/news/lamborghini-murcielago-the-first-v12-engine-of-the-new-millennium); [Lamborghini Murcielago original 6.2 launch press dossier, technical sheet page 16](https://www.lamborghiniclubwest.com/murcielago.pdf).

Collected 47 atomic facts. Identity: `catalog_exact_model_variant`.

## e05: Lamborghini Miura P400 SV

The manufacturer SV retrospective confirms SV-specific power, torque, widened rear track, wider rear wheels and lubrication/suspension changes. A precise SV dimensional drawing and defined-basis mass were not verified; contradictory secondary tables were excluded.

Sources: [Lamborghini Miura SV: iconic supercar turns 50](https://www.lamborghini.com/jp-en/%E3%83%8B%E3%83%A5%E3%83%BC%E3%82%B9/lamborghini-miura-sv-the-iconic-supercar-turns-50).

Collected 12 atomic facts. Identity: `catalog_exact_model_variant`.

## e06: BMW M3 CSL

The factory CSL press kit includes technical tables and braking claims. Its printed mass footnote is retained exactly rather than relabelled using a commonly repeated internet curb weight.

Sources: [BMW M 3 CSL March 2003 press kit](https://www.press.bmwgroup.com/usa/article/attachment/T0022168EN_US/41493/AG030228_M3_CSL_Press_Kit_GB_finalversion.pdf).

Collected 36 atomic facts. Identity: `catalog_exact_model_variant`.

## e07: BMW M5

The E60 launch kit supplies geometry, engine, SMG seven-speed ratios, final drive, tires, brakes and aero. This identity is distinct from the older E39 M5.

Sources: [BMW new M 5 September 2004 launch press kit](https://www.press.bmwgroup.com/usa/article/attachment/T0018520EN_US/36431); [BMW M 5 UKMay 2005 presskit](https://www.press.bmwgroup.com/united-kingdom/article/detail/T0013049EN_GB/the-new-bmw-m5?language=en_GB).

Collected 39 atomic facts. Identity: `catalog_exact_model_variant`.

## e08: MINI Cooper S

The UK 2002 launch release anchors the R53 supercharged Cooper S. A 2003 US product sheet is separately scoped; its published overall drive ratios already include the unusual split final-drive arrangements.

Sources: [MINI Cooper S UK 2002 launchrelease](https://www.press.bmwgroup.com/united-kingdom/article/detail/T0016013EN_GB/mini-cooper-s?language=en_GB); [BMW MINI 2002 owner manual 09/02](https://www.notice-facile.com/en/manual/1239347/mini%2Bcooper-s-2002); [MINICooper S 2003 USproductinformation](https://www.press.bmwgroup.com/usa/article/attachment/T0018282EN_US/36018).

Collected 24 atomic facts. Identity: `catalog_exact_model_variant`.

## e09: Morris Mini Cooper S 1275

The BMC workshop manual distinguishes 1275-specific engine data, standard and optional transmissions, rim-dependent tracks and two suspension-dependent kerbside masses. The exact modeled year and suspension cannot be inferred from the family name.

Sources: [BMWGroup Classic Morris Mini Cooper S 1275](https://www.bmwgroup-classic.com/content/grpw/websites/bmwgroup-classic_com/en/modelle/mini-klassiker/product-description-page.ad-1101-1.morris-mini-cooper-s-1275.content.html); [BMWClassic Mini Timeline 1959–2000](https://www.press.bmwgroup.com/united-kingdom/article/detail/T0330170EN_GB/classic-mini-timeline%3A-1959-2000?language=en_GB); [BMC Mini workshop manual, Cooper S General Data pages 14, 16, 17](https://www.manualslib.com/manual/3119843/Bmw-Mini-Cooper-1968.html).

Collected 32 atomic facts. Identity: `model_variant_confirmed_year_unresolved`.

## e10: Volkswagen Golf IV R32

The European GT4 2003 Mk4 R32 and US 2004 references are kept separate. The US factory technical sheet has a complete dimensional, gearing, suspension and brake table. MCLA traffic and playable assets require distinct handling.

Sources: [Volkswagen 50 yearssporty Golf vehicle 18 Golf IVR 32](https://www.volkswagen-newsroom.com/en/publications/more/50-years-or-the-sporty-golf-2718/download); [Volkswagen of America 2004 U.S.R 32 Technical Specifications](https://billswebspace.com/R32TechnicalSpecifications.pdf).

Collected 43 atomic facts. Identity: `cross_game_family_match_market_year_differ`.

## e11: Audi R8 4.2 FSI quattro

The US 2008 R8 brochure provides performance, axle distribution, transmission-dependent weight and quattro behavior. European brochure data is a separate market reference; US and European acceleration cycles are not interchangeable.

Sources: [Audi of America 2008 R 8 brochure C-R 8-08-01 US](https://xr793.com/wp-content/uploads/2024/01/2008-Audi-R8.pdf); [Audi R 8 UKPricingand Specification Guide October 2007](https://cache3.arabwheels.sa/system/brochures/66/original/Audi-R8-2007-UK.pdf?1762345591=).

Collected 26 atomic facts. Identity: `catalog_exact_model_variant`.

## e12: Audi RS4

The 2008 US release and brochure anchor the B7 saloon. The fuller European 2006 dimensional drawing, tire fitment and unladen mass are marked as adjacent-year/market evidence.

Sources: [Audi of America 2008 full-linebrochure](https://xr793.com/wp-content/uploads/2019/08/2008-Audi-Ful-Line.pdf); [Audi 2007 RS 4 technicalspecifications](https://www.audiworld.com/model/rs4/07/07AudiRS4_TechSpecs.pdf); [Audi of America 2008 RS 4 model announcement June 6,2007](https://www.audiworld.com/model/rs4/08/RS4_Cab_PR.pdf); [Audi RS 4 European 2006 salesbrochure (misnamed Owners Manualbyhost)](https://id.scribd.com/document/1027717492/Audi-Rs4-Owners-Manual).

Collected 28 atomic facts. Identity: `catalog_exact_model_variant`.

## e13: Lotus Elise 111R

The exact 2004 111R leaflet supplies Toyota engine ratings, six ratios, final drive, open differential, weight distribution and brake construction. The MC3 Elise trim mapping remains provisional even though the GT4 trim is explicit.

Sources: [Lotus The Elisein Brief LOTV 505062](https://kurumacatalog.com/wp-content/uploads/2021/01/2005_Lotus_Elise_catalog.pdf); [Lotus Elise 111 R 2004 factory technical leaflet](https://www.lotusdriversguide.com/Downloads/Brochures/Elise_111R_2004.pdf); [Lotus Announces Elise 111 R February 2004 factory release reprint](https://gglotus.org/gghotnews/body-elisenews.htm).

Collected 36 atomic facts. Identity: `GT4_exact_variant_MC3_trim_provisional`.

## e14: Mercedes-Benz SL55 AMG

The detailed factory technical source is the 2004 US SL product guide. It is useful for the MC3 reference and SL55 family, but is not exact proof for every GT4 2002 market parameter. Active Body Control must be represented separately from a passive spring-only car.

Sources: [Mercedes-Benz 2004 SL-Classproductguide](https://benzbits.com/manuals/PG/2004/SL_00.pdf).

Collected 34 atomic facts. Identity: `cross_game_family_match_market_year_differ`.

## e15: Aston Martin DB9 Coupe

The GT4 identity is the 2003 launch DB9. Launch power is retained, and the detailed 2006 owner-handbook dimensions, curb masses and ratios are explicitly prefixed so later revision data cannot silently become launch data.

Sources: [Aston Martin DB 9 companylaunchpressrelease(reprint)](https://www.ultimatecarpage.com/car/1719/Aston-Martin-DB9.html); [Aston Martin DB 9 historicmodel 2003–2012](https://media.astonmartin.com/db9/?lang=eng); [Aston Martin 2006 DB 9 ownerhandbook chapter 13](https://manualzz.com/doc/68803384/aston-martin-2006-db9-owner-manual).

Collected 30 atomic facts. Identity: `launch_year_match_production_reference_2004`.

## e16: Jaguar XJ220

The manufacturer retrospective supplies the production engine, drive layout, clutch, rear tire and unqualified mass. The December 1992 firsthand drive concerns pre-production cars labelled 1993; its dimensions and curb weight are separately prefixed. Exact factory production width and height remain unknown.

Sources: [Jaguar 20 yearssincelaunch XJ 220](https://media.jlr.com/archive-jaguar/en-us/news/2012/01/marking-20-years-launch-jaguar-xj220); [Ray Hutton, XJ 220 first drive, Car and Driver December 1992](https://www.caranddriver.com/reviews/a62243625/1993-jaguar-xj220-archive-drive/).

Collected 21 atomic facts. Identity: `GT4_exact_year_Redline_family_match`.

## e17: Pagani Zonda C12S 7.3

The factory Zonda S 7.3 technical sheet provides geometry, dry mass, engine, suspension, brakes and tire fitments. The rehost filename carries a later year, so publication dating remains uncertain. Zonda F specifications are excluded.

Sources: [Pagani Zonda S 7.3 bilingualtechnicalsheet](https://www.allcarcentral.com/Pagani_pdf/Pagani_Zonda_S_7.3_Tech_IT-GB_2009.pdf).

Collected 26 atomic facts. Identity: `catalog_exact_model_variant`.

## Highest-leverage follow-up questions

- Which assets have wheelbase or track errors relative to the matching real geometry, and how much do those errors alter load transfer?
- Do axle distribution, tire widths and drivetrain behavior produce the expected rear-engine, front-engine and mid-engine distinctions?
- Are short braking distances driven by plausible tire grip and brake balance, or by unrealistically low body motion or omitted load sensitivity?
- Are power, torque, rotational speed, gear ratio and final-drive units consistent across source games?
- Can adjacent-year and market variants be resolved from native game text before making them exact tuning targets?
- Which apparent performance mismatches disappear once dry/curb/EU masses and 0–60 mph/0–100 km/h test definitions are compared correctly?
