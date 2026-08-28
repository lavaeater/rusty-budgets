use std::process::Command;

fn main() {
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| "unknown".to_string(), |s| s.trim().to_string());

    // Git tag if on one, else `git describe`'s best guess.
    let git_version = Command::new("git")
        .args(["describe", "--tags", "--always"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map_or_else(|| git_hash.clone(), |s| s.trim().to_string());

    let build_time = chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();

    let is_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .is_some_and(|o| !o.stdout.is_empty());

    let version_suffix = if is_dirty { "-dirty" } else { "" };

    println!("cargo:rustc-env=GIT_HASH={git_hash}");
    println!("cargo:rustc-env=GIT_VERSION={git_version}{version_suffix}");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads/");
    println!("cargo:rerun-if-changed=.git/refs/tags/");
}
