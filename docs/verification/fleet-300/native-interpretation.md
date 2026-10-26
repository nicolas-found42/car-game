# GT4 native specification interpretation

Inspected 2026-10-25 against the archived **GT4_EU2560** tables. These are source-game values, not manufacturer measurements or recovered global physics. No runtime files changed.

## Verified structure and units

The local collector follows `GENERIC_CAR → DEFAULT_PARTS → numeric (row_id, table_id)` keys. It checks decoded row lengths against the pinned column map. The upstream PDTools maps establish field order and storage types, not physical units: `psvalue`/`torquevalue` are signed shorts; `RedLine`, `shiftlimit` and `revlimit` are separate bytes; gearing includes an explicit extra-final field and usage byte. [Collector](../../vehicle-reference/scripts/collect_game_profiles.py), [pinned local column map](../../vehicle-reference/evidence/gt4-column-map.json), [upstream Engine.cs](https://github.com/Nenkai/PDTools/blob/2a9637d8c36100e4c211d73cd8140a9d690578d2/PDTools.SpecDB/Core/Mapping/Tables/Engine.cs), [Gear.cs](https://github.com/Nenkai/PDTools/blob/2a9637d8c36100e4c211d73cd8140a9d690578d2/PDTools.SpecDB/Core/Mapping/Tables/Gear.cs), [Drivetrain.cs](https://github.com/Nenkai/PDTools/blob/2a9637d8c36100e4c211d73cd8140a9d690578d2/PDTools.SpecDB/Core/Mapping/Tables/Drivetrain.cs).

The decoder author's documentation distinguishes display peaks from simulation torque points and installed modifiers. Interpret display `psvalue` as metric PS and `torquevalue / 100` as kgf·m. Convert with **0.73549875 kW/PS** and **9.80665 Nm/(kgf·m)**. RPM bytes use a factor of 100; `RedLine` marks the UI threshold, whereas `revlimit` is the limiter. The explicit gearing example `3595 → 3.595` establishes division by **1000**, despite the surrounding documentation's erroneous “100” wording. Its drive enum is explicitly documented from GT7, so GT4 archive samples provide the cross-check below. These are community reverse-engineering interpretations, not publisher documentation. [Author's SpecDB guide, pinned source](https://github.com/Nenkai/gt-modding-hub/blob/d7c057f2189dc5987a78e54bfa3461135f23f8c1/docs/concepts/specdb.md).

The local `GT4.profile()` reread all **730** model entries without a row/schema failure. Representative raw values:

| Asset code / native label | `psvalue` | `torquevalue` | `RedLine` / `revlimit` | `drivetype` |
|---|---:|---:|---:|---:|
| `GT4_NISS0004` / `_350z_eu_03` | 287 | 3790 | 66 / 70 | 0 |
| `GT4_BMW_0011` / `m3_csl_03` | 360 | 3780 | 82 / 85 | 0 |
| `GT4_BUIC0001` / `buick_gnx_87` | 280 | 4977 | 56 / 60 | 0 |
| `GT4_VW__0015` / `golf_gti_05` | 200 | 2860 | 69 / 75 | 1 |
| `GT4_SUBR0002` / `impreza_wrx_pro_01` | 280 | 3800 | 80 / 85 | 2 |
| `GT4_HOND0031` / `nsx_r_02` | 280 | 3100 | 80 / 90 | 3 |
| `GT4_RUF_0005` / `ctr_87` | 469 | 5640 | 68 / 75 | 4 |

For example, 350Z raw peaks become **211.08814125 kW / 371.668735 Nm**, with a 6600 rpm UI line and 7000 rpm native limiter. They must not be silently replaced with the separate US reference's 287 mechanical hp. BMW's 360 PS and GNX's 280 PS also corroborate the metric interpretation. The displayed peaks still do not reconstruct native wheel power: NSX-R `torquevol=106` and Impreza `torquevol=110` differ from 100, before other installed modifiers. [Raw ENGINE table](/Users/nicolas/.codex/worktrees/130e/carmodels/games/gran-turismo-4/cars/provenance/specdb/GT4_EU2560/ENGINE.dbt).

## Drive layout cross-check

The 730 joined stock profiles contain **270 code 0, 168 code 1, 166 code 2, 112 code 3 and 14 code 4**. Known layout anchors above support **0 FR, 1 FF, 2 AWD, 3 MR, 4 RR**: 350Z/CSL/GNX, Golf GTI, Impreza, NSX-R and CTR respectively. A drive code supports driven axle/layout; it does not identify an AWD torque split, differential lock behavior or electronic controller. The current reference corpus separately establishes these anchor layouts. [Existing reference profiles](../../vehicle-reference/real-profiles.json), [raw DRIVETRAIN table](/Users/nicolas/.codex/worktrees/130e/carmodels/games/gran-turismo-4/cars/provenance/specdb/GT4_EU2560/DRIVETRAIN.dbt).

## Gearing exceptions and admission limits

The native 350Z default row has forward raw ratios **3827/2360/1685/1312/1000/793**, final **3545**, six shifts, `gearflag=0`, extra-final ratio/usage both zero. These decode as the native game's 3.827/2.360/1.685/1.312/1/.793 and 3.545, not the real US 3.794/.../3.538. Preserve source disagreement.

Across all 730 defaults, **603 have `gearflag=0`; 127 have `gearflag=1`**. The latter require automatic ratio generation rather than blindly treating stored values as final executed ratios. **26 defaults have nonzero extra-final data**. Examples:

| Car | Primary final raw | Extra final raw | Usage | Forward count |
|---|---:|---:|---:|---:|
| Golf IV R32 `vw__0007` | 4235 | 3272 | 2 | 6 |
| S2000 `hond0049` | 4100 | 1160 | 1 | 6 |
| Focus ST170 `fdeu0007` | 2871 | 4238 | 3 | 6 |
| Alto Works `suzk0011` | 4705 | 3363 | 4 | 5 |

The guide describes an alternate final and a usage threshold, but its simple example does not establish execution for these distinct GT4 modes. In particular, a 1.160 compound reduction cannot safely be treated as an ordinary replacement differential ratio. **Exclude nonzero extra-final/usage and automatic-generation rows from unqualified single-final imports unless their executed semantics are independently established.** Do not invent a per-gear mapping or drop the extra value. `GT4_VW__0015` Golf V GTI has zero extra fields in this build, despite being the guide's generic illustrative car. [Raw GEAR table](/Users/nicolas/.codex/worktrees/130e/carmodels/games/gran-turismo-4/cars/provenance/specdb/GT4_EU2560/GEAR.dbt).

The archive supports nominal engine display peaks, layout, dimensions, tires and simple gearing with the qualifications above. It does not establish real-world grip, brake force, CG height, inertia, effective drivetrain efficiency, launch clutch/converter behavior, nor complete GT4 runtime parity. Preserve raw values and classify conversions as game-derived inputs; shared simulation defaults remain explicit estimates.

## Reproduction pins

Use `GT4(Path('../carmodels').resolve()).profile('GT4_' + model.upper())` for every model in `.specs`; obtain fields from joined `default_part_rows`. The collector imports `gt4_specdb` from the sibling tools directory. Hashes captured during the read:

| File | SHA-256 |
|---|---|
| `ENGINE.dbt` | `ecedb8d88d651bb1bfb5e0ecde1ca05928c976567f54c58098c116f606b8ca45` |
| `GEAR.dbt` | `4df92c85439409e6fcc9bd7fa16adf8d8f1bcc3efdd31aa17a4420eb46513afb` |
| `DRIVETRAIN.dbt` | `2643ad2da0accf28ea83a4fbbf2a5a97d12c3345d88a9ad950e4cf1ce8ac07fe` |
| `DEFAULT_PARTS.dbt` | `7dd01531b197b40ef3e717d956657e9ca705016e70ec8730bd82188c909ba938` |
| `GENERIC_CAR.dbt` | `013eeb36c45cfcaad4c9e0f38e5ebb06a7d047600ed189cf31e8e939d0ea1856` |

Pinned upstream column-map commit: `2a9637d8c36100e4c211d73cd8140a9d690578d2`. The local decoder is a table decoder, not game executable disassembly; raw round-trip interpretation and field names do not independently prove all runtime behavior.

For reduced-order import, a displayed peak-RPM interval is represented by its midpoint; the complete original range remains in `native-specifications.json`. Missing peak-RPM text is not invented as a decoded value: it receives an estimated point based on the native limiter and is marked `estimate`. The model’s two-point interpolation is not the complete native torque curve or its tuning modifiers.
