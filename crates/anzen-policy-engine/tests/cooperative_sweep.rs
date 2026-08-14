use anzen_policy_engine::{AnzenIdentity, CooperativeSweepPackage, PolicyError};
use bitcoin::{Network, Psbt};
use serde_json::{json, Value};
use std::str::FromStr;

const APP_SEED: [u8; 32] = [0x42; 32];

fn valid_sweep_json() -> Vec<u8> {
    let policy: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/policy-package-v4/regtest-proposal.json"
    ))
    .expect("policy fixture");
    let manifest = &policy["manifest"];
    let revocation = &manifest["allowances"][0]["revocation"];
    let psbt_file = revocation["psbt_file"].as_str().expect("PSBT path");
    let chain_value = manifest["allowances"][0]["chain_value_sats"]
        .as_u64()
        .expect("chain value");
    let fee = revocation["fee_sats"].as_u64().expect("fee");
    serde_json::to_vec_pretty(&json!({
        "version": 1,
        "kind": "cooperative-sweep",
        "vault_descriptor": manifest["vault_descriptor"],
        "destination": manifest["vault_address"],
        "psbt": policy["psbts"][psbt_file],
        "input_count": 1,
        "sent_sats": chain_value - fee,
        "fee_sats": fee,
        "phone_approved": true,
        "hww_approved": false
    }))
    .expect("sweep fixture")
}

fn identity() -> AnzenIdentity {
    AnzenIdentity::from_app_seed(&APP_SEED, Network::Regtest).expect("identity")
}

#[test]
fn parses_luke_v1_sweep_for_owner_review_without_seed_material() {
    let package = CooperativeSweepPackage::parse_bounded(&valid_sweep_json()).expect("parse");
    let summary = package.summary().expect("summary");

    assert_eq!(summary.network, "regtest");
    assert_eq!(summary.input_count, 1);
    assert_eq!(summary.sent_sats, 120_002_255);
    assert_eq!(summary.fee_sats, 162);
    assert!(summary.destination.starts_with("bcrt1p"));
    assert!(summary.phone_approved);
    assert!(!summary.hww_approved);
}

#[test]
fn independently_validates_phone_signature_and_every_sweep_boundary() {
    let package = CooperativeSweepPackage::parse_bounded(&valid_sweep_json()).expect("parse");
    let validated = package
        .validate(identity().public_key())
        .expect("valid sweep");

    assert_eq!(validated.input_count(), 1);
}

#[test]
fn rejects_destination_amount_fee_descriptor_and_approval_tampering() {
    for (field, replacement) in [
        (
            "destination",
            json!("bcrt1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq0l98cr"),
        ),
        ("sent_sats", json!(120_002_262_u64)),
        ("fee_sats", json!(153_u64)),
        ("vault_descriptor", json!("tr(00)")),
        ("phone_approved", json!(false)),
        ("hww_approved", json!(true)),
    ] {
        let mut value: Value = serde_json::from_slice(&valid_sweep_json()).expect("json");
        value[field] = replacement;
        let package = CooperativeSweepPackage::parse_bounded(
            &serde_json::to_vec(&value).expect("tampered JSON"),
        );
        let result = package.and_then(|package| package.validate(identity().public_key()));
        assert!(result.is_err(), "tampering {field} must fail");
    }
}

#[test]
fn rejects_missing_or_invalid_phone_signature() {
    let mut value: Value = serde_json::from_slice(&valid_sweep_json()).expect("json");
    let mut psbt = Psbt::from_str(value["psbt"].as_str().expect("PSBT")).expect("valid PSBT");
    psbt.inputs[0].tap_script_sigs.clear();
    value["psbt"] = json!(psbt.to_string());
    let package =
        CooperativeSweepPackage::parse_bounded(&serde_json::to_vec(&value).expect("tampered JSON"))
            .expect("envelope parses");

    assert!(package.validate(identity().public_key()).is_err());
}

#[test]
fn signs_every_input_and_serializes_a_luke_compatible_approved_sweep() {
    let identity = identity();
    let package = CooperativeSweepPackage::parse_bounded(&valid_sweep_json()).expect("parse");
    let approved = package
        .validate(identity.public_key())
        .expect("validate")
        .approve(&identity)
        .expect("approve");
    let json = approved.to_json().expect("serialize");
    let value: Value = serde_json::from_slice(&json).expect("approved JSON");

    assert_eq!(approved.hww_signature_count(), 1);
    assert_eq!(value["kind"], "cooperative-sweep");
    assert_eq!(value["phone_approved"], true);
    assert_eq!(value["hww_approved"], true);
    assert!(json.ends_with(b"\n"));
}

#[test]
fn enforces_the_shared_import_limit() {
    let oversized = vec![b' '; anzen_policy_engine::MAX_PACKAGE_BYTES + 1];
    assert_eq!(
        CooperativeSweepPackage::parse_bounded(&oversized).unwrap_err(),
        PolicyError::PackageTooLarge
    );
}
