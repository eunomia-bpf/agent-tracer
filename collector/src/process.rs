use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::path::Path;
use std::fmt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::{timeout, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessEvent {
    pub pid: u32,
    pub ppid: u32,
    pub comm: String,
    pub filename: String,
    pub timestamp: u64,
    pub event_type: String,
}

impl fmt::Display for ProcessEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "🔄 Process Event: {} [PID: {}] [PPID: {}]\n   📁 File: {}\n   ⏰ Timestamp: {}\n   📊 Type: {}",
            self.comm, self.pid, self.ppid, self.filename, self.timestamp, self.event_type
        )
    }
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

    pub async fn collect_raw_output(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.collect_raw_output_with_timeout(Duration::from_secs(30)).await
    }

    pub async fn collect_raw_output_with_timeout(&self, timeout_duration: Duration) -> Result<String, Box<dyn std::error::Error>> {
        println!("🔍 Executing process binary: {}", self.binary_path);
        println!("⏱️ Capturing output for {} seconds...", timeout_duration.as_secs());

        let mut child = TokioCommand::new(&self.binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start process binary");

        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        
        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();
        
        let mut output_lines = Vec::new();
        let mut stderr_lines = Vec::new();
        
        println!("📡 Waiting for output...");

        let result = timeout(timeout_duration, async {
            loop {
                tokio::select! {
                    line = stdout_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                println!("📤 Output: {}", line);
                                output_lines.push(line);
                            }
                            Ok(None) => break,
                            Err(e) => {
                                println!("❌ Error reading stdout: {}", e);
                                break;
                            }
                        }
                    }
                    line = stderr_reader.next_line() => {
                        match line {
                            Ok(Some(line)) => {
                                println!("⚠️ Stderr: {}", line);
                                stderr_lines.push(line);
                            }
                            Ok(None) => {},
                            Err(e) => {
                                println!("❌ Error reading stderr: {}", e);
                            }
                        }
                    }
                }
                
                // If we have some output, we can continue for a bit longer
                if !output_lines.is_empty() || !stderr_lines.is_empty() {
                    // Small delay to allow more output
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }).await;

        // Kill the process
        let _ = child.kill().await;
        
        println!("📊 Captured {} stdout lines, {} stderr lines", output_lines.len(), stderr_lines.len());
        
        if !stderr_lines.is_empty() {
            println!("⚠️ Stderr output:");
            for line in &stderr_lines {
                println!("   {}", line);
            }
        }

        match result {
            Ok(_) => println!("✅ Process completed normally"),
            Err(_) => println!("⏰ Timeout reached, stopping capture"),
        }

        Ok(output_lines.join("\n"))
    }
}