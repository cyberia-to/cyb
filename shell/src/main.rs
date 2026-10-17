fn main() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let log = std::path::PathBuf::from(home).join("cyb").join("panic.log");
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("{info}\n");
        eprintln!("cyb panic: {text}");
        let _ = std::fs::create_dir_all(log.parent().unwrap_or(std::path::Path::new(".")));
        let _ = std::fs::write(&log, &text);
    }));
    cyb::app::build_app().run();
}
