use tauri::WebviewWindow;

pub fn update_splash_message(window: &WebviewWindow, message: &str) {
    let _ = window.eval(&format!("updateSplashMessage('{}')", message));
}
