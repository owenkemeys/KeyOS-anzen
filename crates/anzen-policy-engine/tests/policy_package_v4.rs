use anzen_policy_engine::{PolicyError, PolicyPackage, MAX_PACKAGE_BYTES};
use serde_json::Value;

const FIXTURE: &[u8] = include_bytes!("../../../fixtures/policy-package-v4/regtest-proposal.json");

#[test]
fn parses_luke_policy_package_v4_and_preserves_the_exact_psbt_set() {
    let package = PolicyPackage::parse_bounded(FIXTURE).expect("fixture should parse");

    assert_eq!(package.version, 4);
    assert_eq!(package.kind, "vault-policy");
    assert_eq!(package.manifest.version, 4);
    assert_eq!(package.manifest.network, "regtest");
    assert_eq!(package.manifest.allowance_count, 12);
    assert_eq!(package.manifest.allowances.len(), 12);
    assert_eq!(package.psbts.len(), 28);
    assert!(package.manifest.phone_approved);
    assert!(!package.manifest.hww_approved);
}

#[test]
fn rejects_packages_over_the_import_limit_before_json_parsing() {
    let mut oversized = FIXTURE.to_vec();
    oversized.resize(MAX_PACKAGE_BYTES + 1, b' ');

    assert_eq!(
        PolicyPackage::parse_bounded(&oversized).unwrap_err(),
        PolicyError::PackageTooLarge
    );
}

#[test]
fn rejects_unsupported_package_versions() {
    let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
    value["version"] = 5.into();

    assert_eq!(
        PolicyPackage::parse_bounded(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
        PolicyError::UnsupportedPackage
    );
}

#[test]
fn rejects_unsafe_psbt_paths() {
    let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
    value["manifest"]["rollover"]["psbt_file"] = "../escape.psbt".into();

    assert_eq!(
        PolicyPackage::parse_bounded(&serde_json::to_vec(&value).unwrap()).unwrap_err(),
        PolicyError::UnsafePsbtPath
    );
}

#[test]
fn rejects_missing_or_extra_psbts() {
    let mut missing: Value = serde_json::from_slice(FIXTURE).unwrap();
    let psbts = missing["psbts"].as_object_mut().unwrap();
    let first = psbts.keys().next().unwrap().clone();
    psbts.remove(&first);
    assert_eq!(
        PolicyPackage::parse_bounded(&serde_json::to_vec(&missing).unwrap()).unwrap_err(),
        PolicyError::PsbtSetMismatch
    );

    let mut extra: Value = serde_json::from_slice(FIXTURE).unwrap();
    extra["psbts"]
        .as_object_mut()
        .unwrap()
        .insert("extra.psbt".to_owned(), "cHNidP8BAAoCAAAAAA==".into());
    assert_eq!(
        PolicyPackage::parse_bounded(&serde_json::to_vec(&extra).unwrap()).unwrap_err(),
        PolicyError::PsbtSetMismatch
    );
}
