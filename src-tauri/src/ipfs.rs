use serde::Serialize;
use std::fs::{self, File};
use std::io::copy;
use std::process::Command;

#[derive(Debug, Serialize)]
pub enum IpfsError {
    // AlreadyRunning,
    HomeDirNotFound,
    // CommandFailed(String),
    ConfigError(String),
    Other(String),
}

async fn get_latest_version() -> Result<String, String> {
    // Use the async version of reqwest
    let response = reqwest::get("https://dist.ipfs.tech/kubo/versions")
        .await
        .map_err(|e| e.to_string())?;

    let text = response.text().await.map_err(|e| e.to_string())?;

    // Split the response into lines and filter out release candidates
    let latest_stable = text
        .lines()
        .filter(|line| !line.contains("-rc")) // Exclude release candidates
        .max() // Get the maximum version string (latest)
        .ok_or("No stable versions found")?
        .to_string();

    Ok(latest_stable)
}

fn get_ipfs_download_url(version: &str) -> Result<String, String> {
    if cfg!(target_os = "windows") {
        Ok(format!(
            "https://dist.ipfs.io/kubo/{}/kubo_{}_windows-amd64.zip",
            version,
            version // Use the original version with 'v'
        ))
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "x86_64") {
            Ok(format!(
                "https://dist.ipfs.io/kubo/{}/kubo_{}_darwin-amd64.tar.gz",
                version,
                version // Use the original version with 'v'
            ))
        } else if cfg!(target_arch = "aarch64") {
            Ok(format!(
                "https://dist.ipfs.io/kubo/{}/kubo_{}_darwin-arm64.tar.gz",
                version,
                version // Use the original version with 'v'
            ))
        } else {
            Err("Unsupported macOS architecture".into())
        }
    } else if cfg!(target_os = "linux") {
        Ok(format!(
            "https://dist.ipfs.io/kubo/{}/kubo_{}_linux-amd64.tar.gz",
            version,
            version // Use the original version with 'v'
        ))
    } else {
        Err("Unsupported operating system".into())
    }
}

fn get_installed_version() -> Result<String, String> {
    // Assuming the installed version is stored in a specific file
    let kubo_binary_path = get_ipfs_binary_path().unwrap();
    let output = Command::new(&kubo_binary_path)
        .arg("version")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        // Convert stdout to a String and trim whitespace
        let version_output = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Extract the version number from the output
        if let Some(version) = version_output.strip_prefix("ipfs version ") {
            return Ok(format!("v{}", version)); // Format it as "v0.29.0"
        } else {
            return Err("Unexpected output format".into());
        }
    } else {
        Err("Failed to get installed IPFS version".into())
    }
}

pub async fn check_if_ipfs_latest_version() -> Result<bool, String> {
    println!("Checking if IPFS is the latest version");

    let latest_version = get_latest_version().await?;
    println!("Latest Version: {}", latest_version);

    let installed_version = get_installed_version()?;
    println!("Installed Version: {}", installed_version);

    Ok(installed_version == latest_version) // Compare versions
}

fn get_ipfs_binary_path() -> Result<String, IpfsError> {
    let home_dir = dirs::home_dir().ok_or(IpfsError::HomeDirNotFound)?;
    let cyb_dir = home_dir.join(".cyb");
    let ipfs_binary = cyb_dir.join("kubo/ipfs"); // Adjust based on the extracted path

    Ok(ipfs_binary
        .to_str()
        .ok_or(IpfsError::Other("Invalid path".into()))?
        .to_string())
}

#[tauri::command]
pub async fn download_and_extract_ipfs() -> Result<(), String> {
    let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
    let cyb_dir = home_dir.join(".cyb");

    // Create the ~/.cyb directory if it doesn't exist
    if !cyb_dir.exists() {
        fs::create_dir_all(&cyb_dir).map_err(|e| e.to_string())?;
    }

    let version = get_latest_version().await?;
    println!("Downloading IPFS version: {}", version);
    let url = get_ipfs_download_url(&version)?;
    println!("Download URL: {}", url);

    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    let tar_path = cyb_dir.join("kubo-ipfs-binary.tar.gz");

    let mut file = File::create(&tar_path).map_err(|e| e.to_string())?;
    copy(
        &mut response.bytes().await.map_err(|e| e.to_string())?.as_ref(),
        &mut file,
    )
    .map_err(|e| e.to_string())?;

    // Extract the tar.gz
    let output = Command::new("tar")
        .arg("-xzf")
        .arg(tar_path.to_str().ok_or("Invalid tar path")?)
        .arg("-C")
        .arg(cyb_dir.to_str().ok_or("Invalid extraction directory")?)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into())
    }
}

#[tauri::command]
pub async fn start_ipfs() -> Result<(), IpfsError> {
    println!("Starting IPFS");
    // check if installed
    let is_ipfs_installed = check_if_ipfs_exists()
        .await
        .unwrap_or(false);
    println!("IPFS installed: {}", is_ipfs_installed);

    // check if latest version (only if binary exists)
    let is_ipfs_latest_version = if is_ipfs_installed {
        check_if_ipfs_latest_version().await.unwrap_or(false)
    } else {
        false
    };
    println!("IPFS latest version: {}", is_ipfs_latest_version);

    // if not download and extract
    if !is_ipfs_installed || !is_ipfs_latest_version {
        println!("Downloading and extracting IPFS");
        match download_and_extract_ipfs().await {
            Ok(_) => println!("IPFS downloaded and extracted successfully"),
            Err(e) => return Err(IpfsError::Other(e)),
        }
        println!("IPFS downloaded and extracted successfully");
    }

    // Check if IPFS is initialized
    let is_ipfs_initialized = is_ipfs_initialized().unwrap_or(false);
    if !is_ipfs_initialized {
        println!("Initializing IPFS");
        match init_ipfs() {
            Ok(_) => println!("IPFS initialized successfully"),
            Err(e) => return Err(IpfsError::Other(e)),
        }
        println!("IPFS initialized successfully");
    }

    // Check if IPFS is already running
    let is_running = Command::new("pgrep")
        .arg("ipfs")
        .output()
        .map_err(|e| IpfsError::Other(e.to_string()))?;

    if is_running.status.success() {
        println!("IPFS is already running.");

        if is_ipfs_latest_version {
            return Ok(()); // If it's already running, don't try to start it again
        }

        // Stop the running IPFS daemon
        stop_ipfs().map_err(|e| IpfsError::Other(e))?;
    }

    let ipfs_binary = get_ipfs_binary_path()?;
    // Start the IPFS daemon
    Command::new(&ipfs_binary)
        .arg("daemon")
        .arg("--migrate=true")
        .spawn()
        .map_err(|e| IpfsError::Other(e.to_string()))?;
    println!("IPFS daemon started");

    // Give IPFS a moment to start up properly
    std::thread::sleep(std::time::Duration::from_secs(2));

    // Configure IPFS to allow all origins
    let config_output1 = Command::new(&ipfs_binary)
        .arg("config")
        .arg("--json")
        .arg("API.HTTPHeaders.Access-Control-Allow-Origin")
        .arg(r#"["*"]"#)
        .output()
        .map_err(|e| IpfsError::Other(e.to_string()))?;

    if !config_output1.status.success() {
        return Err(IpfsError::ConfigError(
            String::from_utf8_lossy(&config_output1.stderr).into(),
        ));
    }

    // Configure IPFS to allow specific HTTP methods
    let config_output2 = Command::new(&ipfs_binary)
        .arg("config")
        .arg("--json")
        .arg("API.HTTPHeaders.Access-Control-Allow-Methods")
        .arg(r#"["PUT", "POST", "GET"]"#)
        .output()
        .map_err(|e| IpfsError::Other(e.to_string()))?;

    if config_output2.status.success() {
        Ok(())
    } else {
        Err(IpfsError::ConfigError(
            String::from_utf8_lossy(&config_output2.stderr).into(),
        ))
    }
}

#[tauri::command]
pub fn stop_ipfs() -> Result<(), String> {
    let ipfs_binary = get_ipfs_binary_path().unwrap();

    Command::new(ipfs_binary)
        .arg("shutdown")
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn check_if_ipfs_exists() -> Result<bool, String> {
    let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
    let cyb_dir = home_dir.join(".cyb");
    let ipfs_binary = cyb_dir.join("kubo/ipfs"); // Adjust based on the extracted path
    let ipfs_exists = ipfs_binary.exists();

    Ok(ipfs_exists)
}

#[tauri::command]
pub fn is_ipfs_running() -> Result<bool, String> {
    let output = Command::new("pgrep")
        .arg("ipfs")
        .output()
        .map_err(|e| e.to_string())?;

    Ok(output.status.success())
}

#[tauri::command]
pub fn init_ipfs() -> Result<(), String> {
    let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
    let cyb_dir = home_dir.join(".cyb");
    let ipfs_binary = cyb_dir.join("kubo/ipfs"); // Adjust based on the extracted path

    let output = Command::new(&ipfs_binary)
        .arg("init")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into())
    }
}

#[tauri::command]
pub fn is_ipfs_initialized() -> Result<bool, String> {
    let home_dir = dirs::home_dir().ok_or("Cannot find home directory")?;
    let cyb_dir = home_dir.join(".cyb");
    let ipfs_binary = cyb_dir.join("kubo/ipfs"); // Adjust based on the extracted path

    let output = Command::new(&ipfs_binary)
        .arg("config")
        .arg("show")
        .output()
        .map_err(|e| e.to_string())?;

    Ok(output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    // use reqwest;

    // async fn fetch_data() -> Result<String, reqwest::Error> {
    //     let response = reqwest::get("https://dist.ipfs.tech/kubo/versions").await?;
    //     let body = response.text().await?;
    //     Ok(body)
    // }

    // #[tokio::test]
    // async fn test_fetch_data() {
    //     match fetch_data().await {
    //         Ok(data) => println!("Response: {}", data),
    //         Err(e) => eprintln!("Error fetching data: {}", e),
    //     }
    // }

    #[tokio::test]
    async fn test_get_latest_version() {
        match get_latest_version().await {
            Ok(data) => println!("Latest version: {}", data),
            Err(e) => eprintln!("Failed fetch latest version: {}", e),
        }
    }

    #[test]
    fn test_get_ipfs_download_url() {
        let version = "v0.32.1";
        let url = get_ipfs_download_url(version).unwrap();
        println!("URL: {}", url);
        assert!(!url.is_empty());
        assert_eq!(
            url,
            "https://dist.ipfs.io/kubo/v0.32.1/kubo_v0.32.1_darwin-arm64.tar.gz"
        );
    }

    #[test]
    fn test_get_installed_version() {
        let version = get_installed_version().unwrap();
        assert!(!version.is_empty());
        assert_eq!(version, "v0.29.0");
    }

    // #[test]
    // fn test_get_ipfs_binary_path() {
    //     let path = get_ipfs_binary_path().unwrap();
    //     assert!(!path.is_empty());
    // }

    // #[test]
    // fn test_start_ipfs() {
    //     let result = start_ipfs();
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_stop_ipfs() {
    //     let result = stop_ipfs();
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_check_ipfs() {
    //     let result = check_ipfs();
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_is_ipfs_running() {
    //     let result = is_ipfs_running();
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_init_ipfs() {
    //     let result = init_ipfs();
    //     assert!(result.is_ok());
    // }

    // #[test]
    // fn test_is_ipfs_initialized() {
    //     let result = is_ipfs_initialized();
    //     assert!(result.is_ok());
    // }
}
