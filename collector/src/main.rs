use clap::{Parser, Subcommand};
use futures::stream::StreamExt;

mod framework;

use framework::{
    binary_extractor::BinaryExtractor,
    runners::{SslRunner, ProcessRunner, RunnerError, Runner},
    analyzers::{OutputAnalyzer, FileLogger, SSEProcessor, HTTPParser, HTTPFilter}
};

fn convert_runner_error(e: RunnerError) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze SSL traffic with raw JSON output
    Ssl {
        /// Enable SSE processing for SSL traffic
        #[arg(long)]
        sse_merge: bool,
        /// Enable HTTP parsing (automatically enables SSE merge first)
        #[arg(long)]
        http_parser: bool,
        /// Include raw SSL data in HTTP parser events
        #[arg(long)]
        http_raw_data: bool,
        /// HTTP filter patterns to exclude events (can be used multiple times)
        #[arg(long)]
        http_filter: Vec<String>,
        /// Suppress console output
        #[arg(short, long)]
        quiet: bool,
        /// Additional arguments to pass to the SSL binary
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Test process runner with embedded binary
    Process {
        /// Suppress console output
        #[arg(short, long)]
        quiet: bool,
        /// Additional arguments to pass to the process binary
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Test both runners with embedded binaries
    Agent {
        /// Filter by process command name (comma-separated list)
        #[arg(short = 'c', long)]
        comm: Option<String>,
        /// Filter by process PID
        #[arg(short = 'p', long)]
        pid: Option<u32>,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Create BinaryExtractor with embedded binaries
    let binary_extractor = BinaryExtractor::new().await?;
    
    match &cli.command {
        Commands::Ssl { sse_merge, http_parser, http_raw_data, http_filter, quiet, args } => run_raw_ssl(&binary_extractor, *sse_merge, *http_parser, *http_raw_data, http_filter, *quiet, args).await.map_err(convert_runner_error)?,
        Commands::Process { quiet, args } => run_raw_process(&binary_extractor, *quiet, args).await.map_err(convert_runner_error)?,
        Commands::Agent { comm, pid } => run_both_real(&binary_extractor, comm.as_deref(), *pid).await?,
    }
    
    Ok(())
}

/// Test both runners with embedded binaries
async fn run_both_real(binary_extractor: &BinaryExtractor, comm: Option<&str>, pid: Option<u32>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Both Runners");
    println!("{}", "=".repeat(60));
    
    // Build arguments for filtering
    let mut args = Vec::new();
    if let Some(comm_filter) = comm {
        args.push("-c".to_string());
        args.push(comm_filter.to_string());
    }
    if let Some(pid_filter) = pid {
        args.push("-p".to_string());
        args.push(pid_filter.to_string());
    }
    
    let ssl_handle = {
        let ssl_path = binary_extractor.get_sslsniff_path().to_path_buf();
        let ssl_args = args.clone();
        tokio::spawn(async move {
            let mut ssl_runner = SslRunner::from_binary_extractor(ssl_path);
            
            // Add filter arguments if any
            if !ssl_args.is_empty() {
                ssl_runner = ssl_runner.with_args(&ssl_args);
            }
            
            ssl_runner = ssl_runner.add_analyzer(Box::new(OutputAnalyzer::new()));
            
            match ssl_runner.run().await {
                Ok(mut stream) => {
                    let mut count = 0;
                    println!("SSL Runner started, processing events...");
                    while let Some(_event) = stream.next().await {
                        count += 1;
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
        let process_path = binary_extractor.get_process_path().to_path_buf();
        let process_args = args.clone();
        tokio::spawn(async move {
            let mut process_runner = ProcessRunner::from_binary_extractor(process_path);
            
            // Add filter arguments if any
            if !process_args.is_empty() {
                process_runner = process_runner.with_args(&process_args);
            }
            
            process_runner = process_runner.add_analyzer(Box::new(OutputAnalyzer::new()));
            
            match process_runner.run().await {
                Ok(mut stream) => {
                    let mut count = 0;
                    println!("Process Runner started, processing events...");
                    while let Some(_event) = stream.next().await {
                        count += 1;
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
    
    let (ssl_count, process_count) = tokio::try_join!(ssl_handle, process_handle)?;
    
    println!("{}", "=".repeat(60));
    println!("Both runners completed!");
    println!("SSL events: {}", ssl_count);
    println!("Process events: {}", process_count);
    
    Ok(())
}

/// Show raw SSL events as JSON with optional chunk merging and HTTP parsing
async fn run_raw_ssl(binary_extractor: &BinaryExtractor, enable_chunk_merger: bool, enable_http_parser: bool, include_raw_data: bool, http_filter_patterns: &Vec<String>, quiet: bool, args: &Vec<String>) -> Result<(), RunnerError> {
    println!("Raw SSL Events");
    println!("{}", "=".repeat(60));
    
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path());
    
    // Add additional arguments if provided
    if !args.is_empty() {
        ssl_runner = ssl_runner.with_args(args);
    }
    
    // Add analyzers based on flags - when HTTP parser is enabled, always enable SSE merge first
    if enable_http_parser {
        ssl_runner = ssl_runner.add_analyzer(Box::new(SSEProcessor::new_with_timeout(30000)));
        
        // Create HTTP parser with appropriate configuration
        let http_parser = if include_raw_data {
            HTTPParser::new()
        } else {
            HTTPParser::new().disable_raw_data()
        };
        ssl_runner = ssl_runner.add_analyzer(Box::new(http_parser));
        
        // Add HTTP filter if patterns are provided
        if !http_filter_patterns.is_empty() {
            ssl_runner = ssl_runner.add_analyzer(Box::new(HTTPFilter::with_patterns(http_filter_patterns.clone())));
            let raw_data_info = if include_raw_data { " (with raw data)" } else { "" };
            println!("Starting SSL event stream with SSE processing, HTTP parsing{}, and HTTP filtering enabled (press Ctrl+C to stop):", raw_data_info);
        } else {
            let raw_data_info = if include_raw_data { " (with raw data)" } else { "" };
            println!("Starting SSL event stream with SSE processing and HTTP parsing{} enabled (press Ctrl+C to stop):", raw_data_info);
        }
    } else if enable_chunk_merger {
        ssl_runner = ssl_runner.add_analyzer(Box::new(SSEProcessor::new_with_timeout(30000)));
        println!("Starting SSL event stream with SSE processing enabled (press Ctrl+C to stop):");
    } else {
        println!("Starting SSL event stream with raw JSON output (press Ctrl+C to stop):");
    }
    
    ssl_runner = ssl_runner
        .add_analyzer(Box::new(FileLogger::new("ssl.log").unwrap()));
    
    if !quiet {
        ssl_runner = ssl_runner.add_analyzer(Box::new(OutputAnalyzer::new()));
    }
    
    let mut stream = ssl_runner.run().await?;
    
    // Consume the stream to actually process events
    while let Some(_event) = stream.next().await {
        // Events are processed by the analyzers in the chain
    }
    
    Ok(())
}

/// Show raw process events as JSON
async fn run_raw_process(binary_extractor: &BinaryExtractor, quiet: bool, args: &Vec<String>) -> Result<(), RunnerError> {
    println!("Raw Process Events");
    println!("{}", "=".repeat(60));
    
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_extractor.get_process_path());
    
    // Add additional arguments if provided
    if !args.is_empty() {
        process_runner = process_runner.with_args(args);
    }
    
    if !quiet {
        process_runner = process_runner.add_analyzer(Box::new(OutputAnalyzer::new()));
    }
    
    println!("Starting process event stream with raw JSON output (press Ctrl+C to stop):");
    let mut stream = process_runner.run().await?;

    // Consume the stream to actually process events
    while let Some(_event) = stream.next().await {
        // Events are processed by the analyzers in the chain
    }

    Ok(())
}
