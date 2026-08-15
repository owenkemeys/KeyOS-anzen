use crate::{AppSeedSource, ApprovalClock, ApprovalReceipt, ApprovedPackageSink, PolicyFlowError};
use anzen_policy_engine::{
    AnzenIdentity, CloudRecoveryBackup, DeviceFile, PhoneRotationPackage, PhoneRotationSummary,
    ReviewedPhoneRotation, VaultConfig,
};
use bitcoin::Network;
use zeroize::Zeroize;

struct UntimedClock;

impl ApprovalClock for UntimedClock {
    fn now_ms(&mut self) -> u64 {
        0
    }
}

pub struct ReviewedPhoneRotationRequest {
    reviewed: ReviewedPhoneRotation,
    summary: PhoneRotationSummary,
}

impl ReviewedPhoneRotationRequest {
    pub fn import(
        proposal: &[u8],
        config: &[u8],
        pending: &[u8],
        backup: &[u8],
    ) -> Result<Self, PolicyFlowError> {
        let package = PhoneRotationPackage::parse_bounded(proposal)?;
        let current = VaultConfig::parse_bounded(config)?;
        let pending = DeviceFile::parse_bounded(pending)?;
        let backup = CloudRecoveryBackup::parse_bounded(backup)?;
        let reviewed = package.review(current, pending, backup)?;
        let summary = reviewed.summary();
        Ok(Self { reviewed, summary })
    }

    pub fn summary(&self) -> &PhoneRotationSummary {
        &self.summary
    }

    pub fn approve_and_export_timed(
        &self,
        seed_source: &mut impl AppSeedSource,
        sink: &mut impl ApprovedPackageSink,
        clock: &mut impl ApprovalClock,
    ) -> Result<ApprovalReceipt, PolicyFlowError> {
        let total_start = clock.now_ms();
        let mut app_seed = seed_source
            .app_seed()
            .map_err(|_| PolicyFlowError::SeedUnavailable)?;
        let identity_result = AnzenIdentity::from_app_seed(&app_seed, self.network()?);
        app_seed.zeroize();
        let identity = identity_result?;

        let validation_start = clock.now_ms();
        let reviewed = self.reviewed.clone();
        let validation_end = clock.now_ms();
        let signing_start = clock.now_ms();
        let approved = reviewed.approve(&identity)?;
        let signing_end = clock.now_ms();
        let hww_signature_count =
            approved.sweep_signature_count() + approved.policy_signature_count();
        let json = approved.to_json()?;
        sink.write_temporary(&json)
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        sink.commit_temporary()
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        let total_end = clock.now_ms();

        Ok(ApprovalReceipt {
            psbt_count: 1 + self.summary.policy_psbt_count,
            hww_signature_count,
            validation_ms: validation_end.saturating_sub(validation_start),
            signing_ms: signing_end.saturating_sub(signing_start),
            total_approval_ms: total_end.saturating_sub(total_start),
        })
    }

    pub fn approve_and_export(
        &self,
        seed_source: &mut impl AppSeedSource,
        sink: &mut impl ApprovedPackageSink,
    ) -> Result<ApprovalReceipt, PolicyFlowError> {
        self.approve_and_export_timed(seed_source, sink, &mut UntimedClock)
    }

    fn network(&self) -> Result<Network, PolicyFlowError> {
        match self.summary.network.as_str() {
            "bitcoin" => Ok(Network::Bitcoin),
            "regtest" => Ok(Network::Regtest),
            _ => Err(PolicyFlowError::Policy(
                anzen_policy_engine::PolicyError::InvalidPolicy("unsupported network"),
            )),
        }
    }
}
