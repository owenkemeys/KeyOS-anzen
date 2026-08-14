use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const MAX_PACKAGE_BYTES: usize = 128 * 1024;
const POLICY_PACKAGE_KIND: &str = "vault-policy";
const POLICY_PACKAGE_VERSION: u8 = 4;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PolicyError {
    #[error("policy package exceeds the 128 KiB import limit")]
    PackageTooLarge,
    #[error("policy package JSON is invalid")]
    InvalidJson,
    #[error("unsupported policy package")]
    UnsupportedPackage,
    #[error("policy package contains an unsafe PSBT path")]
    UnsafePsbtPath,
    #[error("policy package PSBT set does not match its manifest")]
    PsbtSetMismatch,
    #[error("policy package contains an invalid PSBT")]
    InvalidPsbt,
    #[error("policy package violates the approved policy: {0}")]
    InvalidPolicy(&'static str),
    #[error("unable to derive the Anzen HWW identity")]
    KeyDerivation,
    #[error("unable to sign the validated policy package")]
    Signing,
    #[error("unable to serialize the approved policy package")]
    Serialization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchTransaction {
    pub psbt_file: String,
    pub unsigned_txid: String,
    pub fee_sats: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowanceStep {
    pub step: u8,
    pub delay_seconds: u32,
    pub delay_sequence: u32,
    pub chain_value_sats: u64,
    pub hot_address: String,
    pub authorization: BatchTransaction,
    pub revocation: BatchTransaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyAccessPolicy {
    pub amount_sats: u64,
    pub delay_seconds: u32,
    pub delay_sequence: u32,
    pub hot_address: String,
    pub staging_vout: u32,
    pub staging_value_sats: u64,
    pub vault_change_vout: u32,
    pub vault_change_value_sats: u64,
    pub trigger: BatchTransaction,
    pub withdrawal: BatchTransaction,
    pub cancellation: BatchTransaction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchManifest {
    pub version: u8,
    pub created_at: i64,
    pub network: String,
    pub vault_descriptor: String,
    pub vault_address: String,
    #[serde(alias = "hard_limit_sats")]
    pub monthly_limit_sats: u64,
    #[serde(default)]
    pub emergency_access_limit_sats: u64,
    pub fee_rate_sat_vb: u64,
    pub total_input_sats: u64,
    pub allowance_count: usize,
    pub rollover: BatchTransaction,
    pub allowance_vout: Option<u32>,
    pub allowance_value_sats: u64,
    pub remainder_vout: u32,
    pub remainder_value_sats: u64,
    pub allowances: Vec<AllowanceStep>,
    #[serde(default)]
    pub emergency_access: Option<EmergencyAccessPolicy>,
    pub phone_approved: bool,
    pub hww_approved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPackage {
    pub version: u8,
    pub kind: String,
    pub manifest: BatchManifest,
    pub psbts: BTreeMap<String, String>,
}

impl PolicyPackage {
    pub fn parse_bounded(bytes: &[u8]) -> Result<Self, PolicyError> {
        if bytes.len() > MAX_PACKAGE_BYTES {
            return Err(PolicyError::PackageTooLarge);
        }
        let package: Self = serde_json::from_slice(bytes).map_err(|_| PolicyError::InvalidJson)?;
        package.validate_envelope()?;
        Ok(package)
    }

    fn validate_envelope(&self) -> Result<(), PolicyError> {
        if self.version != POLICY_PACKAGE_VERSION
            || self.kind != POLICY_PACKAGE_KIND
            || self.manifest.version != POLICY_PACKAGE_VERSION
        {
            return Err(PolicyError::UnsupportedPackage);
        }

        let mut expected = BTreeSet::new();
        for transaction in self.manifest_transactions() {
            if !is_safe_relative_path(&transaction.psbt_file) {
                return Err(PolicyError::UnsafePsbtPath);
            }
            if !expected.insert(transaction.psbt_file.as_str()) {
                return Err(PolicyError::PsbtSetMismatch);
            }
        }
        if self.psbts.keys().any(|path| !is_safe_relative_path(path)) {
            return Err(PolicyError::UnsafePsbtPath);
        }
        if expected.len() != self.psbts.len()
            || !expected
                .iter()
                .all(|expected_path| self.psbts.contains_key(*expected_path))
        {
            return Err(PolicyError::PsbtSetMismatch);
        }
        Ok(())
    }

    pub(crate) fn manifest_transactions(&self) -> Vec<&BatchTransaction> {
        let mut transactions = Vec::with_capacity(
            1 + self.manifest.allowances.len() * 2
                + usize::from(self.manifest.emergency_access.is_some()) * 3,
        );
        transactions.push(&self.manifest.rollover);
        for allowance in &self.manifest.allowances {
            transactions.push(&allowance.authorization);
            transactions.push(&allowance.revocation);
        }
        if let Some(emergency) = &self.manifest.emergency_access {
            transactions.push(&emergency.trigger);
            transactions.push(&emergency.withdrawal);
            transactions.push(&emergency.cancellation);
        }
        transactions
    }
}

fn is_safe_relative_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.starts_with('\\')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
}
