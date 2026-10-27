# Staged prepared fleet catalogue

Recovered 287 prepared editions and their source-preservation receipts from
checkpoint `d75f950`. `staged-fleet/catalog.json` retains their authored
metadata independently of the active 13-car roster. Prepared files live outside
the runtime asset directory, so they neither expand asset loading nor create
unknown-asset omissions in existing run provenance. This slice does not enable
new selectable cars or apply checkpoint-era vehicle dynamics.

`python3 tools/check_asset_catalog.py` checks every staged edition through the
existing source asset verifier, including original vertex/index/image bytes,
wheel ownership and transforms. The C5 receipt explicitly declares its affine
reference-envelope correction; the source shell remains an approximation.
The preparation refactor retains the original material roles and adds an
explicit destination argument so preparation can target staging directories.

Static integrity is not driving, native appearance or stock-physics acceptance.
The separate vehicle-profile/runtime recovery must satisfy those gates before
promoting these editions into the active roster. Original research/results and
reproduction tools remain available on the immutable preservation branch.
