fn main() {
  // Tauri embeds ../build/ into the binary (frontendDist),
  // but Cargo doesn't detect changes there without this.
  println!("cargo:rerun-if-changed=../build");

  tauri_build::build()
}
