use serde::{Deserialize, Serialize};
use std::process::Command;
use std::path::Path;
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct SslEvent {
    pub pid: u32,
    pub comm: String,
    pub fd: i32,
    pub timestamp: u64,
    pub event_type: String,
    pub data: String,
    pub data_len: usize,
}

impl fmt::Display for SslEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let truncated_data = if self.data.len() > 100 {
            format!("{}...", &self.data[..100])
        } else {
            self.data.clone()
        };
        
        write!(
            f,
            "🔐 SSL Event: {} [PID: {}] [FD: {}]\n   📊 Type: {}\n   ⏰ Timestamp: {}\n   📦 Data ({} bytes): {}",
            self.comm, self.pid, self.fd, self.event_type, self.timestamp, self.data_len, truncated_data
        )
    }
}

pub struct SslSniffCollector {
    binary_path: String,
}

impl SslSniffCollector {
    pub fn new(binary_path: impl AsRef<Path>) -> Self {
        Self {
            binary_path: binary_path.as_ref().to_string_lossy().to_string(),
        }
    }

    pub async fn collect_events(&self) -> Result<Vec<SslEvent>, Box<dyn std::error::Error>> {
        let output = Command::new(&self.binary_path)
            .output()
            .expect("Failed to execute sslsniff binary");

        if !output.status.success() {
            return Err(format!("SSLSniff binary failed with status: {}", output.status).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut events = Vec::new();

        for line in stdout.lines() {
            if let Ok(event) = serde_json::from_str::<SslEvent>(line) {
                events.push(event);
            }
        }

        Ok(events)
    }
}