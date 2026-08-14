use anzen_policy_engine::{AnzenIdentity, PolicyError, PolicyPackage, PolicySummary};
use bitcoin::Network;
use zeroize::Zeroize;

pub trait AppSeedSource {
    type Error;

    fn app_seed(&mut self) -> Result<[u8; 32], Self::Error>;
}

pub trait ApprovedPackageSink {
    type Error;

    fn write_temporary(&mut self, bytes: &[u8]) -> Result<(), Self::Error>;
    fn commit_temporary(&mut self) -> Result<(), Self::Error>;
}

pub trait ApprovalClock {
    fn now_ms(&mut self) -> u64;
}

#[derive(Debug, PartialEq, Eq)]
pub enum PolicyFlowError {
    Policy(PolicyError),
    SeedUnavailable,
    ExportFailed,
}

impl From<PolicyError> for PolicyFlowError {
    fn from(value: PolicyError) -> Self {
        Self::Policy(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApprovalReceipt {
    pub psbt_count: usize,
    pub hww_signature_count: usize,
    pub validation_ms: u64,
    pub signing_ms: u64,
    pub total_approval_ms: u64,
}

impl ApprovalReceipt {
    pub fn timing_summary(&self, context: &str) -> String {
        format!(
            "Validation {} ms · Signing {} ms\nTotal approval {} ms\n{}",
            self.validation_ms, self.signing_ms, self.total_approval_ms, context
        )
    }
}

struct UntimedClock;

impl ApprovalClock for UntimedClock {
    fn now_ms(&mut self) -> u64 {
        0
    }
}

pub struct ReviewedPolicyPackage {
    package: PolicyPackage,
    summary: PolicySummary,
    transaction_count: usize,
}

impl ReviewedPolicyPackage {
    pub fn import(bytes: &[u8]) -> Result<Self, PolicyFlowError> {
        let package = PolicyPackage::parse_bounded(bytes)?;
        let summary = package.summary()?;
        let transaction_count = package.psbts.len();
        Ok(Self {
            package,
            summary,
            transaction_count,
        })
    }

    pub fn summary(&self) -> &PolicySummary {
        &self.summary
    }

    pub fn transaction_count(&self) -> usize {
        self.transaction_count
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
        let validated = self.package.clone().validate(identity.public_key())?;
        let validation_end = clock.now_ms();
        let psbt_count = validated.psbt_count();
        let signing_start = clock.now_ms();
        let approved = validated.approve(&identity)?;
        let signing_end = clock.now_ms();
        let hww_signature_count = approved.hww_signature_count();
        let json = approved.to_json()?;

        sink.write_temporary(&json)
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        sink.commit_temporary()
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        let total_end = clock.now_ms();

        Ok(ApprovalReceipt {
            psbt_count,
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
            _ => Err(PolicyFlowError::Policy(PolicyError::InvalidPolicy(
                "unsupported network",
            ))),
        }
    }
}
