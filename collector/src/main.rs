use clap::{Parser, Subcommand};
use framework::{SslRunner, ProcessRunner, RawAnalyzer, Runner, RunnerError};
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
    SslReal,
    /// Test process runner with real binary
    ProcessReal,
    /// Test both runners with real binaries
    BothReal,
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
        Commands::SslReal => {
            run_ssl_real(&binary_paths).await.map_err(convert_runner_error)?;
        }
        Commands::ProcessReal => {
            run_process_real(&binary_paths).await.map_err(convert_runner_error)?;
        }
        Commands::BothReal => {
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
    println!("🚀 Framework Demo: SSL Runner with Raw Analyzer");
    println!("{}", "=".repeat(60));
    
    // Create and configure an SSL runner with raw analyzer
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_paths.get_sslsniff_path())
        .with_id("demo-ssl".to_string())
        .port(443)
        .interface("eth0".to_string())
        .add_analyzer(Box::new(RawAnalyzer::new_with_options(false))); // Don't print to stdout
    
    // Run the SSL collection
    let ssl_stream = ssl_runner.run().await?;
    let ssl_events: Vec<_> = ssl_stream.collect().await;
    
    println!("📡 SSL Runner collected {} events:", ssl_events.len());
    for event in &ssl_events {
        println!("  {}", event);
    }
    
    println!();
    println!("🔄 Framework Demo: Process Runner with Raw Analyzer");
    println!("{}", "=".repeat(60));
    
    // Create and configure a process runner with raw analyzer
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_paths.get_process_path())
        .with_id("demo-process".to_string())
        .name_filter("python".to_string())
        .cpu_threshold(80.0)
        .add_analyzer(Box::new(RawAnalyzer::new_with_options(false))); // Don't print to stdout
    
    // Run the process collection
    let process_stream = process_runner.run().await?;
    let process_events: Vec<_> = process_stream.collect().await;
    
    println!("🖥️  Process Runner collected {} events:", process_events.len());
    for event in &process_events {
        println!("  {}", event);
    }
    
    println!();
    println!("✅ Framework Demo completed successfully!");
    println!("   - SslRunner: {} events", ssl_events.len());
    println!("   - ProcessRunner: {} events", process_events.len());
    println!("   - Total events: {}", ssl_events.len() + process_events.len());
    
    Ok(())
}

/// Test SSL runner with real binary
async fn run_ssl_real(binary_paths: &BinaryPaths) -> Result<(), RunnerError> {
    println!("🔐 Testing SSL Runner with Real Binary");
    println!("{}", "=".repeat(60));
    
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_paths.get_sslsniff_path())
        .with_id("real-ssl".to_string())
        .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));
    
    let stream = ssl_runner.run().await?;
    let events: Vec<_> = stream.collect().await;
    
    println!("📡 SSL Runner (Real) collected {} events:", events.len());
    for event in &events {
        println!("  {}", event);
    }
    
    Ok(())
}

/// Test process runner with real binary
async fn run_process_real(binary_paths: &BinaryPaths) -> Result<(), RunnerError> {
    println!("🔄 Testing Process Runner with Real Binary");
    println!("{}", "=".repeat(60));
    
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_paths.get_process_path())
        .with_id("real-process".to_string())
        .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));
    
    let stream = process_runner.run().await?;
    let events: Vec<_> = stream.collect().await;
    
    println!("🖥️  Process Runner (Real) collected {} events:", events.len());
    for event in &events {
        println!("  {}", event);
    }
    
    Ok(())
}

/// Test both runners with real binaries
async fn run_both_real(binary_paths: &BinaryPaths) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Both Runners with Real Binaries");
    println!("{}", "=".repeat(60));
    
    let ssl_handle = {
        let ssl_path = binary_paths.get_sslsniff_path().to_path_buf();
        tokio::spawn(async move {
            let mut ssl_runner = SslRunner::from_binary_extractor(ssl_path)
                .with_id("real-ssl".to_string())
                .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));
            
            match ssl_runner.run().await {
                Ok(stream) => {
                    let events: Vec<_> = stream.collect().await;
                    println!("📡 SSL Runner (Real) collected {} events:", events.len());
                    for event in &events {
                        println!("  SSL: {}", event);
                    }
                    events.len()
                }
                Err(e) => {
                    println!("❌ SSL Runner error: {}", e);
                    0
                }
            }
        })
    };
    
    let process_handle = {
        let process_path = binary_paths.get_process_path().to_path_buf();
        tokio::spawn(async move {
            let mut process_runner = ProcessRunner::from_binary_extractor(process_path)
                .with_id("real-process".to_string())
                .add_analyzer(Box::new(RawAnalyzer::new_with_options(false)));
            
            match process_runner.run().await {
                Ok(stream) => {
                    let events: Vec<_> = stream.collect().await;
                    println!("🖥️  Process Runner (Real) collected {} events:", events.len());
                    for event in &events {
                        println!("  PROC: {}", event);
                    }
                    events.len()
                }
                Err(e) => {
                    println!("❌ Process Runner error: {}", e);
                    0
                }
            }
        })
    };
    
    let (ssl_count, process_count) = tokio::join!(ssl_handle, process_handle);
    
    println!();
    println!("✅ Both Real Runners completed!");
    println!("   - SSL events: {}", ssl_count.unwrap_or(0));
    println!("   - Process events: {}", process_count.unwrap_or(0));
    
    Ok(())
}

/// Test framework with raw analyzer output (real binaries)
async fn run_test_raw_real(binary_paths: &BinaryPaths) -> Result<(), RunnerError> {
    println!("🧪 Testing Framework with Raw Analyzer (Real Binaries)");
    println!("{}", "=".repeat(60));
    
    // Test SSL with raw output (printing to stdout)
    println!("📡 SSL Raw Output:");
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_paths.get_sslsniff_path())
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let ssl_stream = ssl_runner.run().await?;
    let ssl_events: Vec<_> = ssl_stream.collect().await;
    
    println!();
    println!("🖥️  Process Raw Output:");
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_paths.get_process_path())
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let process_stream = process_runner.run().await?;
    let process_events: Vec<_> = process_stream.collect().await;
    
    println!();
    println!("✅ Raw output test completed!");
    println!("   - SSL events: {}", ssl_events.len());
    println!("   - Process events: {}", process_events.len());
    
    Ok(())
}
