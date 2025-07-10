use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::time::{sleep, Duration};

const PROCESS_BINARY: &[u8] = include_bytes!("../../src/process");
const SSLSNIFF_BINARY: &[u8] = include_bytes!("../../src/sslsniff");

pub struct BinaryExtractor {
    pub temp_dir: TempDir,
    pub process_path: PathBuf,
    pub sslsniff_path: PathBuf,
}

impl BinaryExtractor {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        println!("Creating temporary directory...");
        
        let temp_dir = TempDir::new()?;
        let temp_path = temp_dir.path();
        
        println!("Created temporary directory: {}", temp_path.display());
        
        // Extract and setup the process binary
        let process_path = temp_path.join("process");
        Self::extract_binary(&process_path, PROCESS_BINARY, "process").await?;
        
        // Extract and setup the sslsniff binary
        let sslsniff_path = temp_path.join("sslsniff");
        Self::extract_binary(&sslsniff_path, SSLSNIFF_BINARY, "sslsniff").await?;
        
        // Small delay to ensure files are fully written
        sleep(Duration::from_millis(100)).await;
        
        Ok(Self {
            temp_dir,
            process_path,
            sslsniff_path,
        })
    }
    
    async fn extract_binary(
        path: &Path,
        binary_data: &[u8],
        name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        {
            let mut file = fs::File::create(path)?;
            file.write_all(binary_data)?;
            file.flush()?;
        } // File is closed here
        
        // Make the binary executable
        let mut perms = fs::metadata(path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms)?;
        
        println!("Extracted {} binary to: {}", name, path.display());
        
        Ok(())
    }
    
    pub fn get_process_path(&self) -> &Path {
        &self.process_path
    }
    
    pub fn get_sslsniff_path(&self) -> &Path {
        &self.sslsniff_path
    }
}