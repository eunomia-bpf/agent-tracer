use clap::{Parser, Subcommand};
use futures::stream::StreamExt;

mod framework;

use framework::{
    binary_extractor::BinaryExtractor,
    runners::{SslRunner, ProcessRunner, RunnerError, Runner},
    analyzers::{OutputAnalyzer, RawAnalyzer, HttpAnalyzer, FileLogger}
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
    /// Analyze HTTPS traffic and merge request/response pairs  
    Ssl,
    /// Test process runner with embedded binary
    Process,
    /// Test both runners with embedded binaries
    Both,
    /// Show raw SSL events as JSON
    RawSsl,
    /// Show raw process events as JSON  
    RawProcess,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Create BinaryExtractor with embedded binaries
    let binary_extractor = BinaryExtractor::new().await?;
    
    match &cli.command {
        Commands::Ssl => run_ssl_with_http_analyzer(&binary_extractor).await.map_err(convert_runner_error)?,
        Commands::Process => run_process_real(&binary_extractor).await.map_err(convert_runner_error)?,
        Commands::Both => run_both_real(&binary_extractor).await?,
        Commands::RawSsl => run_raw_ssl(&binary_extractor).await.map_err(convert_runner_error)?,
        Commands::RawProcess => run_raw_process(&binary_extractor).await.map_err(convert_runner_error)?,
    }
    
    Ok(())
}

/// Test process runner with embedded binary
async fn run_process_real(binary_extractor: &BinaryExtractor) -> Result<(), RunnerError> {
    println!("Testing Process Runner");
    println!("{}", "=".repeat(60));
    
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
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

/// Test both runners with embedded binaries
async fn run_both_real(binary_extractor: &BinaryExtractor) -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing Both Runners");
    println!("{}", "=".repeat(60));
    
            let ssl_handle = {
        let ssl_path = binary_extractor.get_sslsniff_path().to_path_buf();
        tokio::spawn(async move {
            let mut ssl_runner = SslRunner::from_binary_extractor(ssl_path)
                .with_id("ssl-both".to_string())
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
        let process_path = binary_extractor.get_process_path().to_path_buf();
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
    
    let (ssl_count, process_count) = tokio::try_join!(ssl_handle, process_handle)?;
    
    println!("{}", "=".repeat(60));
    println!("Both runners completed!");
    println!("SSL events: {}", ssl_count);
    println!("Process events: {}", process_count);
    
    Ok(())
}

/// Analyze HTTPS traffic and merge request/response pairs (renamed from run_http_ssl_real)
async fn run_ssl_with_http_analyzer(binary_extractor: &BinaryExtractor) -> Result<(), RunnerError> {
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-http".to_string())
        .add_analyzer(Box::new(HttpAnalyzer::new_with_wait_time(30000))) // 30 second wait time
        .add_analyzer(Box::new(FileLogger::new_with_options("https.log", true, true).map_err(|e| Box::new(e) as RunnerError)?)) // Log ALL events to https.log
        .add_analyzer(Box::new(OutputAnalyzer::new_with_options(true, true, true))); // Pretty print JSON
    
    println!("Starting HTTPS traffic analysis (press Ctrl+C to stop):");
    let mut stream = ssl_runner.run().await?;
    
    let mut event_count = 0;
    let mut pair_count = 0;
    
    while let Some(event) = stream.next().await {
        event_count += 1;
        
        // Count HTTP request/response pairs
        if event.source == "http_analyzer" {
            if let Some(event_type) = event.data.get("type") {
                match event_type.as_str() {
                    Some("http_request_response_pair") => {
                        pair_count += 1;
                        eprintln!("✓ HTTP Pair #{}: {} {} -> {} {}", 
                            pair_count,
                            event.data["request"]["method"].as_str().unwrap_or("?"),
                            event.data["request"]["url"].as_str().unwrap_or("?"),
                            event.data["response"]["status_code"].as_u64().unwrap_or(0),
                            event.data["response"]["status_text"].as_str().unwrap_or("?")
                        );
                    }
                    Some("http_incomplete_request") => {
                        eprintln!("⚠ Incomplete request: {} {}", 
                            event.data["request"]["method"].as_str().unwrap_or("?"),
                            event.data["request"]["url"].as_str().unwrap_or("?")
                        );
                    }
                    _ => {}
                }
            }
        }
        
        if event_count % 50 == 0 {
            eprintln!("Processed {} events so far ({} HTTP pairs)...", event_count, pair_count);
        }
    }
    
    println!("{}", "=".repeat(60));
    println!("HTTPS Analyzer completed!");
    println!("Total events processed: {}", event_count);
    println!("HTTP request/response pairs found: {}", pair_count);
    
    if pair_count == 0 {
        println!();
        println!("No HTTP pairs found. Try generating HTTPS traffic:");
        println!("  curl -v https://httpbin.org/get");
        println!("  curl -v -X POST -d 'test=data' https://httpbin.org/post");
    }
    
    Ok(())
}

/// Show raw SSL events as JSON (renamed from run_test_raw_real)
async fn run_raw_ssl(binary_extractor: &BinaryExtractor) -> Result<(), RunnerError> {
    println!("Raw SSL Events");
    println!("{}", "=".repeat(60));
    
    let mut ssl_runner = SslRunner::from_binary_extractor(binary_extractor.get_sslsniff_path())
        .with_id("ssl-raw".to_string())
        .add_analyzer(Box::new(RawAnalyzer::new_with_options(true)));
    
    println!("Starting SSL event stream with raw JSON output (press Ctrl+C to stop):");
    let mut stream = ssl_runner.run().await?;
    
    let mut event_count = 0;
    while let Some(_event) = stream.next().await {
        event_count += 1;
        if event_count % 10 == 0 {
            eprintln!("Raw processed {} events so far...", event_count);
        }
    }
    
    println!("Raw SSL analyzer completed with {} events", event_count);
    Ok(())
}

/// Show raw process events as JSON
async fn run_raw_process(binary_extractor: &BinaryExtractor) -> Result<(), RunnerError> {
    println!("Raw Process Events");
    println!("{}", "=".repeat(60));
    
    let mut process_runner = ProcessRunner::from_binary_extractor(binary_extractor.get_process_path())
        .with_id("process-raw".to_string())
        .add_analyzer(Box::new(RawAnalyzer::new_with_options(true)));
    
    println!("Starting process event stream with raw JSON output (press Ctrl+C to stop):");
    let mut stream = process_runner.run().await?;
    
    let mut event_count = 0;
    while let Some(_event) = stream.next().await {
        event_count += 1;
        if event_count % 10 == 0 {
            eprintln!("Raw processed {} events so far...", event_count);
        }
    }
    
    println!("Raw process events completed with {} events", event_count);
    Ok(())
}
