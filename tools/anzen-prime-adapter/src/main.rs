use anzen_policy_engine::{AnzenIdentity, PolicyPackage, MAX_PACKAGE_BYTES};
use bitcoin::Network;
use std::{
    env, fs,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn main() {
    if run().is_err() {
        eprintln!("Policy approval failed; no approved package was written.");
        std::process::exit(1);
    }
}

fn run() -> Result<(), ()> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 4 || args[0] != "approve" {
        return Err(());
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    if output.exists() {
        return Err(());
    }
    let seed_text = args[3].to_str().ok_or(())?;
    let mut seed = decode_seed(seed_text)?;
    let result = approve_file(&input, &output, &seed);
    seed.fill(0);
    result
}

fn approve_file(input: &Path, output: &Path, seed: &[u8; 32]) -> Result<(), ()> {
    let bytes = read_bounded(input)?;
    let package = PolicyPackage::parse_bounded(&bytes).map_err(|_| ())?;
    let network = match package.manifest.network.as_str() {
        "regtest" => Network::Regtest,
        "bitcoin" => Network::Bitcoin,
        _ => return Err(()),
    };
    let identity = AnzenIdentity::from_app_seed(seed, network).map_err(|_| ())?;
    let validated = package.validate(identity.public_key()).map_err(|_| ())?;
    let summary = validated.summary().map_err(|_| ())?;
    let psbt_count = validated.psbt_count();
    let approved = validated.approve(&identity).map_err(|_| ())?;
    let json = approved.to_json().map_err(|_| ())?;
    atomic_write_new(output, &json)?;

    println!("Validated and approved Anzen PolicyPackage v4");
    println!("Network: {}", summary.network);
    println!("Vault address: {}", summary.vault_address);
    println!("Total input: {} sats", summary.total_input_sats);
    println!("Monthly limit: {} sats", summary.monthly_limit_sats);
    println!("Allowance steps: {}", summary.allowance_count);
    println!(
        "Emergency access: {} sats",
        summary.emergency_access_limit_sats
    );
    println!("Fee rate: {} sat/vB", summary.fee_rate_sat_vb);
    println!("Signed PSBTs: {psbt_count}");
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>, ()> {
    let file = fs::File::open(path).map_err(|_| ())?;
    let mut bytes = Vec::with_capacity(MAX_PACKAGE_BYTES.min(64 * 1024));
    file.take((MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err(());
    }
    Ok(bytes)
}

fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<(), ()> {
    let parent = path.parent().ok_or(())?;
    let name = path.file_name().and_then(|name| name.to_str()).ok_or(())?;
    let temporary = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| ())?;
        file.write_all(bytes).map_err(|_| ())?;
        file.sync_all().map_err(|_| ())?;
        fs::rename(&temporary, path).map_err(|_| ())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

fn decode_seed(text: &str) -> Result<[u8; 32], ()> {
    if text.len() != 64 {
        return Err(());
    }
    let mut seed = [0_u8; 32];
    for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        seed[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(seed)
}

fn hex_nibble(value: u8) -> Result<u8, ()> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(()),
    }
}
