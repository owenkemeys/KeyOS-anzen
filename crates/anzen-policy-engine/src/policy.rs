// Adapted from Luke Childs' Anzen src/core/policy.rs at
// 01794bb14d34d01413e3b539c7e5496ecbce0c87 under the MIT license.

use bitcoin::{
    key::TapTweak,
    opcodes::all::{OP_CHECKSIG, OP_CHECKSIGADD, OP_CSV, OP_NUMEQUAL, OP_VERIFY},
    script::Builder,
    secp256k1::{Secp256k1, XOnlyPublicKey},
    taproot::{ControlBlock, LeafVersion, TapLeafHash, TapNodeHash},
    Address, Network, ScriptBuf,
};

pub(crate) const PHONE_RECOVERY_BLOCKS: u16 = 61_200;
pub(crate) const HWW_RECOVERY_BLOCKS: u16 = 65_535;
pub(crate) const BIP341_NUMS_KEY: &str =
    "50929b74c1a04954b78b4b6035e97a5e078a5a0f28ec96d547bfee9ace803ac0";

#[derive(Debug, Clone)]
pub(crate) struct VaultLeaf {
    pub script: ScriptBuf,
    pub control_block: ControlBlock,
    pub leaf_hash: TapLeafHash,
}

#[derive(Debug, Clone)]
pub(crate) struct VaultPolicy {
    pub address: Address,
    phone: XOnlyPublicKey,
    hww: XOnlyPublicKey,
}

impl VaultPolicy {
    pub fn new(phone: XOnlyPublicKey, hww: XOnlyPublicKey, network: Network) -> Self {
        let secp = Secp256k1::verification_only();
        let internal_key: XOnlyPublicKey = BIP341_NUMS_KEY.parse().expect("valid NUMS key");
        let cooperative = cooperative_script(phone, hww);
        let phone_recovery = recovery_script(phone, PHONE_RECOVERY_BLOCKS);
        let hww_recovery = recovery_script(hww, HWW_RECOVERY_BLOCKS);
        let cooperative_node = TapNodeHash::from_script(&cooperative, LeafVersion::TapScript);
        let recovery_node = TapNodeHash::from_node_hashes(
            TapNodeHash::from_script(&phone_recovery, LeafVersion::TapScript),
            TapNodeHash::from_script(&hww_recovery, LeafVersion::TapScript),
        );
        let merkle_root = TapNodeHash::from_node_hashes(cooperative_node, recovery_node);
        let output = internal_key.tap_tweak(&secp, Some(merkle_root)).0;
        Self {
            address: Address::p2tr_tweaked(output, network),
            phone,
            hww,
        }
    }

    pub fn descriptor_string(&self) -> String {
        let descriptor = format!(
            "tr({BIP341_NUMS_KEY},{{multi_a(2,{},{}),{{and_v(v:older({PHONE_RECOVERY_BLOCKS}),pk({})),and_v(v:older({HWW_RECOVERY_BLOCKS}),pk({}))}}}})",
            self.phone, self.hww, self.phone, self.hww
        );
        let checksum = miniscript::descriptor::checksum::desc_checksum(&descriptor)
            .expect("static descriptor is checksummable");
        format!("{descriptor}#{checksum}")
    }

    pub fn cooperative_leaf(&self) -> VaultLeaf {
        let secp = Secp256k1::verification_only();
        let internal_key: XOnlyPublicKey = BIP341_NUMS_KEY.parse().expect("valid NUMS key");
        let cooperative = cooperative_script(self.phone, self.hww);
        let phone_recovery = recovery_script(self.phone, PHONE_RECOVERY_BLOCKS);
        let hww_recovery = recovery_script(self.hww, HWW_RECOVERY_BLOCKS);
        let spend_info = bitcoin::taproot::TaprootBuilder::new()
            .add_leaf(1, cooperative.clone())
            .and_then(|builder| builder.add_leaf(2, phone_recovery))
            .and_then(|builder| builder.add_leaf(2, hww_recovery))
            .expect("fixed tree is valid")
            .finalize(&secp, internal_key)
            .expect("fixed tree is complete");
        let leaf_version = LeafVersion::TapScript;
        let control_block = spend_info
            .control_block(&(cooperative.clone(), leaf_version))
            .expect("cooperative leaf is in tree");
        VaultLeaf {
            leaf_hash: TapLeafHash::from_script(&cooperative, leaf_version),
            script: cooperative,
            control_block,
        }
    }
}

fn cooperative_script(phone: XOnlyPublicKey, hww: XOnlyPublicKey) -> ScriptBuf {
    Builder::new()
        .push_x_only_key(&phone)
        .push_opcode(OP_CHECKSIG)
        .push_x_only_key(&hww)
        .push_opcode(OP_CHECKSIGADD)
        .push_int(2)
        .push_opcode(OP_NUMEQUAL)
        .into_script()
}

fn recovery_script(key: XOnlyPublicKey, delay: u16) -> ScriptBuf {
    Builder::new()
        .push_int(i64::from(delay))
        .push_opcode(OP_CSV)
        .push_opcode(OP_VERIFY)
        .push_x_only_key(&key)
        .push_opcode(OP_CHECKSIG)
        .into_script()
}

pub(crate) fn extract_cooperative_keys(
    descriptor: &str,
) -> Option<(XOnlyPublicKey, XOnlyPublicKey)> {
    use miniscript::{descriptor::DescriptorPublicKey, Descriptor};
    use std::str::FromStr;

    let descriptor = Descriptor::<DescriptorPublicKey>::from_str(descriptor).ok()?;
    let derived = descriptor
        .at_derivation_index(0)
        .ok()?
        .derived_descriptor(&Secp256k1::verification_only())
        .ok()?;
    let tr = match derived {
        Descriptor::Tr(tr) => tr,
        _ => return None,
    };
    let script = tr
        .iter_scripts()
        .find_map(|(depth, miniscript)| (depth == 1).then(|| miniscript.encode()))?;
    let bytes = script.as_bytes();
    if bytes.len() != 70
        || bytes[0] != 32
        || bytes[33] != OP_CHECKSIG.to_u8()
        || bytes[34] != 32
        || bytes[67] != OP_CHECKSIGADD.to_u8()
        || bytes[68] != 0x52
        || bytes[69] != OP_NUMEQUAL.to_u8()
    {
        return None;
    }
    Some((
        XOnlyPublicKey::from_slice(&bytes[1..33]).ok()?,
        XOnlyPublicKey::from_slice(&bytes[35..67]).ok()?,
    ))
}

pub(crate) fn estimate_cooperative_vsize(
    transaction: &bitcoin::Transaction,
    policy: &VaultPolicy,
) -> u64 {
    let leaf = policy.cooperative_leaf();
    let mut estimated = transaction.clone();
    for input in &mut estimated.input {
        let mut witness = bitcoin::Witness::new();
        witness.push([0_u8; 64]);
        witness.push([0_u8; 64]);
        witness.push(leaf.script.as_bytes());
        witness.push(leaf.control_block.serialize());
        input.witness = witness;
    }
    estimated.vsize() as u64
}
