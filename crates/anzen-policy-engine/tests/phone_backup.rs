use anzen_policy_engine::{CloudRecoveryBackup, PhoneBackupSummary, VaultConfig};

#[test]
fn backup_review_exposes_only_the_bound_vault_summary() {
    let backup: CloudRecoveryBackup = serde_json::from_str(include_str!(
        "../../../fixtures/phone-backup-v1/review-only-backup.json"
    ))
    .unwrap();
    let config: VaultConfig = serde_json::from_str(include_str!(
        "../../../fixtures/phone-backup-v1/regtest-config.json"
    ))
    .unwrap();

    let reviewed = backup.review_for_recovery(config).expect("review");
    assert_eq!(
        reviewed.summary(),
        &PhoneBackupSummary {
            network: "regtest".into(),
            vault_address: "bcrt1ptest-review-only".into(),
            recovery_friend_count: 0,
        }
    );
}
