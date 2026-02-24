use serde::Serialize;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Serialize)]
pub enum IpfsError {
    HomeDirNotFound,
    Other(String),
}

fn get_ipfs_repo_path() -> Result<PathBuf, IpfsError> {
    let home_dir = dirs::home_dir().ok_or(IpfsError::HomeDirNotFound)?;
    Ok(home_dir.join(".cyb").join("ipfs-repo"))
}

fn get_ipfs_binary_path() -> Result<PathBuf, IpfsError> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| IpfsError::Other(format!("Cannot find current exe: {}", e)))?
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| IpfsError::Other("Cannot find exe directory".into()))?;

    let target_triple = get_target_triple();
    let suffixed = format!("ipfs-{}", target_triple);

    let prod_plain = exe_dir.join("ipfs");
    if prod_plain.exists() {
        return Ok(prod_plain);
    }

    let prod_suffixed = exe_dir.join(&suffixed);
    if prod_suffixed.exists() {
        return Ok(prod_suffixed);
    }

    let dev_candidates = [
        exe_dir.join("../../bin").join(&suffixed),
        exe_dir.join("../../../bin").join(&suffixed),
    ];
    for candidate in &dev_candidates {
        if let Ok(canonical) = candidate.canonicalize() {
            if canonical.exists() {
                return Ok(canonical);
            }
        }
    }

    // Fallback: try system PATH
    if let Ok(output) = Command::new("which").arg("ipfs").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }

    Err(IpfsError::Other(format!(
        "Kubo binary not found. Looked for ipfs / {} in {:?}",
        suffixed, exe_dir
    )))
}

fn get_target_triple() -> &'static str {
    if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-apple-darwin"
        } else {
            "x86_64-apple-darwin"
        }
    } else if cfg!(target_os = "linux") {
        if cfg!(target_arch = "aarch64") {
            "aarch64-unknown-linux-gnu"
        } else {
            "x86_64-unknown-linux-gnu"
        }
    } else if cfg!(target_os = "windows") {
        "x86_64-pc-windows-msvc"
    } else {
        "unknown"
    }
}

pub async fn start_ipfs() -> Result<(), IpfsError> {
    println!("[IPFS] Starting IPFS daemon");

    let ipfs_binary = get_ipfs_binary_path()?;
    let repo_path = get_ipfs_repo_path()?;
    let repo_str = repo_path.to_string_lossy().to_string();

    let _ = std::fs::create_dir_all(&repo_path);

    if !is_ipfs_initialized_inner(&ipfs_binary, &repo_str) {
        println!("[IPFS] Initializing IPFS repo at {}", repo_str);
        init_ipfs_inner(&ipfs_binary, &repo_str).map_err(IpfsError::Other)?;
    }

    // Configure CORS
    let _ = Command::new(&ipfs_binary)
        .env("IPFS_PATH", &repo_str)
        .args(["config", "--json", "API.HTTPHeaders.Access-Control-Allow-Origin", r#"["*"]"#])
        .output();

    let _ = Command::new(&ipfs_binary)
        .env("IPFS_PATH", &repo_str)
        .args(["config", "--json", "API.HTTPHeaders.Access-Control-Allow-Methods", r#"["PUT", "POST", "GET"]"#])
        .output();

    if is_ipfs_running() {
        println!("[IPFS] Daemon is already running");
        return Ok(());
    }

    Command::new(&ipfs_binary)
        .env("IPFS_PATH", &repo_str)
        .args(["daemon", "--migrate=true"])
        .spawn()
        .map_err(|e| IpfsError::Other(e.to_string()))?;

    // Wait for daemon API to be ready
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    for i in 0..15 {
        match client.post("http://127.0.0.1:5001/api/v0/id").send().await {
            Ok(resp) if resp.status().is_success() => {
                println!("[IPFS] API is ready!");
                return Ok(());
            }
            _ => {
                if i < 14 {
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    println!("[IPFS] Daemon spawned (API may still be starting)");
    Ok(())
}

pub fn stop_ipfs() -> Result<(), String> {
    let ipfs_binary = get_ipfs_binary_path().map_err(|e| format!("{:?}", e))?;
    let repo_path = get_ipfs_repo_path().map_err(|e| format!("{:?}", e))?;

    Command::new(ipfs_binary)
        .env("IPFS_PATH", repo_path.to_string_lossy().as_ref())
        .arg("shutdown")
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn is_ipfs_running() -> bool {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("ipfs daemon")
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

fn init_ipfs_inner(ipfs_binary: &PathBuf, repo_path: &str) -> Result<(), String> {
    let output = Command::new(ipfs_binary)
        .env("IPFS_PATH", repo_path)
        .arg("init")
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("already") {
            Ok(())
        } else {
            Err(stderr.into_owned())
        }
    }
}

fn is_ipfs_initialized_inner(ipfs_binary: &PathBuf, repo_path: &str) -> bool {
    let output = Command::new(ipfs_binary)
        .env("IPFS_PATH", repo_path)
        .arg("config")
        .arg("show")
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}
