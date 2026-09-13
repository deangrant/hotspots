//! Drive the `hotspots` binary so `main` is covered under llvm-cov.

use std::process::Command;

#[test]
fn help_exits_success() {
    let bin = env!("CARGO_BIN_EXE_hotspots");
    let output = Command::new(bin).args(["--help"]).output();
    assert!(output.is_ok(), "failed to spawn hotspots");
    if let Ok(output) = output {
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
    }
}

#[test]
fn missing_log_exits_failure() {
    let bin = env!("CARGO_BIN_EXE_hotspots");
    let output = Command::new(bin)
        .args(["-l", "/no/such/hotspots-cli-main.log", "-c", "git2"])
        .output();
    assert!(output.is_ok(), "failed to spawn hotspots");
    if let Ok(output) = output {
        assert!(!output.status.success());
        let err = String::from_utf8_lossy(&output.stderr);
        assert!(err.contains("error:"));
    }
}
