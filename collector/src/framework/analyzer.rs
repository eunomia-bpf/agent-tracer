use async_trait::async_trait;
use tokio_stream::StreamExt;
use std::collections::HashMap;
use crate::framework::{ObservabilityEvent, EventStream, EventSource, EventType};

#[async_trait]
pub trait Analyzer: Send + Sync {
    async fn analyze(&mut self, mut stream: EventStream) -> Result<(), Box<dyn std::error::Error>>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

pub struct AggregatorAnalyzer {
    stats: HashMap<String, u64>,
    event_count: u64,
}

impl AggregatorAnalyzer {
    pub fn new() -> Self {
        Self {
            stats: HashMap::new(),
            event_count: 0,
        }
    }

    pub fn get_stats(&self) -> &HashMap<String, u64> {
        &self.stats
    }

    pub fn get_event_count(&self) -> u64 {
        self.event_count
    }

    fn update_stats(&mut self, event: &ObservabilityEvent) {
        self.event_count += 1;
        
        // Count by source
        let source_key = format!("source_{:?}", event.source);
        *self.stats.entry(source_key).or_insert(0) += 1;
        
        // Count by event type
        let type_key = format!("type_{:?}", event.event_type);
        *self.stats.entry(type_key).or_insert(0) += 1;
        
        // Source-specific stats
        match &event.source {
            EventSource::Process => {
                if let crate::framework::EventData::Process { pid, .. } = &event.data {
                    let pid_key = format!("process_pid_{}", pid);
                    *self.stats.entry(pid_key).or_insert(0) += 1;
                }
            }
            EventSource::SSL => {
                if let crate::framework::EventData::SSL { data_len, .. } = &event.data {
                    let bytes_key = "ssl_total_bytes".to_string();
                    *self.stats.entry(bytes_key).or_insert(0) += *data_len as u64;
                }
            }
            _ => {}
        }
    }
}

#[async_trait]
impl Analyzer for AggregatorAnalyzer {
    async fn analyze(&mut self, mut stream: EventStream) -> Result<(), Box<dyn std::error::Error>> {
        println!("📊 Starting aggregator analyzer");
        
        while let Some(event) = stream.next().await {
            println!("📈 Analyzing event: {} from {:?}", event.id, event.source);
            self.update_stats(&event);
            
            // Print periodic stats
            if self.event_count % 10 == 0 {
                println!("📊 Current stats: {} events processed", self.event_count);
                for (key, value) in &self.stats {
                    println!("   {}: {}", key, value);
                }
            }
        }
        
        println!("✅ Aggregator analyzer completed");
        Ok(())
    }

    fn name(&self) -> &str {
        "aggregator"
    }

    fn description(&self) -> &str {
        "Event aggregation and statistics analyzer"
    }
}

pub struct FilterAnalyzer {
    filter_fn: Box<dyn Fn(&ObservabilityEvent) -> bool + Send + Sync>,
    filtered_count: u64,
    total_count: u64,
}

impl FilterAnalyzer {
    pub fn new<F>(filter_fn: F) -> Self 
    where 
        F: Fn(&ObservabilityEvent) -> bool + Send + Sync + 'static
    {
        Self {
            filter_fn: Box::new(filter_fn),
            filtered_count: 0,
            total_count: 0,
        }
    }

    pub fn ssl_events_only() -> Self {
        Self::new(|event| matches!(event.source, EventSource::SSL))
    }

    pub fn process_events_only() -> Self {
        Self::new(|event| matches!(event.source, EventSource::Process))
    }

    pub fn get_stats(&self) -> (u64, u64) {
        (self.filtered_count, self.total_count)
    }
}

#[async_trait]
impl Analyzer for FilterAnalyzer {
    async fn analyze(&mut self, mut stream: EventStream) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Starting filter analyzer");
        
        while let Some(event) = stream.next().await {
            self.total_count += 1;
            
            if (self.filter_fn)(&event) {
                self.filtered_count += 1;
                println!("✅ Event passed filter: {} from {:?}", event.id, event.source);
            } else {
                println!("❌ Event filtered out: {} from {:?}", event.id, event.source);
            }
        }
        
        println!("✅ Filter analyzer completed: {}/{} events passed", 
                 self.filtered_count, self.total_count);
        Ok(())
    }

    fn name(&self) -> &str {
        "filter"
    }

    fn description(&self) -> &str {
        "Event filtering analyzer"
    }
}

pub struct TimelineAnalyzer {
    events: Vec<ObservabilityEvent>,
    max_events: usize,
}

impl TimelineAnalyzer {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
        }
    }

    pub fn get_events(&self) -> &[ObservabilityEvent] {
        &self.events
    }

    pub fn get_events_by_source(&self, source: &EventSource) -> Vec<&ObservabilityEvent> {
        self.events.iter()
            .filter(|event| std::mem::discriminant(&event.source) == std::mem::discriminant(source))
            .collect()
    }

    pub fn get_events_in_time_range(&self, start: u64, end: u64) -> Vec<&ObservabilityEvent> {
        self.events.iter()
            .filter(|event| event.timestamp >= start && event.timestamp <= end)
            .collect()
    }
}

#[async_trait]
impl Analyzer for TimelineAnalyzer {
    async fn analyze(&mut self, mut stream: EventStream) -> Result<(), Box<dyn std::error::Error>> {
        println!("📅 Starting timeline analyzer (max {} events)", self.max_events);
        
        while let Some(event) = stream.next().await {
            println!("📝 Recording event: {} at {}", event.id, event.timestamp);
            
            self.events.push(event);
            
            // Keep only the most recent events
            if self.events.len() > self.max_events {
                self.events.remove(0);
            }
        }
        
        println!("✅ Timeline analyzer completed with {} events", self.events.len());
        Ok(())
    }

    fn name(&self) -> &str {
        "timeline"
    }

    fn description(&self) -> &str {
        "Event timeline recording analyzer"
    }
}