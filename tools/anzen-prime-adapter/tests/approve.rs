use serde_json::Value;
use std::{fs, process::Command};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/policy-package-v4/regtest-proposal.json"
);
const SWEEP_FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/cooperative-sweep-v1/regtest-proposal.json"
);
const DEVELOPMENT_SEED: &str = "4242424242424242424242424242424242424242424242424242424242424242";

#[test]
fn approves_a_valid_file_atomically_without_printing_the_seed() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("approved-policy.json");
    let result = Command::new(env!("CARGO_BIN_EXE_anzen-prime-adapter"))
        .args(["approve", FIXTURE])
        .arg(&output)
        .arg(DEVELOPMENT_SEED)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!String::from_utf8_lossy(&result.stdout).contains(DEVELOPMENT_SEED));
    assert!(!String::from_utf8_lossy(&result.stderr).contains(DEVELOPMENT_SEED));
    let value: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(value["version"], 4);
    assert_eq!(value["manifest"]["hww_approved"], true);
    assert_eq!(value["psbts"].as_object().unwrap().len(), 28);
}

#[test]
fn invalid_identity_fails_without_creating_an_output_file() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("approved-policy.json");
    let result = Command::new(env!("CARGO_BIN_EXE_anzen-prime-adapter"))
        .args(["approve", FIXTURE])
        .arg(&output)
        .arg("4343434343434343434343434343434343434343434343434343434343434343")
        .output()
        .unwrap();

    assert!(!result.status.success());
    assert!(!output.exists());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("HWW key does not match vault descriptor"));
    assert!(!stderr.contains("4343434343434343"));
}

#[test]
fn approves_a_luke_compatible_cooperative_sweep_without_printing_the_seed() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("approved-sweep.json");
    let result = Command::new(env!("CARGO_BIN_EXE_anzen-prime-adapter"))
        .args(["approve-sweep", SWEEP_FIXTURE])
        .arg(&output)
        .arg(DEVELOPMENT_SEED)
        .output()
        .unwrap();

    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!String::from_utf8_lossy(&result.stdout).contains(DEVELOPMENT_SEED));
    assert!(!String::from_utf8_lossy(&result.stderr).contains(DEVELOPMENT_SEED));
    let value: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
    assert_eq!(value["version"], 1);
    assert_eq!(value["kind"], "cooperative-sweep");
    assert_eq!(value["phone_approved"], true);
    assert_eq!(value["hww_approved"], true);
}
