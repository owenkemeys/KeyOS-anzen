use anzen_policy_engine::{AnzenIdentity, PolicyError, PolicyPackage, MAX_PACKAGE_BYTES};
use serde_json::Value;
use std::str::FromStr;

const FIXTURE: &[u8] = include_bytes!("../../../fixtures/policy-package-v4/regtest-proposal.json");

fn expected_hww() -> bitcoin::secp256k1::XOnlyPublicKey {
    bitcoin::secp256k1::XOnlyPublicKey::from_str(
        "906979e124fa1d21104bbcfce84d825cb1269fa1dc95a0a27e9bfe120d88e97c",
    )
    .unwrap()
}

fn parse_value(value: &Value) -> PolicyPackage {
    PolicyPackage::parse_bounded(&serde_json::to_vec(value).unwrap()).unwrap()
}

fn rejects_policy(value: &Value) {
    assert!(matches!(
        parse_value(value).validate(expected_hww()),
        Err(PolicyError::InvalidPolicy(_))
    ));
}

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

#[test]
fn summarizes_every_owner_visible_policy_boundary_without_psbt_bodies() {
    let package = PolicyPackage::parse_bounded(FIXTURE).unwrap();
    let summary = package.summary().unwrap();

    assert_eq!(summary.network, "regtest");
    assert_eq!(
        summary.vault_address,
        "bcrt1ppc630jlwa8nykupxpam7rknknpx8mljnll04kjn4cst6zm20t7rslwygna"
    );
    assert_eq!(summary.total_input_sats, 200_000_000);
    assert_eq!(summary.monthly_limit_sats, 10_000_000);
    assert_eq!(summary.allowance_count, 12);
    assert_eq!(summary.allowance_delay_seconds, Some(2_592_256));
    assert_eq!(summary.emergency_access_limit_sats, 50_000_000);
    assert_eq!(summary.emergency_delay_seconds, Some(605_184));
    assert_eq!(summary.fee_rate_sat_vb, 1);
    assert_eq!(
        summary.rollover_txid,
        "303a2fdff90abc3f3ebed24d54ef0e230d5cf5fd4a41fa703ad44ce62293201d"
    );
    assert_eq!(summary.rollover_fee_sats, 205);
    assert_eq!(summary.allowance_value_sats, 120_002_417);
    assert_eq!(summary.remainder_value_sats, 79_997_378);
    assert_eq!(summary.phone_recovery_blocks, 61_200);
    assert_eq!(summary.hww_recovery_blocks, 65_535);
}

#[test]
fn independently_validates_all_28_psbts_and_phone_signatures() {
    let package = PolicyPackage::parse_bounded(FIXTURE).unwrap();
    let validated = package.validate(expected_hww()).unwrap();

    assert_eq!(validated.psbt_count(), 28);
}

#[test]
fn rejects_descriptor_recovery_or_hww_identity_changes() {
    let package = PolicyPackage::parse_bounded(FIXTURE).unwrap();
    let other_hww = bitcoin::secp256k1::XOnlyPublicKey::from_str(
        "79be667ef9dcbbac55a06295ce870b07029bfcdb2dce28d959f2815b16f81798",
    )
    .unwrap();
    assert!(matches!(
        package.validate(other_hww),
        Err(PolicyError::InvalidPolicy(_))
    ));

    let mut recovery: Value = serde_json::from_slice(FIXTURE).unwrap();
    let descriptor = recovery["manifest"]["vault_descriptor"]
        .as_str()
        .unwrap()
        .split('#')
        .next()
        .unwrap()
        .replace("older(65535)", "older(65534)");
    let checksum = miniscript::descriptor::checksum::desc_checksum(&descriptor).unwrap();
    recovery["manifest"]["vault_descriptor"] = format!("{descriptor}#{checksum}").into();
    rejects_policy(&recovery);
}

#[test]
fn rejects_amount_fee_and_transaction_graph_tampering() {
    let mut total: Value = serde_json::from_slice(FIXTURE).unwrap();
    total["manifest"]["total_input_sats"] = 199_999_999_u64.into();
    rejects_policy(&total);

    let mut fee: Value = serde_json::from_slice(FIXTURE).unwrap();
    fee["manifest"]["rollover"]["fee_sats"] = 206_u64.into();
    rejects_policy(&fee);

    let mut link: Value = serde_json::from_slice(FIXTURE).unwrap();
    link["manifest"]["allowances"][1]["authorization"]["unsigned_txid"] =
        "0000000000000000000000000000000000000000000000000000000000000000".into();
    rejects_policy(&link);
}

#[test]
fn rejects_allowance_or_emergency_destination_tampering() {
    let mut allowance: Value = serde_json::from_slice(FIXTURE).unwrap();
    allowance["manifest"]["allowances"][0]["hot_address"] =
        allowance["manifest"]["allowances"][1]["hot_address"].clone();
    rejects_policy(&allowance);

    let mut emergency: Value = serde_json::from_slice(FIXTURE).unwrap();
    emergency["manifest"]["emergency_access"]["hot_address"] =
        emergency["manifest"]["allowances"][0]["hot_address"].clone();
    rejects_policy(&emergency);
}

#[test]
fn rejects_missing_phone_signatures() {
    let mut value: Value = serde_json::from_slice(FIXTURE).unwrap();
    let encoded = value["psbts"]["rollover.psbt"].as_str().unwrap();
    let mut psbt = bitcoin::Psbt::from_str(encoded).unwrap();
    psbt.inputs[0].tap_script_sigs.clear();
    value["psbts"]["rollover.psbt"] = psbt.to_string().into();

    rejects_policy(&value);
}

#[test]
fn derives_lukes_vault_identity_from_the_prime_app_seed_vector() {
    let identity = AnzenIdentity::from_app_seed(&[0x42; 32], bitcoin::Network::Regtest).unwrap();

    assert_eq!(identity.public_key(), expected_hww());
}

#[test]
fn signs_every_validated_psbt_and_serializes_an_approved_v4_package() {
    let identity = AnzenIdentity::from_app_seed(&[0x42; 32], bitcoin::Network::Regtest).unwrap();
    let package = PolicyPackage::parse_bounded(FIXTURE).unwrap();
    let validated = package.validate(identity.public_key()).unwrap();

    let approved = validated.approve(&identity).unwrap();

    assert!(approved.hww_approved());
    assert_eq!(approved.hww_signature_count(), 28);
    let json = approved.to_json().unwrap();
    let reparsed = PolicyPackage::parse_bounded(&json).unwrap();
    assert_eq!(reparsed.version, 4);
    assert_eq!(reparsed.kind, "vault-policy");
    assert!(reparsed.manifest.hww_approved);
    assert_eq!(reparsed.psbts.len(), 28);
    let text = String::from_utf8(json).unwrap();
    for forbidden in ["mnemonic", "seed", "xpriv", "private_key"] {
        assert!(!text.to_ascii_lowercase().contains(forbidden));
    }
}

#[test]
fn refuses_to_sign_with_an_identity_outside_the_validated_descriptor() {
    let expected = AnzenIdentity::from_app_seed(&[0x42; 32], bitcoin::Network::Regtest).unwrap();
    let other = AnzenIdentity::from_app_seed(&[0x43; 32], bitcoin::Network::Regtest).unwrap();
    let validated = PolicyPackage::parse_bounded(FIXTURE)
        .unwrap()
        .validate(expected.public_key())
        .unwrap();

    assert!(matches!(
        validated.approve(&other),
        Err(PolicyError::Signing)
    ));
}
