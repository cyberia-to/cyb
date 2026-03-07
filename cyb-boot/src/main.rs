use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const PKG_URL: &str = "https://github.com/cyberia-to/cyb/releases/latest/download/cyb.pkg";
const APP_DATA_DIR: &str = "ai.cyb.app";
const BOOT_DAT: &str = "boot.dat";

#[derive(Serialize, Deserialize)]
struct Bootstrap {
    mnemonic: String,
    referrer: String,
}

fn get_app_data_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join(APP_DATA_DIR))
}

/// Find boot.dat next to the executable
fn find_boot_dat() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let path = dir.join(BOOT_DAT);
    if path.exists() { Some(path) } else { None }
}

fn decrypt_boot_dat(path: &PathBuf) -> Result<Bootstrap, String> {
    let data = fs::read(path)
        .map_err(|e| format!("Cannot read boot.dat: {e}"))?;

    // Data layout: key[32] || iv[12] || ct_len[2 big-endian] || ciphertext[ct_len]
    if data.len() < 32 + 12 + 2 + 1 {
        return Err("boot.dat too short".into());
    }

    let key_bytes = &data[..32];
    let iv_bytes = &data[32..44];
    let ct_len = ((data[44] as usize) << 8) | (data[45] as usize);
    if ct_len == 0 || 46 + ct_len > data.len() {
        return Err(format!("Invalid ciphertext length: {ct_len}").into());
    }
    let ciphertext = &data[46..46 + ct_len];

    let cipher = Aes256Gcm::new_from_slice(key_bytes)
        .map_err(|e| format!("Invalid key: {e}"))?;
    let nonce = Nonce::from_slice(iv_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {e}"))?;

    serde_json::from_slice(&plaintext)
        .map_err(|e| format!("Invalid bootstrap data: {e}"))
}

fn write_bootstrap(bootstrap: &Bootstrap) -> Result<PathBuf, String> {
    let dir = get_app_data_dir()
        .ok_or("Cannot determine app data directory")?;

    fs::create_dir_all(&dir)
        .map_err(|e| format!("Cannot create app data dir: {e}"))?;

    let path = dir.join("bootstrap.json");
    let json = serde_json::to_string_pretty(bootstrap)
        .map_err(|e| format!("JSON serialize error: {e}"))?;

    fs::write(&path, json)
        .map_err(|e| format!("Cannot write bootstrap.json: {e}"))?;

    Ok(path)
}

fn download_and_open_installer() -> Result<(), String> {
    println!("Downloading cyb installer...");

    let tmp_dir = std::env::temp_dir();
    let pkg_path = tmp_dir.join("cyb.pkg");

    let response = reqwest::blocking::get(PKG_URL)
        .map_err(|e| format!("Download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Download returned status: {}", response.status()));
    }

    let bytes = response.bytes()
        .map_err(|e| format!("Failed to read download: {e}"))?;

    fs::write(&pkg_path, &bytes)
        .map_err(|e| format!("Cannot save installer: {e}"))?;

    println!("Installer saved to {:?}", pkg_path);
    println!("Opening installer...");

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(&pkg_path)
            .spawn()
            .map_err(|e| format!("Cannot open installer: {e}"))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &pkg_path.to_string_lossy()])
            .spawn()
            .map_err(|e| format!("Cannot open installer: {e}"))?;
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(&pkg_path)
            .spawn()
            .map_err(|e| format!("Cannot open installer: {e}"))?;
    }

    Ok(())
}

fn main() {
    println!("cyb-boot — setting up your mining wallet...\n");

    // Step 1: Read boot.dat from same directory as executable
    match find_boot_dat() {
        Some(dat_path) => {
            match decrypt_boot_dat(&dat_path) {
                Ok(bootstrap) => {
                    match write_bootstrap(&bootstrap) {
                        Ok(path) => {
                            println!("Wallet data saved to {:?}", path);
                            // Clean up boot.dat after successful import
                            let _ = fs::remove_file(&dat_path);
                        }
                        Err(e) => {
                            eprintln!("Error writing bootstrap: {e}");
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error decrypting boot.dat: {e}");
                    println!("Proceeding with installer download only...\n");
                }
            }
        }
        None => {
            println!("No boot.dat found — proceeding with installer download only...\n");
        }
    }

    // Step 2: Download and open installer
    match download_and_open_installer() {
        Ok(()) => {
            println!("\nAfter installation, open cyb to start mining!");
        }
        Err(e) => {
            eprintln!("Error: {e}");
            eprintln!("\nYou can manually download cyb from:");
            eprintln!("  {PKG_URL}");
            std::process::exit(1);
        }
    }
}
