#[cfg(desktop)]
mod db;
mod ipfs;
mod mining;
#[cfg(desktop)]
mod server;
#[cfg(desktop)]
mod utils;

use std::sync::Arc;

#[cfg(desktop)]
use db::DbState;
use ipfs::{get_ipfs_mode, init_ipfs, is_ipfs_initialized, is_ipfs_running, start_ipfs, stop_ipfs};
use mining::MiningState;
#[cfg(desktop)]
use server::start_server;
use tauri::generate_handler;
use tauri::Manager;
#[cfg(desktop)]
use tauri::WebviewWindow;

#[cfg(desktop)]
use utils::update_splash_message;

#[cfg(desktop)]
async fn init_ipfs_with_progress(splash: &WebviewWindow) {
    // Step 1: Initialize IPFS repo if needed
    update_splash_message(splash, "Initializing IPFS...");
    println!("[CYB.AI] Step 1: Checking if IPFS is initialized...");

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

    // Step 2: Start the daemon (handles CORS config internally)
    update_splash_message(splash, "Starting IPFS daemon...");
    println!("[CYB.AI] Step 2: Starting IPFS daemon...");

    match start_ipfs().await {
        Ok(_) => println!("[CYB.AI] IPFS daemon started"),
        Err(e) => {
            eprintln!("[CYB.AI] Failed to start IPFS: {:?}", e);
            update_splash_message(splash, &format!("IPFS start failed: {:?}", e));
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            return;
        }
    }

    // Step 3: Wait for daemon API to be ready
    update_splash_message(splash, "Waiting for IPFS daemon...");
    println!("[CYB.AI] Step 3: Waiting for daemon API...");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();

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
                    println!(
                        "[CYB.AI] Waiting for IPFS API... ({}/{})",
                        i + 1,
                        max_attempts
                    );
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    update_splash_message(splash, "IPFS started successfully!");
    println!("[CYB.AI] IPFS initialization complete!");
}

#[tauri::command]
fn toggle_devtools(window: tauri::WebviewWindow) {
    if window.is_devtools_open() {
        window.close_devtools();
    } else {
        window.open_devtools();
    }
}

#[cfg(desktop)]
fn build_tauri_app() -> tauri::Builder<tauri::Wry> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(MiningState::new()))
        .invoke_handler(generate_handler![
            toggle_devtools,
            get_ipfs_mode,
            start_ipfs,
            stop_ipfs,
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
            toggle_devtools,
            get_ipfs_mode,
            start_ipfs,
            stop_ipfs,
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
                            if cfg!(debug_assertions) {
                                main_window.open_devtools();
                            }
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
