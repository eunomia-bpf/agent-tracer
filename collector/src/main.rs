use clap::{Parser, Subcommand};
use tokio::time::Duration;

mod binary_extractor;
mod process;
mod sslsniff;
mod framework;

use binary_extractor::BinaryExtractor;
use framework::{
    ProcessRunner, SSLRunner, Runner,
    AggregatorAnalyzer, TimelineAnalyzer, FilterAnalyzer, Analyzer,
    InMemoryStorage, EventBroadcaster, receiver_to_stream
};

#[derive(Parser)]
#[command(name = "collector")]
#[command(about = "A tracer collector for process and SSL events")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run process tracer
    Process {
        /// Print raw output instead of parsing JSON
        #[arg(short, long)]
        raw: bool,
        /// Timeout in seconds (default: 10, 0 = no timeout)
        #[arg(short, long, default_value = "10")]
        timeout: u64,
        /// Additional arguments to pass to the process tracer
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run SSL sniffer
    Sslsniff {
        /// Print raw output instead of parsing JSON
        #[arg(short, long)]
        raw: bool,
        /// Timeout in seconds (default: 10, 0 = no timeout)
        #[arg(short, long, default_value = "10")]
        timeout: u64,
        /// Additional arguments to pass to the SSL sniffer
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run both tracers
    Both {
        /// Print raw output instead of parsing JSON
        #[arg(short, long)]
        raw: bool,
        /// Timeout in seconds (default: 10, 0 = no timeout)
        #[arg(short, long, default_value = "10")]
        timeout: u64,
        /// Additional arguments to pass to both tracers
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run with new framework (experimental)
    Framework {
        /// Timeout in seconds (default: 10, 0 = no timeout)
        #[arg(short, long, default_value = "10")]
        timeout: u64,
        /// Runner type: process, ssl, or both
        #[arg(short, long, default_value = "both")]
        runner: String,
        /// Storage size limit
        #[arg(short, long, default_value = "1000")]
        storage_limit: usize,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    println!("Starting collector...");
    
    let extractor = BinaryExtractor::new().await?;
    
    match cli.command {
        Commands::Process { raw, timeout, args } => {
            run_process_tracer(&extractor, raw, timeout, args).await?;
        }
        Commands::Sslsniff { raw, timeout, args } => {
            run_sslsniff_tracer(&extractor, raw, timeout, args).await?;
        }
        Commands::Both { raw, timeout, args } => {
            run_both_tracers(&extractor, raw, timeout, args).await?;
        }
        Commands::Framework { timeout, runner, storage_limit } => {
            run_framework(&extractor, timeout, &runner, storage_limit).await?;
        }
    }
    
    Ok(())
}

async fn run_process_tracer(
    extractor: &BinaryExtractor,
    raw: bool,
    timeout_secs: u64,
    _args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let process_collector = process::ProcessCollector::new(extractor.get_process_path());
    
    println!("Starting process binary...");
    
    if raw {
        let timeout_duration = if timeout_secs == 0 {
            tokio::time::Duration::from_secs(u64::MAX) // Effectively no timeout
        } else {
            tokio::time::Duration::from_secs(timeout_secs)
        };
        match process_collector.collect_raw_output_with_timeout(timeout_duration).await {
            Ok(output) => {
                println!("📄 Raw Process Output:");
                println!("{}", "=".repeat(60));
                println!("{}", output);
                println!("{}", "=".repeat(60));
            }
            Err(e) => {
                println!("❌ Error collecting process output: {}", e);
            }
        }
    } else {
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
    }
    
    Ok(())
}

async fn run_sslsniff_tracer(
    extractor: &BinaryExtractor,
    raw: bool,
    timeout_secs: u64,
    _args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let sslsniff_collector = sslsniff::SslSniffCollector::new(extractor.get_sslsniff_path());
    
    println!("Starting sslsniff binary...");
    
    if raw {
        let timeout_duration = if timeout_secs == 0 {
            tokio::time::Duration::from_secs(u64::MAX) // Effectively no timeout
        } else {
            tokio::time::Duration::from_secs(timeout_secs)
        };
        match sslsniff_collector.collect_raw_output_with_timeout(timeout_duration).await {
            Ok(output) => {
                println!("📄 Raw SSLSniff Output:");
                println!("{}", "=".repeat(60));
                println!("{}", output);
                println!("{}", "=".repeat(60));
            }
            Err(e) => {
                println!("❌ Error collecting SSL output: {}", e);
            }
        }
    } else {
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
    }
    
    Ok(())
}

async fn run_both_tracers(
    extractor: &BinaryExtractor,
    raw: bool,
    timeout_secs: u64,
    _args: Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let process_collector = process::ProcessCollector::new(extractor.get_process_path());
    let sslsniff_collector = sslsniff::SslSniffCollector::new(extractor.get_sslsniff_path());
    
    let process_handle = tokio::spawn(async move {
        println!("Starting process binary...");
        if raw {
            let timeout_duration = if timeout_secs == 0 {
                tokio::time::Duration::from_secs(u64::MAX)
            } else {
                tokio::time::Duration::from_secs(timeout_secs)
            };
            match process_collector.collect_raw_output_with_timeout(timeout_duration).await {
                Ok(output) => {
                    println!("📄 Raw Process Output:");
                    println!("{}", "=".repeat(60));
                    println!("{}", output);
                    println!("{}", "=".repeat(60));
                }
                Err(e) => {
                    println!("❌ Error collecting process output: {}", e);
                }
            }
        } else {
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
        }
    });
    
    let sslsniff_handle = tokio::spawn(async move {
        println!("Starting sslsniff binary...");
        if raw {
            let timeout_duration = if timeout_secs == 0 {
                tokio::time::Duration::from_secs(u64::MAX)
            } else {
                tokio::time::Duration::from_secs(timeout_secs)
            };
            match sslsniff_collector.collect_raw_output_with_timeout(timeout_duration).await {
                Ok(output) => {
                    println!("📄 Raw SSLSniff Output:");
                    println!("{}", "=".repeat(60));
                    println!("{}", output);
                    println!("{}", "=".repeat(60));
                }
                Err(e) => {
                    println!("❌ Error collecting SSL output: {}", e);
                }
            }
        } else {
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
        }
    });
    
    let _ = tokio::join!(process_handle, sslsniff_handle);
    
    println!("Both binaries have completed execution");
    
    Ok(())
}
