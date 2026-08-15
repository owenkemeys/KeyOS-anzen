use crate::{AppSeedSource, ApprovalClock, ApprovalReceipt, ApprovedPackageSink, PolicyFlowError};
use anzen_policy_engine::{
    AnzenIdentity, HwwRecoverySnapshot, HwwRecoverySummary, ReviewedHwwRecovery,
};
use bitcoin::Network;
use zeroize::Zeroize;

struct UntimedClock;

impl ApprovalClock for UntimedClock {
    fn now_ms(&mut self) -> u64 {
        0
    }
}

pub struct ReviewedHwwRecoveryRequest {
    reviewed: ReviewedHwwRecovery,
    summary: HwwRecoverySummary,
}

impl ReviewedHwwRecoveryRequest {
    pub fn import(bytes: &[u8]) -> Result<Self, PolicyFlowError> {
        let reviewed = HwwRecoverySnapshot::parse_bounded(bytes)?.review()?;
        let summary = reviewed.summary().clone();
        Ok(Self { reviewed, summary })
    }

    pub fn summary(&self) -> &HwwRecoverySummary {
        &self.summary
    }

    pub fn approve_and_export(
        &self,
        seed_source: &mut impl AppSeedSource,
        sink: &mut impl ApprovedPackageSink,
    ) -> Result<ApprovalReceipt, PolicyFlowError> {
        self.approve_and_export_timed(seed_source, sink, &mut UntimedClock)
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
        let hww_signature_count = approved.hww_signature_count();
        let json = approved.to_json()?;
        sink.write_temporary(&json)
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        sink.commit_temporary()
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        let total_end = clock.now_ms();

        Ok(ApprovalReceipt {
            psbt_count: 1,
            hww_signature_count,
            validation_ms: validation_end.saturating_sub(validation_start),
            signing_ms: signing_end.saturating_sub(signing_start),
            total_approval_ms: total_end.saturating_sub(total_start),
        })
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
