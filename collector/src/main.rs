use clap::{Parser, Subcommand};

mod binary_extractor;
mod framework;
mod process;
mod sslsniff;

use binary_extractor::BinaryExtractor;
use framework::{SslRunner, ProcessRunner, RawAnalyzer, Runner};
use futures::stream::StreamExt;

#[derive(Parser)]
#[command(name = "collector")]
#[command(about = "A tracer collector for process and SSL events")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run process tracer (legacy)
    Process {
        /// Additional arguments to pass to the process tracer
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run SSL sniffer (legacy)
    Sslsniff {
        /// Additional arguments to pass to the SSL sniffer
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run both tracers (legacy)
    Both {
        /// Additional arguments to pass to both tracers
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Demo the new framework (simulation mode)
    Demo,
    /// Test SSL runner with real binary
    SslReal,
    /// Test process runner with real binary
    ProcessReal,
    /// Test both runners with real binaries
    BothReal,
    /// Test framework with raw analyzer output
    TestRaw {
        /// Use real binaries instead of simulation
        #[arg(long)]
        real: bool,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    println!("Starting collector...");
    
    match cli.command {
        Commands::Demo => {
            run_framework_demo().await?;
        }
        Commands::SslReal => {
            let extractor = BinaryExtractor::new().await?;
            run_ssl_real(&extractor).await?;
        }
        Commands::ProcessReal => {
            let extractor = BinaryExtractor::new().await?;
            run_process_real(&extractor).await?;
        }
        Commands::BothReal => {
            let extractor = BinaryExtractor::new().await?;
            run_both_real(&extractor).await?;
        }
        Commands::TestRaw { real } => {
            if real {
                let extractor = BinaryExtractor::new().await?;
                run_test_raw_real(&extractor).await?;
            } else {
                run_test_raw_simulation().await?;
            }
        }
        _ => {
            let extractor = BinaryExtractor::new().await?;
            
            match cli.command {
                Commands::Process { args } => {
                    run_process_tracer(&extractor, args).await?;
                }
                Commands::Sslsniff { args } => {
                    run_sslsniff_tracer(&extractor, args).await?;
                }
                Commands::Both { args } => {
                    run_both_tracers(&extractor, args).await?;
                }
                _ => unreachable!(),
            }
        }
    }
    
    Ok(())
}

async fn run_process_tracer(
    extractor: &BinaryExtractor,
    _args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let process_collector = process::ProcessCollector::new(extractor.get_process_path());
    
    println!("Starting process binary...");
    match process_collector.collect_events().await {
        Ok(events) => {
            println!("🔄 Process events collected: {}", events.len());
            println!("{}", "=".repeat(60));
            for event in events {
                println!("{}", event);
                println!("{}", "-".repeat(60));
            }
        }
        Err(e) => {
            println!("❌ Error collecting process events: {}", e);
        }
    }
    
    Ok(())
}

async fn run_sslsniff_tracer(
    extractor: &BinaryExtractor,
    _args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sslsniff_collector = sslsniff::SslSniffCollector::new(extractor.get_sslsniff_path());
    
    println!("Starting sslsniff binary...");
    match sslsniff_collector.collect_events().await {
        Ok(events) => {
            println!("🔐 SSL events collected: {}", events.len());
            println!("{}", "=".repeat(60));
            for event in events {
                println!("{}", event);
                println!("{}", "-".repeat(60));
            }
        }
        Err(e) => {
            println!("❌ Error collecting SSL events: {}", e);
        }
    }
    
    Ok(())
}

async fn run_both_tracers(
    extractor: &BinaryExtractor,
    _args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let process_collector = process::ProcessCollector::new(extractor.get_process_path());
    let sslsniff_collector = sslsniff::SslSniffCollector::new(extractor.get_sslsniff_path());
    
    let process_handle = tokio::spawn(async move {
        println!("Starting process binary...");
        match process_collector.collect_events().await {
            Ok(events) => {
                println!("🔄 Process events collected: {}", events.len());
                println!("{}", "=".repeat(60));
                for event in events {
                    println!("{}", event);
                    println!("{}", "-".repeat(60));
                }
            }
            Err(e) => {
                println!("❌ Error collecting process events: {}", e);
            }
        }
    });
    
    let sslsniff_handle = tokio::spawn(async move {
        println!("Starting sslsniff binary...");
        match sslsniff_collector.collect_events().await {
            Ok(events) => {
                println!("🔐 SSL events collected: {}", events.len());
                println!("{}", "=".repeat(60));
                for event in events {
                    println!("{}", event);
                    println!("{}", "-".repeat(60));
                }
            }
            Err(e) => {
                println!("❌ Error collecting SSL events: {}", e);
            }
        }
    });
    
    let _ = tokio::join!(process_handle, sslsniff_handle);
    
    println!("Both binaries have completed execution");
    
    Ok(())
}

/// Demo function showing the new framework in action
async fn run_framework_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Framework Demo: SSL Runner with Raw Analyzer");
    println!("{}", "=".repeat(60));
    
    // Create and configure an SSL runner with raw analyzer
    let mut ssl_runner = SslRunner::new()
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
    let mut process_runner = ProcessRunner::new()
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
async fn run_ssl_real(extractor: &BinaryExtractor) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔐 Testing SSL Runner with Real Binary");
    println!("{}", "=".repeat(60));
    
    let mut ssl_runner = SslRunner::from_binary_extractor(extractor.get_sslsniff_path())
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
async fn run_process_real(extractor: &BinaryExtractor) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔄 Testing Process Runner with Real Binary");
    println!("{}", "=".repeat(60));
    
    let mut process_runner = ProcessRunner::from_binary_extractor(extractor.get_process_path())
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
async fn run_both_real(extractor: &BinaryExtractor) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Both Runners with Real Binaries");
    println!("{}", "=".repeat(60));
    
    let ssl_handle = {
        let ssl_path = extractor.get_sslsniff_path().to_path_buf();
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
        let process_path = extractor.get_process_path().to_path_buf();
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
async fn run_test_raw_real(extractor: &BinaryExtractor) -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Framework with Raw Analyzer (Real Binaries)");
    println!("{}", "=".repeat(60));
    
    // Test SSL with raw output (printing to stdout)
    println!("📡 SSL Raw Output:");
    let mut ssl_runner = SslRunner::from_binary_extractor(extractor.get_sslsniff_path())
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let ssl_stream = ssl_runner.run().await?;
    let ssl_events: Vec<_> = ssl_stream.collect().await;
    
    println!();
    println!("🖥️  Process Raw Output:");
    let mut process_runner = ProcessRunner::from_binary_extractor(extractor.get_process_path())
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let process_stream = process_runner.run().await?;
    let process_events: Vec<_> = process_stream.collect().await;
    
    println!();
    println!("✅ Raw output test completed!");
    println!("   - SSL events: {}", ssl_events.len());
    println!("   - Process events: {}", process_events.len());
    
    Ok(())
}

/// Test framework with raw analyzer output (simulation)
async fn run_test_raw_simulation() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Framework with Raw Analyzer (Simulation)");
    println!("{}", "=".repeat(60));
    
    // Test SSL with raw output (printing to stdout)
    println!("📡 SSL Raw Output (Simulated):");
    let mut ssl_runner = SslRunner::new()
        .simulation(true)
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let ssl_stream = ssl_runner.run().await?;
    let ssl_events: Vec<_> = ssl_stream.collect().await;
    
    println!();
    println!("🖥️  Process Raw Output (Simulated):");
    let mut process_runner = ProcessRunner::new()
        .simulation(true)
        .add_analyzer(Box::new(RawAnalyzer::new())); // This will print to stdout
    
    let process_stream = process_runner.run().await?;
    let process_events: Vec<_> = process_stream.collect().await;
    
    println!();
    println!("✅ Raw output test completed!");
    println!("   - SSL events: {}", ssl_events.len());
    println!("   - Process events: {}", process_events.len());
    
    Ok(())
}
