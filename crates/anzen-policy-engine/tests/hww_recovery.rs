use anzen_policy_engine::{AnzenIdentity, HwwRecoverySnapshot, PolicyError};
use bitcoin::{consensus, Address, Network, Sequence, Transaction};
use serde_json::json;
use std::str::FromStr;

const DESCRIPTOR: &str = "tr(50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0,{multi_a(2,b90c0f8da23429c1f36288f53c46f50c54660804dcdd04fe342a3fd3d9e8c9ac,906979e124fa1d21104bbcfce84d825cb1269fa1dc95a0a27e9bfe120d88e97c),{and_v(v:older(61200),pk(b90c0f8da23429c1f36288f53c46f50c54660804dcdd04fe342a3fd3d9e8c9ac)),and_v(v:older(65535),pk(906979e124fa1d21104bbcfce84d825cb1269fa1dc95a0a27e9bfe120d88e97c))}})#a303ks35";
const ADDRESS: &str = "bcrt1ppc630jlwa8nykupxpam7rknknpx8mljnll04kjn4cst6zm20t7rslwygna";

fn snapshot(tip_height: u64) -> Vec<u8> {
    let address = Address::from_str(ADDRESS)
        .unwrap()
        .require_network(Network::Regtest)
        .unwrap();
    let script = address
        .script_pubkey()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    serde_json::to_vec(&json!({
        "version": 1,
        "kind": "hww-recovery-snapshot",
        "network": "regtest",
        "vault_descriptor": DESCRIPTOR,
        "destination": ADDRESS,
        "tip_height": tip_height,
        "utxos": [{
            "txid": "0000000000000000000000000000000000000000000000000000000000000001",
            "vout": 0,
            "value_sats": 1_000_000,
            "script_pubkey": script,
            "confirmation_height": 1
        }]
    }))
    .unwrap()
}

#[test]
fn reviews_and_signs_only_the_mature_65535_block_hww_path() {
    let reviewed = HwwRecoverySnapshot::parse_bounded(&snapshot(65_535))
        .unwrap()
        .review()
        .unwrap();
    let summary = reviewed.summary();
    assert_eq!(summary.delay_blocks, 65_535);
    assert_eq!(summary.input_count, 1);
    assert_eq!(summary.destination, ADDRESS);
    assert!(summary.sent_sats < 1_000_000);
    assert_eq!(summary.fee_sats, 1_000_000 - summary.sent_sats);

    let identity = AnzenIdentity::from_app_seed(&[0x42; 32], Network::Regtest).unwrap();
    let approved = reviewed.approve(&identity).unwrap();
    let transaction: Transaction = consensus::deserialize(&approved.transaction_bytes()).unwrap();
    assert_eq!(
        transaction.input[0].sequence,
        Sequence::from_consensus(65_535)
    );
    assert_eq!(transaction.input[0].witness.len(), 3);
    assert_eq!(approved.hww_signature_count(), 1);
}

#[test]
fn refuses_an_immature_snapshot_before_any_signing_identity_is_needed() {
    let error = HwwRecoverySnapshot::parse_bounded(&snapshot(65_534))
        .unwrap()
        .review()
        .unwrap_err();
    assert_eq!(
        error,
        PolicyError::InvalidPolicy("no vault UTXOs are mature for HWW recovery")
    );
}
