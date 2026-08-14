use crate::PolicyError;
use bip39::{Language, Mnemonic};
use bitcoin::{
    bip32::{DerivationPath, Xpriv},
    secp256k1::{Keypair, Secp256k1, XOnlyPublicKey},
    Network,
};
use std::str::FromStr;
use zeroize::Zeroize;

/// Anzen's HWW identity derived only inside the app from KeyOS-isolated seed material.
///
/// KeyOS supplies a different app seed to each app, providing the outer domain separation.
/// Within Anzen, the seed is interpreted as BIP39 entropy and follows Luke's exact BIP86
/// vault-key path so approved packages remain protocol-compatible.
pub struct AnzenIdentity {
    pub(crate) keypair: Keypair,
    public_key: XOnlyPublicKey,
}

impl AnzenIdentity {
    pub fn from_app_seed(app_seed: &[u8; 32], network: Network) -> Result<Self, PolicyError> {
        let coin_type = match network {
            Network::Bitcoin => 0,
            Network::Regtest => 1,
            _ => return Err(PolicyError::KeyDerivation),
        };
        let mnemonic = Mnemonic::from_entropy_in(Language::English, app_seed)
            .map_err(|_| PolicyError::KeyDerivation)?;
        let mut bip39_seed = mnemonic.to_seed_normalized("");
        let result = (|| {
            let secp = Secp256k1::new();
            let master =
                Xpriv::new_master(network, &bip39_seed).map_err(|_| PolicyError::KeyDerivation)?;
            let path = DerivationPath::from_str(&format!("m/86'/{coin_type}'/100'/0/0"))
                .map_err(|_| PolicyError::KeyDerivation)?;
            let vault = master
                .derive_priv(&secp, &path)
                .map_err(|_| PolicyError::KeyDerivation)?;
            let keypair = Keypair::from_secret_key(&secp, &vault.private_key);
            let (public_key, _) = keypair.x_only_public_key();
            Ok(Self {
                keypair,
                public_key,
            })
        })();
        bip39_seed.zeroize();
        result
    }

    pub fn public_key(&self) -> XOnlyPublicKey {
        self.public_key
    }
}
