fn main() {
    // Pass git describe output as GIT_VERSION env var at compile time.
    // Omit --dirty in CI since the build process may touch tracked files.
    let in_ci = std::env::var("CI").is_ok();
    let args = if in_ci {
        vec!["describe", "--tags", "--always"]
    } else {
        vec!["describe", "--tags", "--dirty", "--always"]
    };
    let version = std::process::Command::new("git")
        .args(&args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=GIT_VERSION={}", version.trim());

    tauri_build::build()
}
