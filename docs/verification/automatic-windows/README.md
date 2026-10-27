# Automatic diagnostic windows

Recover bounded nonfatal gear-hunting, sustained contact-loss, zero-drive-demand
and real/virtual-clock-gap windows from checkpoint `d75f950`. Every vehicle is
observed after completed-step recording through the current production pipeline.
Monitor state is separate per actor so simultaneous cars cannot invent or hide
symptoms; triggering events and incident metadata retain that actor identity.

At most8 windows retain15seconds before and2seconds after a trigger, bounded by
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
