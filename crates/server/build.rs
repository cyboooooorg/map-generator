//! Records the build id of the server as `MAPGEN_BUILD_ID`, read by
//! `index::GENERATOR_VERSION`: `<crate version>+<git commit>`.
//!
//! The commit is, in order:
//!
//! 1. the `GIT_SHA` environment variable (the Docker build passes it as a
//!    build argument, the image has no `.git`);
//! 2. `git rev-parse` in the checkout;
//! 3. none: `MAPGEN_BUILD_ID` is not set and the generator version is the
//!    bare crate version.
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=GIT_SHA");
    let sha = std::env::var("GIT_SHA")
        .ok()
        .map(|sha| sha.trim().to_owned())
        .filter(|sha| !sha.is_empty())
        .or_else(git_head);
    if let Some(sha) = sha {
        let short: String = sha.chars().take(12).collect();
        let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
        println!("cargo:rustc-env=MAPGEN_BUILD_ID={version}+{short}");
    }
}

/// The checkout's current commit; also asks Cargo to build again when it
/// moves.
fn git_head() -> Option<String> {
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git").args(args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let sha = git(&["rev-parse", "HEAD"])?;
    // Only files that exist: a path that does not (a packed ref) would
    // make Cargo run this script on every build.
    let watch = |path: Option<String>| {
        if let Some(path) = path.filter(|p| std::path::Path::new(p).exists()) {
            println!("cargo:rerun-if-changed={path}");
        }
    };
    watch(git(&["rev-parse", "--git-path", "HEAD"]));
    if let Some(branch) = git(&["symbolic-ref", "-q", "HEAD"]) {
        watch(git(&["rev-parse", "--git-path", &branch]));
    }
    Some(sha)
}
