use serde::Serialize;

#[cfg(desktop)]
use std::path::PathBuf;
#[cfg(desktop)]
use std::process::{Command, Stdio};
#[cfg(desktop)]
use std::sync::Mutex;
#[cfg(desktop)]
use std::process::Child;

#[derive(Debug, Serialize)]
pub enum IpfsError {
    HomeDirNotFound,
    Other(String),
}

/// Returns "local" on desktop (Kubo daemon) or "gateway" on mobile (cybernode)
#[tauri::command]
pub fn get_ipfs_mode() -> String {
    if cfg!(desktop) {
        "local".to_string()
    } else {
        "gateway".to_string()
    }
}

// ============================================================================
// Desktop: full Kubo daemon lifecycle
// ============================================================================

#[cfg(desktop)]
/// Returns the IPFS repo path (~/.cyb/ipfs-repo)
fn get_ipfs_repo_path() -> Result<PathBuf, IpfsError> {
    let home_dir = dirs::home_dir().ok_or(IpfsError::HomeDirNotFound)?;
    Ok(home_dir.join(".cyb").join("ipfs-repo"))
}

#[cfg(desktop)]
/// Resolves the bundled Kubo sidecar binary path.
/// In production bundles, Tauri strips the target triple: binary is just "ipfs".
/// In development, binary is in src-tauri/bin/ with -{target_triple} suffix.
fn get_ipfs_binary_path() -> Result<PathBuf, IpfsError> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| IpfsError::Other(format!("Cannot find current exe: {}", e)))?
        .parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| IpfsError::Other("Cannot find exe directory".into()))?;

    let target_triple = get_target_triple();
    let suffixed = format!("ipfs-{}", target_triple);

    // Production bundle: Tauri strips target triple, binary is just "ipfs" next to app
    let prod_plain = exe_dir.join("ipfs");
    if prod_plain.exists() {
        return Ok(prod_plain);
    }

    // Production with suffix (some Tauri versions keep it)
    let prod_suffixed = exe_dir.join(&suffixed);
    if prod_suffixed.exists() {
        return Ok(prod_suffixed);
    }

    // Development: binary is in src-tauri/bin/ with suffix
    let dev_candidates = [
        exe_dir.join("../../bin").join(&suffixed),
        exe_dir.join("../../../src-tauri/bin").join(&suffixed),
    ];
    for candidate in &dev_candidates {
        if let Ok(canonical) = candidate.canonicalize() {
            if canonical.exists() {
                return Ok(canonical);
            }
        }
    }

    Err(IpfsError::Other(format!(
        "Kubo binary not found. Looked for ipfs / {} in {:?}",
        suffixed, exe_dir
    )))
}

#[cfg(desktop)]
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

/// Holds the IPFS daemon child process so it isn't dropped (which closes
/// stdout/stderr pipes and can cause SIGPIPE → daemon crash).
#[cfg(desktop)]
static IPFS_CHILD: Mutex<Option<Child>> = Mutex::new(None);

#[cfg(desktop)]
fn spawn_ipfs_daemon(ipfs_binary: &PathBuf, repo_str: &str) -> Result<(), IpfsError> {
    let child = Command::new(ipfs_binary)
        .env("IPFS_PATH", repo_str)
        .arg("daemon")
        .arg("--migrate=true")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| IpfsError::Other(e.to_string()))?;

    println!("[IPFS] Daemon spawned (pid {})", child.id());

    if let Ok(mut guard) = IPFS_CHILD.lock() {
        *guard = Some(child);
    }
    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
pub async fn start_ipfs() -> Result<(), IpfsError> {
    println!("[IPFS] Starting IPFS daemon (bundled sidecar)");

    let ipfs_binary = get_ipfs_binary_path()?;
    let repo_path = get_ipfs_repo_path()?;
    let repo_str = repo_path.to_string_lossy().to_string();

    // Ensure repo directory exists
    let _ = std::fs::create_dir_all(&repo_path);

    // Check if IPFS is initialized
    if !is_ipfs_initialized_inner(&ipfs_binary, &repo_str) {
        println!("[IPFS] Initializing IPFS repo at {}", repo_str);
        init_ipfs_inner(&ipfs_binary, &repo_str).map_err(IpfsError::Other)?;
    }

    // Configure CORS before starting daemon
    let _ = Command::new(&ipfs_binary)
        .env("IPFS_PATH", &repo_str)
        .arg("config")
        .arg("--json")
        .arg("API.HTTPHeaders.Access-Control-Allow-Origin")
        .arg(r#"["*"]"#)
        .output();

    let _ = Command::new(&ipfs_binary)
        .env("IPFS_PATH", &repo_str)
        .arg("config")
        .arg("--json")
        .arg("API.HTTPHeaders.Access-Control-Allow-Methods")
        .arg(r#"["PUT", "POST", "GET"]"#)
        .output();

    // Check if already running
    if is_ipfs_running_inner() {
        println!("[IPFS] Daemon is already running");
        return Ok(());
    }

    spawn_ipfs_daemon(&ipfs_binary, &repo_str)?;

    // Spawn a watchdog that restarts the daemon if it exits unexpectedly
    let binary_clone = ipfs_binary.clone();
    let repo_clone = repo_str.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

            // Check if child process is still alive
            let exited = if let Ok(mut guard) = IPFS_CHILD.lock() {
                match guard.as_mut() {
                    Some(child) => match child.try_wait() {
                        Ok(Some(status)) => {
                            println!("[IPFS] Daemon exited with {status}, restarting...");
                            true
                        }
                        Ok(None) => false, // still running
                        Err(e) => {
                            println!("[IPFS] Failed to check daemon status: {e}");
                            false
                        }
                    },
                    None => {
                        // No child handle but check via pgrep
                        !is_ipfs_running_inner()
                    }
                }
            } else {
                false
            };

            if exited {
                if let Err(e) = spawn_ipfs_daemon(&binary_clone, &repo_clone) {
                    eprintln!("[IPFS] Watchdog: failed to restart daemon: {:?}", e);
                }
            }
        }
    });

    println!("[IPFS] Daemon spawned with watchdog");
    Ok(())
}

#[cfg(desktop)]
#[tauri::command]
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

#[cfg(desktop)]
#[tauri::command]
pub fn is_ipfs_running() -> Result<bool, String> {
    Ok(is_ipfs_running_inner())
}

#[cfg(desktop)]
fn is_ipfs_running_inner() -> bool {
    let output = Command::new("pgrep")
        .arg("-f")
        .arg("ipfs daemon")
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}

#[cfg(desktop)]
#[tauri::command]
pub fn init_ipfs() -> Result<(), String> {
    let ipfs_binary = get_ipfs_binary_path().map_err(|e| format!("{:?}", e))?;
    let repo_path = get_ipfs_repo_path().map_err(|e| format!("{:?}", e))?;
    init_ipfs_inner(&ipfs_binary, &repo_path.to_string_lossy())
}

#[cfg(desktop)]
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
        // "already initialized" is not an error
        if stderr.contains("already") {
            Ok(())
        } else {
            Err(stderr.into_owned())
        }
    }
}

#[cfg(desktop)]
#[tauri::command]
pub fn is_ipfs_initialized() -> Result<bool, String> {
    let ipfs_binary = get_ipfs_binary_path().map_err(|e| format!("{:?}", e))?;
    let repo_path = get_ipfs_repo_path().map_err(|e| format!("{:?}", e))?;
    Ok(is_ipfs_initialized_inner(
        &ipfs_binary,
        &repo_path.to_string_lossy(),
    ))
}

#[cfg(desktop)]
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

// ============================================================================
// Mobile: no-op stubs (IPFS uses remote gateway on mobile)
// ============================================================================

#[cfg(not(desktop))]
#[tauri::command]
pub async fn start_ipfs() -> Result<(), IpfsError> {
    println!("[IPFS] Mobile platform — using remote gateway, daemon not needed");
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
pub fn stop_ipfs() -> Result<(), String> {
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
pub fn is_ipfs_running() -> Result<bool, String> {
    // On mobile, "running" means the gateway is available (always true)
    Ok(true)
}

#[cfg(not(desktop))]
#[tauri::command]
pub fn init_ipfs() -> Result<(), String> {
    Ok(())
}

#[cfg(not(desktop))]
#[tauri::command]
pub fn is_ipfs_initialized() -> Result<bool, String> {
    Ok(true)
}
