use anzen_policy_engine::{PhoneRotationPackage, PolicyError};

const MINIMAL_ROTATION: &str = r#"{
  "version": 1,
  "kind": "phone-key-rotation",
  "old_vault_descriptor": "old-descriptor",
  "new_phone_vault_pubkey": "1111111111111111111111111111111111111111111111111111111111111111",
  "new_vault_descriptor": "new-descriptor",
  "new_vault_address": "bcrt1ppc630jlwa8nykupxpam7rknknpx8mljnll04kjn4cst6zm20t7rslwygna",
  "monthly_limit_sats": 10000000,
  "emergency_access_limit_sats": 50000000,
  "sweep": {
    "version": 1,
    "kind": "cooperative-sweep",
    "vault_descriptor": "old-descriptor",
    "destination": "bcrt1ppc630jlwa8nykupxpam7rknknpx8mljnll04kjn4cst6zm20t7rslwygna",
    "psbt": "not-yet-validated",
    "input_count": 2,
    "sent_sats": 99000000,
    "fee_sats": 250,
    "phone_approved": true,
    "hww_approved": false
  },
  "renewed_policy": null,
  "cloud_recovery_backup": null
}"#;

#[test]
fn rotation_summary_exposes_every_owner_decision_before_validation() {
    let package = PhoneRotationPackage::parse_bounded(MINIMAL_ROTATION.as_bytes()).unwrap();
    let summary = package.summary().unwrap();

    assert_eq!(
        summary.new_vault_address,
        "bcrt1ppc630jlwa8nykupxpam7rknknpx8mljnll04kjn4cst6zm20t7rslwygna"
    );
    assert_eq!(summary.input_count, 2);
    assert_eq!(summary.sent_sats, 99_000_000);
    assert_eq!(summary.fee_sats, 250);
    assert_eq!(summary.monthly_limit_sats, 10_000_000);
    assert_eq!(summary.emergency_access_limit_sats, 50_000_000);
    assert!(!summary.renews_policy);
}

#[test]
fn unsupported_rotation_envelope_is_rejected_at_import() {
    let changed = MINIMAL_ROTATION.replace("\"version\": 1", "\"version\": 2");
    assert_eq!(
        PhoneRotationPackage::parse_bounded(changed.as_bytes()).unwrap_err(),
        PolicyError::UnsupportedPackage
    );
}
