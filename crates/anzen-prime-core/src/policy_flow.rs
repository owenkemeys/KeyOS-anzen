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
        let mut app_seed = seed_source
            .app_seed()
            .map_err(|_| PolicyFlowError::SeedUnavailable)?;
        let identity_result = AnzenIdentity::from_app_seed(&app_seed, self.network()?);
        app_seed.zeroize();
        let identity = identity_result?;

        let validated = self.package.clone().validate(identity.public_key())?;
        let psbt_count = validated.psbt_count();
        let approved = validated.approve(&identity)?;
        let hww_signature_count = approved.hww_signature_count();
        let json = approved.to_json()?;

        sink.write_temporary(&json)
            .map_err(|_| PolicyFlowError::ExportFailed)?;
        sink.commit_temporary()
            .map_err(|_| PolicyFlowError::ExportFailed)?;

        Ok(ApprovalReceipt {
            psbt_count,
            hww_signature_count,
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
