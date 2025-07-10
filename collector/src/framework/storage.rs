use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use crate::framework::{ObservabilityEvent, EventSource, EventType};

#[derive(Clone)]
pub struct InMemoryStorage {
    events: Arc<RwLock<VecDeque<ObservabilityEvent>>>,
    indices: Arc<RwLock<StorageIndices>>,
    max_events: usize,
}

#[derive(Default)]
struct StorageIndices {
    by_source: HashMap<String, Vec<usize>>,
    by_type: HashMap<String, Vec<usize>>,
    by_pid: HashMap<u32, Vec<usize>>,
    by_timestamp: Vec<(u64, usize)>, // (timestamp, index)
}

impl InMemoryStorage {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::new())),
            indices: Arc::new(RwLock::new(StorageIndices::default())),
            max_events,
        }
    }

    pub fn store_event(&self, event: ObservabilityEvent) -> Result<(), Box<dyn std::error::Error>> {
        let mut events = self.events.write().unwrap();
        let mut indices = self.indices.write().unwrap();

        // Add event
        events.push_back(event.clone());
        let event_index = events.len() - 1;

        // Maintain max size
        if events.len() > self.max_events {
            events.pop_front();
            // Rebuild indices when we remove events (simplified approach)
            self.rebuild_indices(&events, &mut indices);
        } else {
            // Update indices
            self.update_indices(&event, event_index, &mut indices);
        }

        Ok(())
    }

    pub fn get_all_events(&self) -> Vec<ObservabilityEvent> {
        self.events.read().unwrap().iter().cloned().collect()
    }

    pub fn get_events_by_source(&self, source: &EventSource) -> Vec<ObservabilityEvent> {
        let events = self.events.read().unwrap();
        let indices = self.indices.read().unwrap();
        
        let source_key = format!("{:?}", source);
        if let Some(event_indices) = indices.by_source.get(&source_key) {
            event_indices.iter()
                .filter_map(|&i| events.get(i))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_events_by_type(&self, event_type: &EventType) -> Vec<ObservabilityEvent> {
        let events = self.events.read().unwrap();
        let indices = self.indices.read().unwrap();
        
        let type_key = format!("{:?}", event_type);
        if let Some(event_indices) = indices.by_type.get(&type_key) {
            event_indices.iter()
                .filter_map(|&i| events.get(i))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_events_by_pid(&self, pid: u32) -> Vec<ObservabilityEvent> {
        let events = self.events.read().unwrap();
        let indices = self.indices.read().unwrap();
        
        if let Some(event_indices) = indices.by_pid.get(&pid) {
            event_indices.iter()
                .filter_map(|&i| events.get(i))
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_events_in_time_range(&self, start: u64, end: u64) -> Vec<ObservabilityEvent> {
        let events = self.events.read().unwrap();
        let indices = self.indices.read().unwrap();
        
        indices.by_timestamp.iter()
            .filter(|(timestamp, _)| *timestamp >= start && *timestamp <= end)
            .filter_map(|(_, index)| events.get(*index))
            .cloned()
            .collect()
    }

    pub fn get_recent_events(&self, count: usize) -> Vec<ObservabilityEvent> {
        let events = self.events.read().unwrap();
        events.iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    pub fn get_event_count(&self) -> usize {
        self.events.read().unwrap().len()
    }

    pub fn get_stats(&self) -> StorageStats {
        let events = self.events.read().unwrap();
        let indices = self.indices.read().unwrap();
        
        StorageStats {
            total_events: events.len(),
            sources: indices.by_source.keys().cloned().collect(),
            types: indices.by_type.keys().cloned().collect(),
            pids: indices.by_pid.keys().cloned().collect(),
            oldest_timestamp: events.front().map(|e| e.timestamp),
            newest_timestamp: events.back().map(|e| e.timestamp),
        }
    }

    fn update_indices(&self, event: &ObservabilityEvent, index: usize, indices: &mut StorageIndices) {
        // Index by source
        let source_key = format!("{:?}", event.source);
        indices.by_source.entry(source_key).or_default().push(index);

        // Index by type
        let type_key = format!("{:?}", event.event_type);
        indices.by_type.entry(type_key).or_default().push(index);

        // Index by PID (if applicable)
        match &event.data {
            crate::framework::EventData::Process { pid, .. } => {
                indices.by_pid.entry(*pid).or_default().push(index);
            }
            crate::framework::EventData::SSL { pid, .. } => {
                indices.by_pid.entry(*pid).or_default().push(index);
            }
            _ => {}
        }

        // Index by timestamp
        indices.by_timestamp.push((event.timestamp, index));
        indices.by_timestamp.sort_by_key(|(timestamp, _)| *timestamp);
    }

    fn rebuild_indices(&self, events: &VecDeque<ObservabilityEvent>, indices: &mut StorageIndices) {
        indices.by_source.clear();
        indices.by_type.clear();
        indices.by_pid.clear();
        indices.by_timestamp.clear();

        for (index, event) in events.iter().enumerate() {
            self.update_indices(event, index, indices);
        }
    }
}

#[derive(Debug)]
pub struct StorageStats {
    pub total_events: usize,
    pub sources: Vec<String>,
    pub types: Vec<String>,
    pub pids: Vec<u32>,
    pub oldest_timestamp: Option<u64>,
    pub newest_timestamp: Option<u64>,
}

impl StorageStats {
    pub fn print_summary(&self) {
        println!("📊 Storage Statistics:");
        println!("   Total events: {}", self.total_events);
        println!("   Sources: {:?}", self.sources);
        println!("   Event types: {:?}", self.types);
        println!("   PIDs: {:?}", self.pids);
        if let (Some(oldest), Some(newest)) = (self.oldest_timestamp, self.newest_timestamp) {
            println!("   Time range: {} - {} (duration: {}ns)", oldest, newest, newest - oldest);
        }
    }
}