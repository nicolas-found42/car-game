# Searchable recorded-run diagnosis

Recovered the standalone JSON interface from checkpoint `d75f950`. It searches
explicit run roots, filters actor/tick windows, identifies shift hunting and
contact/clock anomalies, and compares recorded samples. It reports existing
recorded facts and inferences separately. It does not replay a simulation or
replace `tools/verify_run.py` as the integrity acceptance entry point.

This recovery slice adds the analysis interface only. New automatic incident
recording and richer force/shift decision production remain a separate runtime
recovery slice; old traces cannot gain missing decisions by analysis.

Run `python3 tools/diagnose.py --help` for the available subcommands. Focused
controls are `python3 tools/test_diagnosis.py`. Historical failures remain in
their original bundles; diagnosis reads them without mutation.
