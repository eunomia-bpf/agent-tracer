use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub comm: String,
    pub filename: String,
    pub timestamp: u64,
    pub event_type: String,
}

pub struct ProcessCollector {
    binary_path: String,
}

impl ProcessCollector {
    pub fn new(binary_path: impl AsRef<Path>) -> Self {
        Self {
            binary_path: binary_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub async fn collect_events(&self) -> Result<Vec<ProcessEvent>, Box<dyn std::error::Error>> {
        let output = Command::new(&self.binary_path)
            .output()
            .expect("Failed to execute process binary");

        if !output.status.success() {
            return Err(format!("Process binary failed with status: {}", output.status).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut events = Vec::new();

        for line in stdout.lines() {
            if let Ok(event) = serde_json::from_str::<ProcessEvent>(line) {
                events.push(event);
            }
        }

        Ok(events)
    }
}