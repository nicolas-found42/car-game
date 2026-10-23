use super::{RecorderConfig, recording::Record};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

pub struct History {
    ticks: BTreeMap<u64, VecDeque<Arc<Record>>>,
    count: usize,
    bytes: usize,
    latest_tick: u64,
    window_evictions: u64,
    capacity_evictions: u64,
    last_capacity_tick: Option<u64>,
    config: RecorderConfig,
}
pub struct HistorySnapshot {
    pub records: Vec<Arc<Record>>,
    pub status: Value,
}
impl History {
    pub fn new(config: RecorderConfig) -> Self {
        Self {
            ticks: BTreeMap::new(),
            count: 0,
            bytes: 0,
            latest_tick: 0,
            window_evictions: 0,
            capacity_evictions: 0,
            last_capacity_tick: None,
            config,
        }
    }
    pub fn push(&mut self, record: Arc<Record>) {
        if record.stream == super::EvidenceStream::Log {
            return;
        }
        self.advance(record.tick);
        if record.tick < self.earliest() {
            self.window_evictions += 1;
            return;
        }
        self.bytes += record.cost();
        self.count += 1;
        self.ticks.entry(record.tick).or_default().push_back(record);
        while self.bytes > self.config.history_bytes || self.count > self.config.history_records {
            if let Some((tick, _)) = self.ticks.first_key_value() {
                self.last_capacity_tick = Some(self.last_capacity_tick.unwrap_or(*tick).max(*tick));
            }
            self.evict();
            self.capacity_evictions += 1;
        }
    }
    fn earliest(&self) -> u64 {
        self.latest_tick
            .saturating_sub(self.config.history_ticks.saturating_sub(1))
    }
    pub fn advance(&mut self, tick: u64) {
        self.latest_tick = self.latest_tick.max(tick);
        while self
            .ticks
            .first_key_value()
            .is_some_and(|(tick, _)| *tick < self.earliest())
        {
            self.evict();
            self.window_evictions += 1;
        }
    }
    fn evict(&mut self) {
        if let Some(mut entry) = self.ticks.first_entry() {
            let record = entry
                .get_mut()
                .pop_front()
                .expect("nonempty history bucket");
            self.bytes -= record.cost();
            self.count -= 1;
            if entry.get().is_empty() {
                entry.remove();
            }
        }
    }
    pub fn status(&self) -> Value {
        let first = self.ticks.first_key_value().map(|(tick, _)| *tick);
        json!({"requested_ticks":self.config.history_ticks,"retained_records":self.count,"retained_bytes":self.bytes,
            "first_tick":first,"last_tick":self.ticks.last_key_value().map(|(tick,_)|*tick),"window_evictions":self.window_evictions,
            "capacity_evictions":self.capacity_evictions,"shortened":self.last_capacity_tick.is_some_and(|tick|tick>=self.earliest())})
    }
    pub fn recent_events(&self) -> Vec<Value> {
        let mut records: Vec<_> = self
            .ticks
            .values()
            .flatten()
            .filter(|record| record.stream == super::EvidenceStream::Events)
            .collect();
        records.sort_unstable_by_key(|record| std::cmp::Reverse(record.seq));
        let mut bytes = 0;
        let mut values = Vec::new();
        for record in records.into_iter().take(200) {
            if bytes + record.bytes.len() > 256 * 1024 {
                break;
            }
            if let Ok(value) = serde_json::from_slice::<Value>(&record.bytes) {
                bytes += record.bytes.len();
                values.push(value);
            }
        }
        values.reverse();
        values
    }
    pub fn snapshot(&self) -> HistorySnapshot {
        let mut records: Vec<_> = self.ticks.values().flatten().cloned().collect();
        records.sort_unstable_by_key(|record| record.seq);
        HistorySnapshot {
            records,
            status: self.status(),
        }
    }
}
