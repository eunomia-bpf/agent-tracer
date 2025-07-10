use serde::{Deserialize, Serialize};
use std::process::{Command, Stdio};
use std::path::Path;
use std::fmt;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::time::{timeout, Duration};

#[derive(Debug, Serialize, Deserialize)]
pub struct SslEvent {
    #[serde(rename = "function")]
    pub function: String,
    pub time_s: f64,
    pub timestamp_ns: u64,
    pub comm: String,
    pub pid: u32,
    pub len: usize,
    pub is_handshake: bool,
    pub data: String,
    pub truncated: bool,
}

impl fmt::Display for SslEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let truncated_data = if self.data.len() > 100 {
            format!("{}...", &self.data[..100])
        } else {
            self.data.clone()
        };
        
        let handshake_indicator = if self.is_handshake { "🤝" } else { "📡" };
        let truncated_indicator = if self.truncated { " [TRUNCATED]" } else { "" };
        
        write!(
            f,
            "🔐 SSL {} {}: {} [PID: {}]\n   ⏰ Time: {:.6}s (NS: {})\n   📦 Data ({} bytes){}: {}",
            handshake_indicator,
            self.function,
            self.comm,
            self.pid,
            self.time_s,
            self.timestamp_ns,
            self.len,
            truncated_indicator,
            truncated_data
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
        println!("🔍 Executing sslsniff binary: {}", self.binary_path);
        let output = Command::new(&self.binary_path)
            .output()
            .expect("Failed to execute sslsniff binary");

        println!("📊 SSLSniff exit status: {}", output.status);
        println!("📤 SSLSniff stdout length: {} bytes", output.stdout.len());
        println!("📤 SSLSniff stderr length: {} bytes", output.stderr.len());

        if !output.stderr.is_empty() {
            println!("⚠️ SSLSniff stderr: {}", String::from_utf8_lossy(&output.stderr));
        }

        if !output.status.success() {
            return Err(format!("SSLSniff binary failed with status: {}", output.status).into());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("📝 Raw stdout content (first 500 chars): {}", 
                 if stdout.len() > 500 { &stdout[..500] } else { &stdout });
        
        let mut events = Vec::new();
        let mut line_count = 0;
        let mut parse_errors = 0;

        for line in stdout.lines() {
            line_count += 1;
            if line.trim().is_empty() {
                continue;
            }
            
            match serde_json::from_str::<SslEvent>(line) {
                Ok(event) => {
                    events.push(event);
                }
                Err(e) => {
                    parse_errors += 1;
                    println!("❌ Failed to parse line {}: {} | Error: {}", line_count, line, e);
                }
            }
        }

        println!("📈 Processed {} lines, {} events parsed, {} parse errors", 
                 line_count, events.len(), parse_errors);

        Ok(events)
    }

    pub async fn collect_raw_output(&self) -> Result<String, Box<dyn std::error::Error>> {
        self.collect_raw_output_with_timeout(Duration::from_secs(30)).await
    }

    pub async fn collect_raw_output_with_timeout(&self, timeout_duration: Duration) -> Result<String, Box<dyn std::error::Error>> {
        println!("🔍 Executing sslsniff binary: {}", self.binary_path);
        println!("⏱️ Capturing output for {} seconds...", timeout_duration.as_secs());

        let mut child = TokioCommand::new(&self.binary_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start sslsniff binary");

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