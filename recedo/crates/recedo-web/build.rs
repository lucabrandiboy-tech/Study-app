fn main() {
    // Commit the build came from, with "-dirty" if the working tree had uncommitted changes,
    // so a user's report always says exactly which code produced it.
    let hash = std::process::Command::new("git")
        .args(["describe", "--always", "--dirty", "--abbrev=10", "--match=NO-TAGS"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=RECEDO_GIT_HASH={hash}");
    // Rebuild the stamp when HEAD moves or the index changes (commits, staging).
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../../.git/index");
    println!("cargo:rerun-if-changed=../../../.git/refs/heads");
    // ...and when this crate's sources change, so "-dirty" appears or disappears.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=../recedo-app/src");
}
