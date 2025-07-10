use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

mod process;
mod sslsniff;

// Embed the binaries at compile time
const PROCESS_BINARY: &[u8] = include_bytes!("../../src/process");
const SSLSNIFF_BINARY: &[u8] = include_bytes!("../../src/sslsniff");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting collector...");
    
    // Create a temporary directory
    let temp_dir = TempDir::new()?;
    let temp_path = temp_dir.path();
    
    println!("Created temporary directory: {}", temp_path.display());
    
    // Extract and setup the process binary
    let process_path = temp_path.join("process");
    {
        let mut process_file = fs::File::create(&process_path)?;
        process_file.write_all(PROCESS_BINARY)?;
        process_file.flush()?;
    } // File is closed here
    
    // Make the process binary executable
    let mut perms = fs::metadata(&process_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&process_path, perms)?;
    
    println!("Extracted process binary to: {}", process_path.display());
    
    // Extract and setup the sslsniff binary
    let sslsniff_path = temp_path.join("sslsniff");
    {
        let mut sslsniff_file = fs::File::create(&sslsniff_path)?;
        sslsniff_file.write_all(SSLSNIFF_BINARY)?;
        sslsniff_file.flush()?;
    } // File is closed here
    
    // Make the sslsniff binary executable
    let mut perms = fs::metadata(&sslsniff_path)?.permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&sslsniff_path, perms)?;
    
    println!("Extracted sslsniff binary to: {}", sslsniff_path.display());
    
    // Small delay to ensure files are fully written
    sleep(Duration::from_millis(100)).await;
    
    // Create collectors
    let process_collector = process::ProcessCollector::new(&process_path);
    let sslsniff_collector = sslsniff::SslSniffCollector::new(&sslsniff_path);
    
    // Start the process binary in background
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
    
    // Start the sslsniff binary in background
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
    
    // Wait for both processes to complete
    let _ = tokio::join!(process_handle, sslsniff_handle);
    
    println!("Both binaries have completed execution");
    
    // The temporary directory will be automatically cleaned up when temp_dir goes out of scope
    Ok(())
}
