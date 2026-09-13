//! CLI golden for default risk analysis JSON output.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn risk_format_json_matches_fixture_oracle() {
    let bin = env!("CARGO_BIN_EXE_hotspots");
    let log = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/risk_mini.log");
    let output = Command::new(bin)
        .arg("-l")
        .arg(&log)
        .args(["-c", "git2", "-a", "risk", "-n", "1", "--format", "json"])
        .output();
    assert!(output.is_ok(), "failed to spawn hotspots");
    if let Ok(output) = output {
        assert!(
            output.status.success(),
            "hotspots failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected = "[\n  {\"entity\": \"src/a.rs\", \"risk\": \"80.00\", \"revs\": \"3\", \"churn\": \"7\", \"soc\": \"0\", \"fragmentation\": \"44.44\"}\n]\n";
        assert_eq!(stdout.as_ref(), expected);
    }
}
