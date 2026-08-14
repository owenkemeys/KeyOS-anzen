use anzen_policy_engine::{
    AnzenIdentity, CloudRecoveryBackup, CooperativeSweepPackage, DeviceFile, PhoneRotationPackage,
    PolicyPackage, VaultConfig, MAX_PACKAGE_BYTES,
};
use bitcoin::Network;
use std::{
    env, fs,
    fs::OpenOptions,
    io::{Read, Write},
    path::{Path, PathBuf},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("Anzen approval failed: {error}; no approved package was written.");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if matches!(
        args.first().and_then(|value| value.to_str()),
        Some("approve-rotation")
    ) {
        return approve_rotation_command(&args);
    }
    if args.len() != 4 {
        return Err("invalid command arguments".into());
    }
    let input = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    if output.exists() {
        return Err("output file already exists".into());
    }
    let seed_text = args[3]
        .to_str()
        .ok_or_else(|| "development seed is not UTF-8".to_owned())?;
    let mut seed = decode_seed(seed_text)?;
    let result = match args[0].to_str() {
        Some("approve") => approve_file(&input, &output, &seed),
        Some("approve-sweep") => approve_sweep_file(&input, &output, &seed),
        _ => Err("invalid command arguments".into()),
    };
    seed.fill(0);
    result
}

fn approve_rotation_command(args: &[std::ffi::OsString]) -> Result<(), String> {
    if args.len() != 7 {
        return Err("invalid command arguments".into());
    }
    let proposal = PathBuf::from(&args[1]);
    let config = PathBuf::from(&args[2]);
    let pending = PathBuf::from(&args[3]);
    let backup = PathBuf::from(&args[4]);
    let output = PathBuf::from(&args[5]);
    if output.exists() {
        return Err("output file already exists".into());
    }
    let seed_text = args[6]
        .to_str()
        .ok_or_else(|| "development seed is not UTF-8".to_owned())?;
    let mut seed = decode_seed(seed_text)?;
    let result = approve_rotation_files(&proposal, &config, &pending, &backup, &output, &seed);
    seed.fill(0);
    result
}

fn approve_rotation_files(
    proposal_path: &Path,
    config_path: &Path,
    pending_path: &Path,
    backup_path: &Path,
    output: &Path,
    seed: &[u8; 32],
) -> Result<(), String> {
    let package = PhoneRotationPackage::parse_bounded(&read_bounded(proposal_path)?)
        .map_err(|error| error.to_string())?;
    let current = VaultConfig::parse_bounded(&read_bounded(config_path)?)
        .map_err(|error| error.to_string())?;
    let network = match current.network.as_str() {
        "regtest" => Network::Regtest,
        "bitcoin" | "mainnet" => Network::Bitcoin,
        _ => return Err("unsupported Bitcoin network".into()),
    };
    let pending = DeviceFile::parse_bounded(&read_bounded(pending_path)?)
        .map_err(|error| error.to_string())?;
    let backup = CloudRecoveryBackup::parse_bounded(&read_bounded(backup_path)?)
        .map_err(|error| error.to_string())?;
    let reviewed = package
        .review(current, pending, backup)
        .map_err(|error| error.to_string())?;
    let summary = reviewed.summary();
    let identity =
        AnzenIdentity::from_app_seed(seed, network).map_err(|error| error.to_string())?;
    let approved = reviewed
        .approve(&identity)
        .map_err(|error| error.to_string())?;
    let sweep_signatures = approved.sweep_signature_count();
    let policy_signatures = approved.policy_signature_count();
    let json = approved.to_json().map_err(|error| error.to_string())?;
    atomic_write_new(output, &json)?;

    println!("Validated and approved Anzen phone-key rotation v1");
    println!("New vault address: {}", summary.new_vault_address);
    println!("Inputs: {}", summary.input_count);
    println!("Sent: {} sats", summary.sent_sats);
    println!("Fee: {} sats", summary.fee_sats);
    println!("Monthly limit: {} sats", summary.monthly_limit_sats);
    println!(
        "Emergency access: {} sats",
        summary.emergency_access_limit_sats
    );
    println!(
        "Recovery friends preserved: {}",
        summary.recovery_friend_count
    );
    println!("Sweep HWW signatures: {sweep_signatures}");
    println!("Policy HWW signatures: {policy_signatures}");
    Ok(())
}

fn approve_sweep_file(input: &Path, output: &Path, seed: &[u8; 32]) -> Result<(), String> {
    let bytes = read_bounded(input)?;
    let package =
        CooperativeSweepPackage::parse_bounded(&bytes).map_err(|error| error.to_string())?;
    let summary = package.summary().map_err(|error| error.to_string())?;
    let network = match summary.network.as_str() {
        "regtest" => Network::Regtest,
        "bitcoin" => Network::Bitcoin,
        _ => return Err("unsupported Bitcoin network".into()),
    };
    let identity =
        AnzenIdentity::from_app_seed(seed, network).map_err(|error| error.to_string())?;
    let approved = package
        .validate(identity.public_key())
        .and_then(|validated| validated.approve(&identity))
        .map_err(|error| error.to_string())?;
    let signatures = approved.hww_signature_count();
    let json = approved.to_json().map_err(|error| error.to_string())?;
    atomic_write_new(output, &json)?;

    println!("Validated and approved Anzen cooperative sweep v1");
    println!("Network: {}", summary.network);
    println!("Destination: {}", summary.destination);
    println!("Inputs: {}", summary.input_count);
    println!("Sent: {} sats", summary.sent_sats);
    println!("Fee: {} sats", summary.fee_sats);
    println!("HWW signatures: {signatures}");
    Ok(())
}

fn approve_file(input: &Path, output: &Path, seed: &[u8; 32]) -> Result<(), String> {
    let bytes = read_bounded(input)?;
    let package = PolicyPackage::parse_bounded(&bytes).map_err(|error| error.to_string())?;
    let network = match package.manifest.network.as_str() {
        "regtest" => Network::Regtest,
        "bitcoin" => Network::Bitcoin,
        _ => return Err("unsupported Bitcoin network".into()),
    };
    let identity =
        AnzenIdentity::from_app_seed(seed, network).map_err(|error| error.to_string())?;
    let validated = package
        .validate(identity.public_key())
        .map_err(|error| error.to_string())?;
    let summary = validated.summary().map_err(|error| error.to_string())?;
    let psbt_count = validated.psbt_count();
    let approved = validated
        .approve(&identity)
        .map_err(|error| error.to_string())?;
    let json = approved.to_json().map_err(|error| error.to_string())?;
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

fn read_bounded(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|_| "unable to open input file".to_owned())?;
    let mut bytes = Vec::with_capacity(MAX_PACKAGE_BYTES.min(64 * 1024));
    file.take((MAX_PACKAGE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "unable to read input file".to_owned())?;
    if bytes.len() > MAX_PACKAGE_BYTES {
        return Err("policy package exceeds the 128 KiB import limit".into());
    }
    Ok(bytes)
}

fn atomic_write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "output path has no parent".to_owned())?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "invalid output file name".to_owned())?;
    let temporary = parent.join(format!(".{name}.{}.tmp", std::process::id()));
    let write_result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| "unable to create temporary output file".to_owned())?;
        file.write_all(bytes)
            .map_err(|_| "unable to write temporary output file".to_owned())?;
        file.sync_all()
            .map_err(|_| "unable to sync temporary output file".to_owned())?;
        fs::rename(&temporary, path).map_err(|_| "unable to publish output file".to_owned())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

fn decode_seed(text: &str) -> Result<[u8; 32], String> {
    if text.len() != 64 {
        return Err("development seed must contain 64 hexadecimal characters".into());
    }
    let mut seed = [0_u8; 32];
    for (index, pair) in text.as_bytes().chunks_exact(2).enumerate() {
        seed[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
    }
    Ok(seed)
}

fn hex_nibble(value: u8) -> Result<u8, String> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err("development seed contains a non-hexadecimal character".into()),
    }
}
