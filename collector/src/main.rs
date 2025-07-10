use clap::{Parser, Subcommand};
use framework::{SslRunner, ProcessRunner, RawAnalyzer, OutputAnalyzer, Runner, RunnerError};
use futures::stream::StreamExt;
use std::path::PathBuf;

mod framework;

// Helper function to convert RunnerError to Box<dyn std::error::Error>
fn convert_runner_error(e: RunnerError) -> Box<dyn std::error::Error> {
    e as Box<dyn std::error::Error>
}

// Simple binary path provider for testing
struct BinaryPaths {
    sslsniff_path: PathBuf,
    process_path: PathBuf,
}

impl BinaryPaths {
    fn new() -> Self {
        Self {
            sslsniff_path: PathBuf::from("../src/sslsniff"),
            process_path: PathBuf::from("../src/process"),
        }
    }
    
    fn get_sslsniff_path(&self) -> &PathBuf {
        &self.sslsniff_path
    }
    
    fn get_process_path(&self) -> &PathBuf {
        &self.process_path
    }
}

#[derive(Parser)]
#[command(name = "collector")]
#[command(about = "A tracer collector for process and SSL events")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Demo the new framework with real binaries
    Demo,
    /// Test SSL runner with real binary
    Ssl,
    /// Test process runner with real binary
    Process,
    /// Test both runners with real binaries
    Both,
    /// Test framework with raw analyzer output
    TestRaw,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    println!("Starting collector...");
    
    let binary_paths = BinaryPaths::new();
    
    match cli.command {
        Commands::Demo => {
            run_framework_demo(&binary_paths).await.map_err(convert_runner_error)?;
        }
        Commands::Ssl => {
            run_ssl_real(&binary_paths).await.map_err(convert_runner_error)?;
        }
        Commands::Process => {
            run_process_real(&binary_paths).await.map_err(convert_runner_error)?;
        }
        Commands::Both => {
            run_both_real(&binary_paths).await?;
        }
        Commands::TestRaw => {
            run_test_raw_real(&binary_paths).await.map_err(convert_runner_error)?;
        }
    }
    
    Ok(())
}

/// Demo function showing the new framework in action
async fn run_framework_demo(binary_paths: &BinaryPaths) -> Result<(), RunnerError> {
    println!("Framework Demo: SSL Runner with Output Analyzer");
    println!("{}", "=".repeat(60));
    
    // Create and configure an SSL runner with output analyzer
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_paths.get_sslsniff_path())
        .with_id("demo-ssl".to_string())
        .port(443)
        .interface("eth0".to_string())
        .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, true, false))); // timestamps off for demo
    
    // Run the SSL collection with streaming output
    println!("SSL Runner starting (streaming events in real-time):");
    let mut ssl_stream = ssl_runner.run().await?;
    let mut ssl_count = 0;
    while let Some(_event) = ssl_stream.next().await {
        ssl_count += 1;
        if ssl_count >= 5 { // Limit demo events
            break;
        }
    }
    
    println!();
    println!("Framework Demo: Process Runner with Output Analyzer");
    println!("{}", "=".repeat(60));
    
    // Create and configure a process runner with output analyzer
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_paths.get_process_path())
        .with_id("demo-process".to_string())
        .name_filter("python".to_string())
        .cpu_threshold(80.0)
        .add_analyzer(Box::new(OutputAnalyzer::new_with_options(false, true, false))); // timestamps off for demo
    
    // Run the process collection with streaming output
    println!("Process Runner starting (streaming events in real-time):");
    let mut process_stream = process_runner.run().await?;
    let mut process_count = 0;
    while let Some(_event) = process_stream.next().await {
        process_count += 1;
        if process_count >= 5 { // Limit demo events
            break;
        }
    }
    
    println!();
    println!("Framework Demo completed successfully!");
    println!("   - SslRunner: {} events", ssl_count);
    println!("   - ProcessRunner: {} events", process_count);
    println!("   - Total events: {}", ssl_count + process_count);
    
    Ok(())
}

/// Test SSL runner with real binary
async fn run_ssl_real(binary_paths: &BinaryPaths) -> Result<(), RunnerError> {
    println!("Testing SSL Runner");
    println!("{}", "=".repeat(60));
    
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_paths.get_sslsniff_path())
        .with_id("ssl".to_string())
        .add_analyzer(Box::new(OutputAnalyzer::new_simple()));
    
    println!("Starting SSL event stream (press Ctrl+C to stop):");
    let mut stream = ssl_runner.run().await?;
    
    // Process events as they come in - this provides real-time output
    let mut event_count = 0;
    while let Some(_event) = stream.next().await {
        event_count += 1;
        // OutputAnalyzer already prints the events, we just count them
        if event_count % 10 == 0 {
            eprintln!("Processed {} events so far...", event_count);
        }
    }
    
    println!("SSL Runner completed with {} events", event_count);
    Ok(())
}

/// Test process runner with real binary
async fn run_process_real(binary_paths: &BinaryPaths) -> Result<(), RunnerError> {
    println!("Testing Process Runner");
    println!("{}", "=".repeat(60));
    
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_paths.get_process_path())
        .with_id("process".to_string())
        .add_analyzer(Box::new(OutputAnalyzer::new_simple()));
    
    println!("Starting process event stream (press Ctrl+C to stop):");
    let mut stream = process_runner.run().await?;
    
    // Process events as they come in - this provides real-time output
    let mut event_count = 0;
    while let Some(_event) = stream.next().await {
        event_count += 1;
        // OutputAnalyzer already prints the events, we just count them
        if event_count % 10 == 0 {
            eprintln!("Processed {} events so far...", event_count);
        }
    }
    
    println!("Process Runner completed with {} events", event_count);
    Ok(())
}

/// Test both runners with real binaries
async fn run_both_real(binary_paths: &BinaryPaths) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Both Runners");
    println!("{}", "=".repeat(60));
    
    let ssl_handle = {
        let ssl_path = binary_paths.get_sslsniff_path().to_path_buf();
        tokio::spawn(async move {
            let mut ssl_runner = SslRunner::from_binary_extractor(ssl_path)
                .with_id("ssl".to_string())
                .add_analyzer(Box::new(OutputAnalyzer::new_with_options(true, true, false)));
            
            match ssl_runner.run().await {
                Ok(mut stream) => {
                    let mut count = 0;
                    println!("SSL Runner started, processing events...");
                    while let Some(_event) = stream.next().await {
                        count += 1;
                        if count % 5 == 0 {
                            eprintln!("SSL: {} events processed", count);
                        }
                    }
                    println!("SSL Runner completed with {} events", count);
                    count
                }
                Err(e) => {
                    println!("SSL Runner error: {}", e);
                    0
                }
            }
        })
    };
    
    let process_handle = {
        let process_path = binary_paths.get_process_path().to_path_buf();
        tokio::spawn(async move {
            let mut process_runner = ProcessRunner::from_binary_extractor(process_path)
                .with_id("process".to_string())
                .add_analyzer(Box::new(OutputAnalyzer::new_with_options(true, true, false)));
            
            match process_runner.run().await {
                Ok(mut stream) => {
                    let mut count = 0;
                    println!("Process Runner started, processing events...");
                    while let Some(_event) = stream.next().await {
                        count += 1;
                        if count % 5 == 0 {
                            eprintln!("Process: {} events processed", count);
                        }
                    }
                    println!("Process Runner completed with {} events", count);
                    count
                }
                Err(e) => {
                    println!("Process Runner error: {}", e);
                    0
                }
            }
        })
    };
    
    let (ssl_count, process_count) = tokio::join!(ssl_handle, process_handle);
    
    println!();
    println!("Both Runners completed!");
    println!("   - SSL events: {}", ssl_count.unwrap_or(0));
    println!("   - Process events: {}", process_count.unwrap_or(0));
    
    Ok(())
}

/// Test framework with raw analyzer output (real binaries)
async fn run_test_raw_real(binary_paths: &BinaryPaths) -> Result<(), RunnerError> {
    println!("Testing Framework with Raw Analyzer (Real Binaries)");
    println!("{}", "=".repeat(60));
    
    // Test SSL with raw output (printing to stdout)
    println!("SSL Raw Output:");
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_paths.get_sslsniff_path())
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let ssl_stream = ssl_runner.run().await?;
    let ssl_events: Vec<_> = ssl_stream.collect().await;
    
    println!();
    println!("Process Raw Output:");
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_paths.get_process_path())
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let process_stream = process_runner.run().await?;
    let process_events: Vec<_> = process_stream.collect().await;
    
    println!();
    println!("Raw output test completed!");
    println!("   - SSL events: {}", ssl_events.len());
    println!("   - Process events: {}", process_events.len());
    
    Ok(())
}
