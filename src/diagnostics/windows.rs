//! One asynchronous automatic export at a time; the fatal incident slot stays separate.
use super::{RunRecorder, history::HistorySnapshot};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;

const PRE_TICKS: u64 = 1800;
const POST_TICKS: u64 = 240;
const MAX_WINDOWS: u64 = 8;

pub(super) struct Window {
    pub number: u64,
    pub reason: &'static str,
    pub tick: u64,
    pub car: String,
    pub actor_id: Option<String>,
}
#[derive(Default)]
pub(super) struct Windows {
    active: Option<Window>,
    count: u64,
    suppressed: u64,
    last_end: Option<u64>,
}
impl RunRecorder {
    pub(super) fn trigger_window(
        &self,
        reason: &'static str,
        tick: u64,
        car: &str,
        actor: Option<&str>,
    ) {
        let mut state = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if state.active.is_some()
            || state.count >= MAX_WINDOWS
            || state
                .last_end
                .is_some_and(|end| tick < end.saturating_add(360))
        {
            state.suppressed += 1;
            return;
        }
        state.count += 1;
        state.active = Some(Window {
            number: state.count,
            reason,
            tick,
            car: car.into(),
            actor_id: actor.map(str::to_owned),
        });
        drop(state);
        let mut envelope = self.envelope();
        envelope.tick = tick;
        envelope.actor_id = actor.map(str::to_owned);
        self.event_envelope("diagnostic.incident_triggered", json!({"reason":reason,"trigger_tick":tick,"car":car,"pre_ticks":PRE_TICKS,"post_ticks":POST_TICKS}), envelope);
    }
    pub(super) fn poll_window(&self, tick: u64, finishing: bool) {
        let mut state = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        if state
            .active
            .as_ref()
            .is_none_or(|w| !finishing && tick < w.tick.saturating_add(POST_TICKS))
        {
            return;
        }
        if self.health.window_pending.load(Ordering::Acquire) {
            return;
        }
        let window = state.active.take().expect("active window");
        state.last_end = Some(tick);
        drop(state);
        self.recording.window(window, tick);
    }
    pub(super) fn window_status(&self) -> Value {
        let state = self.windows.lock().unwrap_or_else(|e| e.into_inner());
        json!({"schema":1,"triggered":state.count,"suppressed":state.suppressed,"active":state.active.is_some(),
            "exported":self.health.update(|d|d.window_exports),
            "max_windows":MAX_WINDOWS,"pre_ticks":PRE_TICKS,"post_ticks":POST_TICKS})
    }
}

pub(super) fn publish(
    health: &super::health::HealthState,
    window: Window,
    end: u64,
    mut snapshot: HistorySnapshot,
) {
    let start = std::time::Instant::now();
    let first = window.tick.saturating_sub(PRE_TICKS);
    snapshot
        .records
        .retain(|r| r.tick >= first && r.tick <= end.min(window.tick.saturating_add(POST_TICKS)));
    let observed_first = snapshot.records.iter().map(|r| r.tick).min();
    let evidence_healthy = !health.failed();
    let requested_last = window.tick.saturating_add(POST_TICKS);
    let post_retained = snapshot.status["shortened"] == false
        && observed_first.is_some_and(|first| first <= window.tick.saturating_add(1))
        && snapshot
            .records
            .iter()
            .any(|record| record.tick == requested_last);
    let coverage = json!({"first_tick":observed_first,
        "last_tick":snapshot.records.iter().map(|r|r.tick).max(),"history":snapshot.status,
        "requested_first_tick":first,"requested_last_tick":window.tick.saturating_add(POST_TICKS),
        "recorder_healthy_at_export":evidence_healthy,
        "pre_complete":evidence_healthy && observed_first.is_some_and(|tick|tick<=first) && snapshot.status["shortened"]==false,
        "post_complete":evidence_healthy && end>=requested_last && post_retained});
    let dir = health
        .dir
        .join("incidents")
        .join(format!("{:04}", window.number));
    if let Err(error) = super::incident::write_snapshot(&dir, &health.dir, &snapshot)
        .and_then(|()| super::write_json_atomic(&dir.join("incident.json"), &json!({"schema":1,"reason":window.reason,
            "trigger_tick":window.tick,"car":window.car,"actor_id":window.actor_id,"coverage":coverage,"records":snapshot.records.len(),
            "behavioral_failure":false,"path":"incident-history.jsonl"}))) {
        health.error("window_export", "history", error.kind());
    } else {
        health.update(|d|d.window_exports+=1);
    }
    health.observe("automatic_window_export", start);
    health.window_pending.store(false, Ordering::Release);
}
