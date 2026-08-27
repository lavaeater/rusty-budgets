use dioxus::prelude::*;

pub const GIT_VERSION: &str = env!("GIT_VERSION");
pub const GIT_HASH: &str = env!("GIT_HASH");
pub const BUILD_TIME: &str = env!("BUILD_TIME");

/// Small `vX.Y.Z` (or commit hash, if untagged) badge — the release running
/// in this build, sourced from the git tag at compile time by `build.rs`.
#[component]
pub fn VersionBadge() -> Element {
    rsx! {
        span {
            class: "version-badge",
            title: "Commit {GIT_HASH} · Byggd {BUILD_TIME}",
            "v{GIT_VERSION}"
        }
    }
}
