use async_trait::async_trait;
use std::path::Path;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::time::Duration;
use crate::framework::{ObservabilityEvent, EventSender, EventSource, EventType, EventData};

#[async_trait]
pub trait Runner: Send + Sync {
    async fn run(&self, sender: EventSender, timeout: Option<Duration>) -> Result<(), Box<dyn std::error::Error>>;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
}

pub struct ProcessRunner {
    binary_path: String,
}

impl ProcessRunner {
    pub fn new(binary_path: impl AsRef<Path>) -> Self {
        Self {
            binary_path: binary_path.as_ref().to_string_lossy().to_string(),
        }
    }
}

#[async_trait]
impl Runner for ProcessRunner {
    async fn run(&self, sender: EventSender, timeout: Option<Duration>) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Starting process runner: {}", self.binary_path);
        
        let mut child = Command::new(&self.binary_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let mut reader = BufReader::new(stdout).lines();

        let start_time = std::time::Instant::now();
        
        loop {
            // Check timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() > timeout_duration {
                    println!("⏰ Process runner timeout reached");
                    let _ = child.kill().await;
                    break;
                }
            }

            tokio::select! {
                line = reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            println!("📤 Process output: {}", line);
                            
                            // Try to parse as JSON first
                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(event) = self.parse_process_event(&json_value) {
                                    let _ = sender.send(event);
                                }
                            } else {
                                // Raw output as custom event
                                let event = ObservabilityEvent::new(
                                    EventSource::Process,
                                    EventType::Custom("raw_output".to_string()),
                                    EventData::Custom({
                                        let mut data = std::collections::HashMap::new();
                                        data.insert("raw_line".to_string(), serde_json::Value::String(line));
                                        data
                                    })
                                );
                                let _ = sender.send(event);
                            }
                        }
                        Ok(None) => {
                            println!("✅ Process runner completed");
                            break;
                        }
                        Err(e) => {
                            println!("❌ Process runner error: {}", e);
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Continue loop
                }
            }
        }

        let _ = child.kill().await;
        Ok(())
    }

    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "Process monitoring runner"
    }
}

impl ProcessRunner {
    fn parse_process_event(&self, json: &serde_json::Value) -> Option<ObservabilityEvent> {
        // Parse process-specific JSON format
        let pid = json.get("pid")?.as_u64()? as u32;
        let ppid = json.get("ppid")?.as_u64()? as u32;
        let comm = json.get("comm")?.as_str()?.to_string();
        let filename = json.get("filename")?.as_str()?.to_string();
        let event_type = json.get("event_type")?.as_str()?;

        let event_type = match event_type {
            "exec" => EventType::ProcessStart,
            "exit" => EventType::ProcessExit,
            "open" => EventType::FileAccess,
            _ => EventType::Custom(event_type.to_string()),
        };

        Some(ObservabilityEvent::new(
            EventSource::Process,
            event_type,
            EventData::Process { pid, ppid, comm, filename }
        ))
    }
}

pub struct SSLRunner {
    binary_path: String,
}

impl SSLRunner {
    pub fn new(binary_path: impl AsRef<Path>) -> Self {
        Self {
            binary_path: binary_path.as_ref().to_string_lossy().to_string(),
        }
    }
}

#[async_trait]
impl Runner for SSLRunner {
    async fn run(&self, sender: EventSender, timeout: Option<Duration>) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Starting SSL runner: {}", self.binary_path);
        
        let mut child = Command::new(&self.binary_path)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let mut reader = BufReader::new(stdout).lines();

        let start_time = std::time::Instant::now();
        
        loop {
            // Check timeout
            if let Some(timeout_duration) = timeout {
                if start_time.elapsed() > timeout_duration {
                    println!("⏰ SSL runner timeout reached");
                    let _ = child.kill().await;
                    break;
                }
            }

            tokio::select! {
                line = reader.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            println!("📤 SSL output: {}", line);
                            
                            // Try to parse as JSON first
                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
                                if let Some(event) = self.parse_ssl_event(&json_value) {
                                    let _ = sender.send(event);
                                }
                            } else {
                                // Raw output as custom event
                                let event = ObservabilityEvent::new(
                                    EventSource::SSL,
                                    EventType::Custom("raw_output".to_string()),
                                    EventData::Custom({
                                        let mut data = std::collections::HashMap::new();
                                        data.insert("raw_line".to_string(), serde_json::Value::String(line));
                                        data
                                    })
                                );
                                let _ = sender.send(event);
                            }
                        }
                        Ok(None) => {
                            println!("✅ SSL runner completed");
                            break;
                        }
                        Err(e) => {
                            println!("❌ SSL runner error: {}", e);
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Continue loop
                }
            }
        }

        let _ = child.kill().await;
        Ok(())
    }

    fn name(&self) -> &str {
        "ssl"
    }

    fn description(&self) -> &str {
        "SSL traffic monitoring runner"
    }
}

impl SSLRunner {
    fn parse_ssl_event(&self, json: &serde_json::Value) -> Option<ObservabilityEvent> {
        // Parse SSL-specific JSON format
        let function = json.get("function")?.as_str()?.to_string();
        let pid = json.get("pid")?.as_u64()? as u32;
        let comm = json.get("comm")?.as_str()?.to_string();
        let data = json.get("data")?.as_str()?.to_string();
        let data_len = json.get("len")?.as_u64()? as usize;
        let is_handshake = json.get("is_handshake")?.as_bool().unwrap_or(false);

        let event_type = if is_handshake {
            EventType::SSLHandshake
        } else {
            EventType::SSLData
        };

        Some(ObservabilityEvent::new(
            EventSource::SSL,
            event_type,
            EventData::SSL { function, pid, comm, data, data_len, is_handshake }
        ))
    }
}