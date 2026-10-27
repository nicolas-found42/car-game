# Automatic diagnostic windows

Recover bounded nonfatal gear-hunting, sustained contact-loss, zero-drive-demand
and real/virtual-clock-gap windows from checkpoint `d75f950`. Every vehicle is
observed after completed-step recording through the current production pipeline.
Monitor state is separate per actor so simultaneous cars cannot invent or hide
symptoms; triggering events and incident metadata retain that actor identity.

At most 8 windows retain 15 seconds before and 2 seconds after a trigger, bounded by
the existing recorder history/queue budgets. Exports run on the existing writer,
keep the fatal-incident slot separate and report shortened/lost/incomplete
coverage honestly. Export failure cannot remain a healthy recording. These are
observed symptoms, not automatic proof of a physics defect or retuning advice.
Default-mode contact evidence comes from completed-step queries; reference-mode
shift/contact decisions add the explicit model explanations where available.

The existing fatal recorder, finalization deadlines and source/provenance
contracts remain in place. Production physics, controls, geometry and acceptance
thresholds are unchanged. The JSON diagnosis CLI can read both raw runs and
these frozen windows without requiring the complete session traces.

The full headless ladder passes 107 Rust tests, including eight diagnostic-window
controls and a real/virtual clock observer. A partial first-post-tick eviction
control failed before the coverage fix and passes after. Controlled gear hunting
exports a window; two independent same-model actors do not invent a shift fault.
Failed export and incomplete/lost history cannot claim healthy, complete evidence.

Recorded binary `0660d762234793bb30b9684870be9047555414eb0e1377e72c02f0890f8f51d3`
first encountered a host frame stall during fleet braking; the run failed and
exported a clock-gap window. `clock-gap-control.json` demonstrates three actual
1.1–2.1 second gaps through the frozen-window diagnosis CLI. That failure is
retained, not reported as acceptance. After local compilation finished, the
unchanged binary passed all 13 fleet cars and 52 captures, both fleet/run
verifiers, and a native bounded driving scenario. The native capture was visually
inspected. Results and original evidence paths are in `results.json`.

Reviewed with sol-loop: three rounds with gpt-6.1-sol on medium, approved after
fixing post-trigger coverage and completed-observation boundary labels.
