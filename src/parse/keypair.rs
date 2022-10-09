use anyhow::{anyhow, Context, Result as AnyResult};
use solana_sdk::signer::keypair::Keypair;
use std::fs;

use super::solana_config::SolanaConfig;

pub fn parse_keypair(
    keypair_opt: Option<String>,
    sol_config_option: Option<SolanaConfig>,
) -> AnyResult<Keypair> {
    let keypair = match keypair_opt {
        Some(keypair_path) => read_keypair(&keypair_path).expect("Failed to read keypair file."),
        None => match sol_config_option {
            Some(ref sol_config) => {
                read_keypair(&sol_config.keypair_path).expect("Failed to read keypair file.")
            }
            None => return Err(anyhow!("No keypair provided.")),
        },
    };
    Ok(keypair)
}

pub fn read_keypair(path: &String) -> AnyResult<Keypair> {
    let secret_string: String = fs::read_to_string(path).context("Can't find key file")?;

    // Try to decode the secret string as a JSON array of ints first and then as a base58 encoded string to support Phantom private keys.
    let secret_bytes: Vec<u8> = match serde_json::from_str(&secret_string) {
        Ok(bytes) => bytes,
        Err(_) => match bs58::decode(&secret_string.trim()).into_vec() {
            Ok(bytes) => bytes,
            Err(_) => return Err(anyhow!("Unsupported key type!")),
        },
    };

    let keypair = Keypair::from_bytes(&secret_bytes)?;
    Ok(keypair)
}
