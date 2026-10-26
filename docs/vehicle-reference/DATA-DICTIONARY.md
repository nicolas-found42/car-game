# Vehicle reference data contract

This is a reference corpus, not a set of drop-in physics presets. Three evidence layers remain separate:

1. **Real-life facts** identify a year, market, trim, body and transmission where the source supports them. Each fact cites its own source. Original units, mass conventions, test conditions and uncertainty belong to the fact; an unqualified number must not override its note.
2. **Native game data** describes the recovered build/configuration. Static database values, author settings, raw binary words and catalog ratings are different kinds of evidence. None is an observed driving result unless explicitly labeled that way. No game driving measurements were performed for this corpus.
3. **Exported geometry** measures the current archived GLB's default scene. This is an art/assembly measurement, potentially reconstructed by an exporter. It is neither a manufacturer specification nor proof of the original game's runtime geometry.

## Files and grain

- `real-profiles.json`: exactly 50 real-car reference identities. `facts` use original field names, values, units, source IDs and qualifications. Sources are attached per car to preserve applicability.
- `game-profiles.json`: one record per selected game asset, including its complete catalog record, model hash, origin receipt, geometry, native data and limits. Cross-game counterparts are separate records.
- `corpus.sqlite`: normalized index of the same evidence. `car` → `real_fact` → `source`; `car` → `counterpart` → `game_fact` and `model_measurement`; `car` → `unknown`.
- CSV files mirror each SQL table. JSON-valued cells preserve ranges/vectors/lists; they are not malformed numbers. `counterpart.original_json` is the complete game profile, and `real_fact.original_json` preserves source-specific metadata.
- `profiles/<id>.md`: readable per-car real and game profiles. For Midnight Club, the complete customization/tuning offsets are in JSON/SQL rather than an unreadable giant table.
- `selection.json`: asset-code selection used by the read-only extractor.
- `research/*.json` and accompanying Markdown: reviewed research inputs, identity reasoning, source qualifications and unresolved items.
- `evidence/gt4-column-map.json`: typed decoder column map pinned to the source revision. Field names are reverse-engineering interpretations, not a blanket guarantee of physical units.

`selected_reference` in the counterpart table means the primary chosen model for this profile, **not independently verified equality of every trim/market detail**. `related_counterpart_review_year_trim_market` means shared vehicle lineage or a plausible counterpart whose year, market, trim, body or role must be reviewed before using it for a controlled cross-game comparison. Read `identity_status` and `match_note`. No sibling model automatically inherits real-world facts.

## Units, measurements and missingness

- Millimetres and metres are explicit. Track is wheel-center spacing; tire outer width, body width and visual wheel spacing are not interchangeable. Model `track_at_negative_z_m` and `track_at_positive_z_m` deliberately avoid assuming every game uses the same forward direction.
- Mass may be dry, DIN unladen, EU unladen (often includes 75 kg), US curb, homologated minimum or measured test weight. Preserve the basis. Payload/GVWR is not curb mass. Race minimum is not proof of the modeled car's actual fuel/driver load.
- `PS`, `CV`, mechanical `hp`/`bhp`, SAE gross, SAE net and `kW` are not synonyms. When conversion is needed: metric PS × 0.73549875 = kW; mechanical hp × 0.745699871582 = kW. Conversion does not repair a mismatched test standard.
- `kgf m`/`kgm` torque × 9.80665 = N m; lb ft × 1.3558179483314 = N m. Rotary displacement preserves chamber/rotor convention.
- 0–60 mph and 0–100 km/h are distinct tests; 60 mph = 96.56064 km/h. Rollout, tires, surface, launch method, weather, loading and transmission can dominate small differences. Manufacturer claims and independent tests remain labeled separately.
- Transmission ratios may include an intermediate reduction as well as the axle final drive. For example, do not discard the S2000 primary reduction. AWD center split, differential locking and intervention logic are separate variables.
- Unknown fields stay absent or explicitly listed under `unknowns`; no zero filling, synthetic spring rates, inferred drag coefficient, generic tire friction coefficient, or guessed CG height/inertia tensor.
- Multiple facts can describe alternatives or conflict. They remain multiple source-qualified facts. A consumer must choose a compatible configuration before ranking or simulation.

## Gran Turismo 4

The local build uses the `GT4_EU2560` database and British display strings. The extractor joins model code → exact car label → `GENERIC_CAR.DefaultParts` → table-ID/row-ID pairs. It follows tire references into size/compound/force rows. Raw row bytes and source file hashes remain available. Factory-default parts are the scope; a modified player save, mileage/oil/wear state or runtime tuning setup is not inferred.

The read-only decoder's typed maps come from [Nenkai/PDTools](https://github.com/Nenkai/PDTools), with the exact revision in the map. Explanatory mappings are described by its author in the [SpecDB documentation](https://nenkai.github.io/gt-modding-hub/concepts/specdb/). Native game bytes are the authority for the values; community decoder names/scales are interpretations. Unmapped rows stay hex rather than receive invented semantics.

Useful interpretation rules, subject to that distinction:

| Native field | Interpretation / limit |
|---|---|
| `CHASSIS.length`, `height`, `wheelbase`, `mass` | Decoder's chassis dimensions in mm and mass in kg; display counterparts have `d` prefixes. |
| `CHASSIS.percentageF` | Game front static weight percentage; not an independently measured manufacturer distribution. |
| `CHASSIS.performanceF/R`, `yaw`, stiffness bytes | Game handling modifiers. They are not direct tire friction, inertia or N/m values. |
| `CHASSIS.treadF/R`, `width/dwidth` | Preserve raw names/values. Local exporter uses some of these slots as outer wheel span and tire-position-related readings; do not promote them to real track/body width. |
| `ENGINE.psvalue` | Base engine display PS value. Not necessarily installed engine output after modifiers or the live garage display. |
| `ENGINE.torquevalue`, `torqueA..X` | Encoded kgf m ×100; torque curve points pair with `rpmA..X` (encoded rpm/100). |
| `ENGINE.psrpm`, `torquerpm` | Text references resolved through `UnistrDB.sdb`; ranges remain text. |
| `ENGINE.torquepoint` | Number of active curve points; do not treat unused zero slots as measured points. |
| `ENGINE.revlimit`, `shiftlimit`, `rpmA..X` | Encoded rpm/100 according to the decoder documentation. |
| `GEAR.Gear1st..`, `finalgearDF` | Encoded ratio ×1000 (3595 →3.595); retain raw values for validation. Other gearing fields can be limits or interface settings. |
| `TIRESIZE` | The existing exporter interprets aspect/5, width/5 mm, rim inches ×10 for ordinary cars. This convention is separate from visual tire mesh size. |
| `SUSPENSION` spring/damping/stabilizer fields | Native encodings. Conversion to physical force requires the game's formula; no universal N/m conversion is asserted here. |
| `BRAKE.BraketorqueF/R` | Native encoded parameters, not proven brake torque in N m. |
| `DRIVETRAIN.drivetype` | Raw enum retained. The author's published enum is based on later GT; no silent cross-version AWD algorithm assumption. |

GT4 `DEFAULT_PARAM` rows include event/AI-specific configurations. They are **not** joined by a substring match to the car name or automatically substituted for its default part settings.

## Gran Turismo (PS1)

The catalog's `nativeMetadata.specifications` preserves exact native SPEC-code base values and provenance hashes. `power_hp_game` uses the recorded display conversion; it is not an independent real-world dyno number. `displacement_rotary_chamber_cc` and factor remain separate. Year and manufacturer can be unresolved even if the model/grade name is known. Tuning and racing-family inheritance are not inferred.

## Redline

Every active `.car` assignment is retained with its array index and logical source line. Commented settings are excluded; repeated active assignments remain ordered. Scalar fields can include mass, inertia, mass center, air resistance/lift, power/torque and RPM, clutch, differential, gear ratios, damping/sway bars; wheel arrays include location, radius, width, powered fraction, steering angle, travel and brake settings.

The author may intend SI-like values (for example power values consistent with watts), but names alone do not verify execution formulas or units. These are **game author settings**, including community add-ons, and must not be labeled manufacturer measurements. Package-local asset overrides and load precedence may affect runtime behavior. The parser captures the selected authored configuration, not an executed simulation.

## Midnight Club 3: DUB Edition Remix

The corpus preserves all exact **unsuffixed** `CarCfg` matches for selected source codes. Duplicate archive entries remain separate. Values describe default-named customization configurations; tire width/profile and upgrade values may be indices. Full player engine/grip/drivetrain tuning was not recovered by this extraction. No generic top speed, power, mass or tire friction has been substituted from another game or a web wiki.

## Midnight Club: Los Angeles

The native package's `.xct` resource is decompressed and decoded using the archive's existing bounded reader. Seventeen sections include `vehSim`, `vehGrip`, `vehInput`, gyro and ability settings, with base/mods sections separate. Fields are typed words keyed by offset, not recovered physical parameter names. NaN padding is preserved as a hexadecimal string. These fields can support later executable-tracing work, but unknown floats are not tuning constants with guessed units. Ambient (`va_`) and player (`vp_`) namespaces remain distinct.

## Ford Racing 2

Catalog values and normalized ratings are retained verbatim alongside model geometry and origin receipts. Ratings such as `agility`, `accel`, `speed` and `weight` are dimensionless game UI values. Catalog top-speed units and runtime consumer formulas need independent confirmation; a numeric `topSpeed` is not silently labeled km/h or mph. Source model names can omit important trim details (e.g. Cobra versus Cobra R).
