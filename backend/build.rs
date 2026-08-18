use std::process::Command;

// ADR-0078: embeds build provenance (commit SHA + commit timestamp) as
// compile-time constants so a running binary can log which commit it was
// built from -- a stale container becomes visible in `podman compose logs`
// instead of requiring a manual `podman inspect` comparison. Git being
// unavailable (e.g. a build context with no .git) degrades to "unknown"
// rather than failing the build; this is a diagnostic aid, not a
// requirement.
fn git_output(args: &[&str]) -> Option<String> {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn main() {
    let git_sha =
        git_output(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".to_string());
    let git_commit_time =
        git_output(&["log", "-1", "--format=%cI"]).unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=RINGMASTER_GIT_SHA={git_sha}");
    println!("cargo:rustc-env=RINGMASTER_GIT_COMMIT_TIME={git_commit_time}");
    println!("cargo:rerun-if-changed=../.git/HEAD");
}
