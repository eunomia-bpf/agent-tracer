use clap::{Parser, Subcommand};

mod binary_extractor;
mod process;
mod sslsniff;

use binary_extractor::BinaryExtractor;

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
        /// Additional arguments to pass to the process tracer
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run SSL sniffer
    Sslsniff {
        /// Additional arguments to pass to the SSL sniffer
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run both tracers
    Both {
        /// Additional arguments to pass to both tracers
        #[arg(last = true)]
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    println!("Starting collector...");
    
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
