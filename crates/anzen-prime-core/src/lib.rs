use anzen_cold_signer::benchmark::{
    transcript_hash, BenchmarkConfig, BenchmarkError as GraphError, PolicyCommitment, Sha256,
    VisitError, BIP341_NUMS_XONLY,
};
use k256::{
    elliptic_curve::{sec1::ToEncodedPoint, PrimeField},
    schnorr::SigningKey,
    AffinePoint, ProjectivePoint, PublicKey, Scalar,
};
use sha2::{Digest, Sha256 as Sha2};

mod policy_flow;

pub use policy_flow::{
    AppSeedSource, ApprovalReceipt, ApprovedPackageSink, PolicyFlowError, ReviewedPolicyPackage,
};

const FIXED_PHONE_XONLY_PUBLIC_KEY: [u8; 32] = [
    0x79, 0xbe, 0x66, 0x7e, 0xf9, 0xdc, 0xbb, 0xac, 0x55, 0xa0, 0x62, 0x95, 0xce, 0x87, 0x0b, 0x07,
    0x02, 0x9b, 0xfc, 0xdb, 0x2d, 0xce, 0x28, 0xd9, 0x59, 0xf2, 0x81, 0x5b, 0x16, 0xf8, 0x17, 0x98,
];
const KEY_DOMAIN: &[u8] = b"Anzen Passport Prime proof key v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemoResult {
    pub transactions: u8,
    pub signatures: u8,
    pub verified_signatures: u8,
    pub public_key: [u8; 32],
    pub transcript: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemoPresentation {
    pub title: String,
    pub workload: String,
    pub verification: String,
    pub transcript: String,
    pub environment: String,
}

impl DemoPresentation {
    pub fn from_result(result: DemoResult, environment: impl Into<String>) -> Self {
        Self {
            title: "Anzen policy signed".to_string(),
            workload: format!(
                "{} transactions  ·  {} Schnorr signatures",
                result.transactions, result.signatures
            ),
            verification: format!("All {} signatures verified", result.verified_signatures),
            transcript: format!("Transcript {}", hex_prefix(&result.transcript, 8)),
            environment: environment.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoError {
    InvalidKey,
    InvalidTweak,
    InvalidNumsPoint,
    InvalidSignature,
    Graph(GraphError),
}

impl From<GraphError> for DemoError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}

impl From<VisitError<DemoError>> for DemoError {
    fn from(value: VisitError<DemoError>) -> Self {
        match value {
            VisitError::Graph(error) => Self::Graph(error),
            VisitError::Callback(error) => error,
        }
    }
}

pub struct RustSha256;

impl Sha256 for RustSha256 {
    fn hash(&mut self, parts: &[&[u8]]) -> [u8; 32] {
        let mut hasher = Sha2::new();
        for part in parts {
            hasher.update(part);
        }
        hasher.finalize().into()
    }
}

/// Executes Luke Childs' largest current Anzen hardware-wallet benchmark.
///
/// The caller supplies KeyOS's app-isolated seed. The seed never leaves this
/// function; only the x-only public key and a transcript commitment are returned.
pub fn run_demo(app_seed: [u8; 32]) -> Result<DemoResult, DemoError> {
    let signing_key = derive_demo_key(app_seed)?;
    let verifying_key = signing_key.verifying_key();
    let hww_xonly: [u8; 32] = verifying_key.to_bytes().into();

    let mut hasher = RustSha256;
    let policy = PolicyCommitment::new(&mut hasher, FIXED_PHONE_XONLY_PUBLIC_KEY, hww_xonly);
    let vault_output_key = taproot_output_key(policy.output_key_tweak)?;
    let config = BenchmarkConfig::deterministic(
        &mut hasher,
        12,
        vault_output_key,
        policy.cooperative_leaf_hash,
    )?;

    let mut transcript_hasher = RustSha256;
    let mut transcript = [0_u8; 32];
    let mut verified = 0_u8;
    let summary = config.for_each_signature_job(&mut hasher, |job| {
        let signature = signing_key
            .sign_raw(&job.sighash, &[0_u8; 32])
            .map_err(|_| DemoError::InvalidSignature)?;
        verifying_key
            .verify_raw(&job.sighash, &signature)
            .map_err(|_| DemoError::InvalidSignature)?;
        let signature_bytes: [u8; 64] = signature.to_bytes();
        transcript = transcript_hash(
            &mut transcript_hasher,
            transcript,
            job.sighash,
            signature_bytes,
        );
        verified += 1;
        Ok(())
    })?;

    Ok(DemoResult {
        transactions: summary.transactions,
        signatures: summary.signature_jobs,
        verified_signatures: verified,
        public_key: hww_xonly,
        transcript,
    })
}

fn derive_demo_key(app_seed: [u8; 32]) -> Result<SigningKey, DemoError> {
    let digest: [u8; 32] = Sha2::new()
        .chain_update(KEY_DOMAIN)
        .chain_update(app_seed)
        .finalize()
        .into();
    SigningKey::from_bytes(&digest).map_err(|_| DemoError::InvalidKey)
}

fn taproot_output_key(tweak_bytes: [u8; 32]) -> Result<[u8; 32], DemoError> {
    let mut compressed_nums = [0_u8; 33];
    compressed_nums[0] = 0x02;
    compressed_nums[1..].copy_from_slice(&BIP341_NUMS_XONLY);
    let nums =
        PublicKey::from_sec1_bytes(&compressed_nums).map_err(|_| DemoError::InvalidNumsPoint)?;
    let tweak = Option::<Scalar>::from(Scalar::from_repr(tweak_bytes.into()))
        .ok_or(DemoError::InvalidTweak)?;
    let output_point =
        ProjectivePoint::from(*nums.as_affine()) + ProjectivePoint::GENERATOR * tweak;
    let encoded = AffinePoint::from(output_point).to_encoded_point(true);
    let x = encoded.x().ok_or(DemoError::InvalidTweak)?;
    let mut output_key = [0_u8; 32];
    output_key.copy_from_slice(x);
    Ok(output_key)
}

fn hex_prefix(bytes: &[u8], count: usize) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(count * 2 + 3);
    for byte in bytes.iter().take(count) {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output.push_str("...");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signs_lukes_largest_benchmark_with_valid_schnorr_signatures() {
        let result = run_demo([0x42; 32]).expect("benchmark should sign");

        assert_eq!(result.transactions, 28);
        assert_eq!(result.signatures, 39);
        assert_eq!(result.verified_signatures, 39);
        assert_eq!(
            result.public_key,
            [
                0x51, 0xb6, 0x50, 0xe6, 0x88, 0xb5, 0x7c, 0x62, 0x4e, 0x5b, 0x8f, 0xba, 0x14, 0x1d,
                0xbd, 0x46, 0x01, 0xc6, 0x8e, 0xb4, 0x81, 0xf4, 0xc7, 0x38, 0x84, 0xfd, 0x89, 0x95,
                0x55, 0x7e, 0x90, 0xb4,
            ]
        );
        assert_eq!(
            result.transcript,
            [
                0xb5, 0x50, 0x14, 0x86, 0x5a, 0x57, 0x07, 0x97, 0xcc, 0xfb, 0x16, 0xb4, 0xb9, 0xd5,
                0xdc, 0xec, 0xad, 0x8f, 0x7e, 0x05, 0x57, 0xbb, 0xec, 0x85, 0xf7, 0x26, 0x96, 0x64,
                0x50, 0xa9, 0x53, 0xa6,
            ]
        );
    }

    #[test]
    fn result_is_deterministic_for_the_same_prime_app_seed() {
        let first = run_demo([0x42; 32]).expect("first run");
        let second = run_demo([0x42; 32]).expect("second run");

        assert_eq!(first, second);
    }

    #[test]
    fn different_prime_app_seeds_produce_different_signing_identities() {
        let first = run_demo([0x42; 32]).expect("first run");
        let second = run_demo([0x43; 32]).expect("second run");

        assert_ne!(first.public_key, second.public_key);
        assert_ne!(first.transcript, second.transcript);
    }

    #[test]
    fn presentation_reports_the_verified_workload_without_overclaiming_hardware() {
        let result = run_demo([0x42; 32]).expect("benchmark should sign");
        let presentation = DemoPresentation::from_result(result, "Windows preview");

        assert_eq!(presentation.title, "Anzen policy signed");
        assert_eq!(
            presentation.workload,
            "28 transactions  ·  39 Schnorr signatures"
        );
        assert_eq!(presentation.verification, "All 39 signatures verified");
        assert_eq!(presentation.environment, "Windows preview");
        assert!(presentation.transcript.starts_with("Transcript "));
        assert!(presentation.transcript.ends_with("..."));
    }
}
