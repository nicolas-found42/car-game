# Preservation checkpoint recovery

The immutable `codex/preserve-fleet-progress-20261026` checkpoint (`d75f950`)
contains unique work from before the independent-control and synchronized-physics
integration. It is a recovery source, not a merge candidate. `inventory.json`
assigns every one of its 1,068 changed paths to a focused recovery slice and pins
the exact checkpoint bytes. Shared glue files need hunk-level separation.

The first slice recovers the vehicle-reference corpus only. Its original research
qualifications and manufacturer/game distinctions remain authoritative. Structural
verification rechecked all 50 references, 80 counterpart assets, 280 source file
hashes, 99,166 exported native rows/values and 1,555 cited facts. It does not
independently certify the truth of every historical external claim.

Remaining slices separate asset preparation/catalog data, diagnostic recording
and provenance, scenario courses/fast runner, vehicle profiles/force integration,
and study tooling. Large historical result matrices remain recoverable at the
checkpoint; their old passing runs do not certify current-main integration.
Do not restore whole old runtime files over the newer per-vehicle controls,
assembly retention, synchronized query/force schedule or completed-step records.

Recovery command for any original file:

```sh
git show codex/preserve-fleet-progress-20261026:path/from/inventory
```
