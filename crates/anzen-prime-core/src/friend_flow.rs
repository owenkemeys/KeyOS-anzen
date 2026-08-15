use crate::{AppSeedSource, ApprovalClock, ApprovalReceipt, ApprovedPackageSink, PolicyFlowError};
use anzen_policy_engine::{
    AnzenIdentity, CloudRecoveryBackup, RecoveryFriendSummary, ReviewedRecoveryFriend, VaultConfig,
};
use bitcoin::Network;
use zeroize::Zeroize;

pub struct ReviewedRecoveryFriendRequest {
    reviewed: ReviewedRecoveryFriend,
    summary: RecoveryFriendSummary,
}

impl ReviewedRecoveryFriendRequest {
    pub fn import(
        backup: &[u8],
        config: &[u8],
        public_key: &[u8],
    ) -> Result<Self, PolicyFlowError> {
        let backup = CloudRecoveryBackup::parse_bounded(backup)?;
        let config = VaultConfig::parse_bounded(config)?;
        let reviewed = backup.review_friend_enrollment(config, public_key)?;
        let summary = reviewed.summary().clone();
        Ok(Self { reviewed, summary })
    }

    pub fn summary(&self) -> &RecoveryFriendSummary {
        &self.summary
    }

    pub fn approve_and_export(
        &self,
        seed_source: &mut impl AppSeedSource,
        sink: &mut impl ApprovedPackageSink,
    ) -> Result<ApprovalReceipt, PolicyFlowError> {
        struct Untimed;
        impl ApprovalClock for Untimed {
            fn now_ms(&mut self) -> u64 {
                0
            }
        }
        self.approve_and_export_timed(seed_source, sink, &mut Untimed)
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
        let approved = self.reviewed.clone().approve(&identity)?;
        let validation_end = clock.now_ms();
        let json = approved.to_json()?;
        sink.write_temporary(&json)
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        sink.commit_temporary()
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        Ok(ApprovalReceipt {
            psbt_count: 0,
            hww_signature_count: 0,
            validation_ms: validation_end.saturating_sub(validation_start),
            signing_ms: 0,
            total_approval_ms: clock.now_ms().saturating_sub(total_start),
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
