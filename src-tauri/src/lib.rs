#[cfg(desktop)]
mod db;
#[cfg(desktop)]
mod ipfs;
mod mining;
#[cfg(desktop)]
mod server;
#[cfg(desktop)]
mod utils;

use std::sync::Arc;

#[cfg(desktop)]
use db::DbState;
#[cfg(desktop)]
use ipfs::{
    check_if_ipfs_exists, download_and_extract_ipfs, init_ipfs, is_ipfs_initialized,
    is_ipfs_running, start_ipfs, stop_ipfs,
};
use mining::MiningState;
#[cfg(desktop)]
use server::start_server;
use tauri::generate_handler;
#[cfg(desktop)]
use tauri::{Manager, WebviewWindow};

#[cfg(desktop)]
use std::process::Command;
#[cfg(desktop)]
use utils::update_splash_message;

#[cfg(desktop)]
async fn init_ipfs_with_progress(splash: &WebviewWindow) {
    // Step 1: Check if binary exists
    update_splash_message(splash, "Checking IPFS installation...");
    println!("[CYB.AI] Step 1: Checking if IPFS binary exists...");

    let is_installed = check_if_ipfs_exists().await.unwrap_or(false);
    println!("[CYB.AI] IPFS installed: {}", is_installed);

    // Step 2: If IPFS is installed, skip version check (avoid slow network request on every launch)
    let needs_download = !is_installed;

    // Step 3: Download if needed
    if needs_download {
        update_splash_message(splash, "Downloading IPFS (this may take a minute)...");
        println!("[CYB.AI] Step 3: Downloading IPFS...");

        match download_and_extract_ipfs().await {
            Ok(_) => println!("[CYB.AI] IPFS downloaded and extracted"),
            Err(e) => {
                eprintln!("[CYB.AI] Failed to download IPFS: {}", e);
                update_splash_message(splash, &format!("IPFS download failed: {}", e));
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                return;
            }
        }
    }

    // Step 4: Initialize IPFS repo if needed
    update_splash_message(splash, "Initializing IPFS repo...");
    println!("[CYB.AI] Step 4: Checking if IPFS is initialized...");

    let is_init = is_ipfs_initialized().unwrap_or(false);
    if !is_init {
        println!("[CYB.AI] Initializing IPFS repo...");
        if let Err(e) = init_ipfs() {
            eprintln!("[CYB.AI] Failed to init IPFS: {}", e);
            update_splash_message(splash, &format!("IPFS init failed: {}", e));
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            return;
        }
    }

    // Step 5: Configure CORS BEFORE starting daemon (daemon reads config at startup)
    let home_dir = match dirs::home_dir() {
        Some(d) => d,
        None => {
            update_splash_message(splash, "Cannot find home directory");
            return;
        }
    };
    let ipfs_binary = home_dir.join(".cyb/kubo/ipfs");

    update_splash_message(splash, "Configuring IPFS...");
    println!("[CYB.AI] Step 5: Configuring IPFS CORS (before daemon start)...");

    let _ = Command::new(&ipfs_binary)
        .arg("config")
        .arg("--json")
        .arg("API.HTTPHeaders.Access-Control-Allow-Origin")
        .arg(r#"["*"]"#)
        .output();

    let _ = Command::new(&ipfs_binary)
        .arg("config")
        .arg("--json")
        .arg("API.HTTPHeaders.Access-Control-Allow-Methods")
        .arg(r#"["PUT", "POST", "GET"]"#)
        .output();

    // Step 6: Check if already running — if so, reuse it (CORS config persists)
    update_splash_message(splash, "Starting IPFS daemon...");
    println!("[CYB.AI] Step 6: Checking if IPFS daemon is already running...");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

    let already_responding = match client.post("http://127.0.0.1:5001/api/v0/id").send().await {
        Ok(resp) => resp.status().is_success(),
        _ => false,
    };

    if already_responding {
        println!("[CYB.AI] IPFS daemon is already running and responding!");
        update_splash_message(splash, "IPFS is running");
    } else {
        // Kill any zombie ipfs processes that aren't responding
        let _ = Command::new("pkill").arg("-f").arg("ipfs daemon").output();
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Step 7: Start daemon (fresh start with correct CORS config)
        println!("[CYB.AI] Step 7: Spawning IPFS daemon...");
        match Command::new(&ipfs_binary)
            .arg("daemon")
            .arg("--migrate=true")
            .spawn()
        {
            Ok(_) => println!("[CYB.AI] IPFS daemon spawned"),
            Err(e) => {
                eprintln!("[CYB.AI] Failed to spawn IPFS daemon: {}", e);
                update_splash_message(splash, &format!("IPFS daemon failed: {}", e));
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                return;
            }
        }

        // Step 8: Wait for daemon API to be ready
        update_splash_message(splash, "Waiting for IPFS daemon...");
        println!("[CYB.AI] Step 8: Waiting for daemon API...");

        let max_attempts = 15;
        for i in 0..max_attempts {
            match client.post("http://127.0.0.1:5001/api/v0/id").send().await {
                Ok(resp) if resp.status().is_success() => {
                    println!("[CYB.AI] IPFS API is ready!");
                    break;
                }
                _ => {
                    if i == max_attempts - 1 {
                        eprintln!("[CYB.AI] IPFS API did not become ready in time");
                        update_splash_message(splash, "IPFS daemon slow to start, continuing...");
                    } else {
                        println!("[CYB.AI] Waiting for IPFS API... ({}/{})", i + 1, max_attempts);
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }
                }
            }
        }
    }

    update_splash_message(splash, "IPFS started successfully!");
    println!("[CYB.AI] IPFS initialization complete!");
}

#[cfg(desktop)]
fn build_tauri_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(MiningState::new()))
        .invoke_handler(generate_handler![
            download_and_extract_ipfs,
            start_ipfs,
            stop_ipfs,
            check_if_ipfs_exists,
            is_ipfs_running,
            is_ipfs_initialized,
            init_ipfs,
            mining::start_mining,
            mining::stop_mining,
            mining::get_mining_status,
            mining::take_proofs,
            mining::mining_benchmark,
            mining::get_mining_params,
        ])
}

#[cfg(not(desktop))]
fn build_tauri_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(MiningState::new()))
        .invoke_handler(generate_handler![
            mining::start_mining,
            mining::stop_mining,
            mining::get_mining_status,
            mining::take_proofs,
            mining::mining_benchmark,
            mining::get_mining_params,
        ])
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(desktop)]
    {
        let app_state = Arc::new(DbState::new());

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let server_state = app_state.clone();
                tokio::spawn(async move {
                    println!("[CYB.AI] Starting server...");
                    start_server(server_state).await;
                    println!("[CYB.AI] Server is started!");
                });

                build_tauri_app()
                    .setup(|app| {
                        println!("[CYB.AI] Starting setup...");

                        let splashscreen_window =
                            app.get_webview_window("splashscreen").unwrap();
                        splashscreen_window.show().unwrap();

                        let main_window = app.get_webview_window("main").unwrap();

                        tauri::async_runtime::spawn(async move {
                            init_ipfs_with_progress(&splashscreen_window).await;

                            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                            println!("[CYB.AI] Showing main window...");
                            main_window.show().unwrap();
                            #[cfg(debug_assertions)]
                            main_window.open_devtools();
                            let _ = splashscreen_window.close();
                            println!("[CYB.AI] App ready!");
                        });

                        Ok(())
                    })
                    .run(tauri::generate_context!())
                    .expect("error while running tauri application");
            });
    }

    #[cfg(not(desktop))]
    {
        println!("[CYB.AI] Mobile platform startup");
        build_tauri_app()
            .setup(|_app| {
                println!("[CYB.AI] App ready!");
                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
