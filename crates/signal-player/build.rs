use std::process::Command;

// libmpv2-sys links `-lmpv` but emits no search path; resolve it here so
// linking works wherever pkg-config knows about mpv (brew, distro packages).
fn main() {
    let libdir = Command::new("pkg-config")
        .args(["--variable=libdir", "mpv"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());

    if let Some(dir) = libdir {
        println!("cargo:rustc-link-search=native={dir}");
    } else if cfg!(target_os = "macos") {
        // brew default when pkg-config is unavailable
        println!("cargo:rustc-link-search=native=/opt/homebrew/lib");
    }
}
