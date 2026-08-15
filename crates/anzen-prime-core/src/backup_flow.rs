use crate::{AppSeedSource, ApprovalReceipt, ApprovedPackageSink, PolicyFlowError};
use anzen_policy_engine::{
    AnzenIdentity, CloudRecoveryBackup, PhoneBackupSummary, ReviewedPhoneBackup, VaultConfig,
};
use bitcoin::Network;
use zeroize::Zeroize;

pub struct ReviewedPhoneBackupRequest {
    reviewed: ReviewedPhoneBackup,
    summary: PhoneBackupSummary,
}

impl ReviewedPhoneBackupRequest {
    pub fn import(backup: &[u8], config: &[u8]) -> Result<Self, PolicyFlowError> {
        let backup = CloudRecoveryBackup::parse_bounded(backup)?;
        let config = VaultConfig::parse_bounded(config)?;
        let reviewed = backup.review_for_recovery(config)?;
        let summary = reviewed.summary().clone();
        Ok(Self { reviewed, summary })
    }

    pub fn summary(&self) -> &PhoneBackupSummary {
        &self.summary
    }

    pub fn approve_and_export(
        &self,
        seed_source: &mut impl AppSeedSource,
        sink: &mut impl ApprovedPackageSink,
    ) -> Result<ApprovalReceipt, PolicyFlowError> {
        let mut app_seed = seed_source
            .app_seed()
            .map_err(|_| PolicyFlowError::SeedUnavailable)?;
        let identity_result = AnzenIdentity::from_app_seed(&app_seed, self.network()?);
        app_seed.zeroize();
        let identity = identity_result?;
        let package = self.reviewed.clone().approve(&identity)?;
        let json = package.to_json()?;
        sink.write_temporary(&json)
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        sink.commit_temporary()
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        Ok(ApprovalReceipt {
            psbt_count: 0,
            hww_signature_count: 0,
            validation_ms: 0,
            signing_ms: 0,
            total_approval_ms: 0,
        })
    }

    fn network(&self) -> Result<Network, PolicyFlowError> {
        match self.summary.network.as_str() {
            "bitcoin" | "mainnet" => Ok(Network::Bitcoin),
            "regtest" => Ok(Network::Regtest),
            _ => Err(PolicyFlowError::Policy(
                anzen_policy_engine::PolicyError::InvalidPolicy("unsupported network"),
            )),
        }
    }
}
