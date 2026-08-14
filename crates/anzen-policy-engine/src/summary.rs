use crate::{PolicyError, PolicyPackage};

const PHONE_RECOVERY_BLOCKS: u16 = 61_200;
const HWW_RECOVERY_BLOCKS: u16 = 65_535;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicySummary {
    pub network: String,
    pub vault_address: String,
    pub total_input_sats: u64,
    pub monthly_limit_sats: u64,
    pub allowance_count: usize,
    pub allowance_delay_seconds: Option<u32>,
    pub emergency_access_limit_sats: u64,
    pub emergency_delay_seconds: Option<u32>,
    pub fee_rate_sat_vb: u64,
    pub rollover_txid: String,
    pub rollover_fee_sats: u64,
    pub allowance_value_sats: u64,
    pub remainder_value_sats: u64,
    pub phone_recovery_blocks: u16,
    pub hww_recovery_blocks: u16,
}

impl PolicyPackage {
    pub fn summary(&self) -> Result<PolicySummary, PolicyError> {
        Ok(PolicySummary {
            network: self.manifest.network.clone(),
            vault_address: self.manifest.vault_address.clone(),
            total_input_sats: self.manifest.total_input_sats,
            monthly_limit_sats: self.manifest.monthly_limit_sats,
            allowance_count: self.manifest.allowance_count,
            allowance_delay_seconds: self
                .manifest
                .allowances
                .first()
                .map(|allowance| allowance.delay_seconds),
            emergency_access_limit_sats: self.manifest.emergency_access_limit_sats,
            emergency_delay_seconds: self
                .manifest
                .emergency_access
                .as_ref()
                .map(|emergency| emergency.delay_seconds),
            fee_rate_sat_vb: self.manifest.fee_rate_sat_vb,
            rollover_txid: self.manifest.rollover.unsigned_txid.clone(),
            rollover_fee_sats: self.manifest.rollover.fee_sats,
            allowance_value_sats: self.manifest.allowance_value_sats,
            remainder_value_sats: self.manifest.remainder_value_sats,
            phone_recovery_blocks: PHONE_RECOVERY_BLOCKS,
            hww_recovery_blocks: HWW_RECOVERY_BLOCKS,
        })
    }
}
