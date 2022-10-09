use anyhow::{anyhow, Result as AnyResult};
use borsh::BorshDeserialize;
use mpl_token_metadata::state::Metadata;
use mpl_token_metadata::state::{Edition, EditionMarker, MasterEditionV2};
use serde::Serialize;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::borsh::try_from_slice_unchecked;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::constants::*;
use crate::derive::*;
use crate::errors::*;

#[derive(Debug, Serialize)]
pub struct JSONCreator {
    pub address: String,
    pub verified: bool,
    pub share: u8,
}

#[derive(Debug, Serialize)]
pub struct JSONCollection {
    pub verified: bool,
    pub key: String,
}

#[derive(Debug, Serialize)]
pub enum JSONCollectionDetails {
    V1 { size: u64 },
}

#[derive(Debug, Serialize)]
pub struct JSONUses {
    pub use_method: String,
    pub remaining: u64,
    pub total: u64,
}

pub async fn decode_master_edition(client: &RpcClient, mint_account: &str) -> AnyResult<()> {
    let master_edition = decode_master_edition_from_mint(client, mint_account).await?;
    log::debug!("{:?}", master_edition);

    Ok(())
}

pub async fn decode_print_edition(client: &RpcClient, mint_account: &str) -> AnyResult<()> {
    let print_edition = decode_edition_from_mint(client, mint_account).await?;
    log::debug!("{:?}", print_edition);

    Ok(())
}

pub async fn decode_edition_marker(
    client: &RpcClient,
    mint_account: &str,
    edition_num: Option<u64>,
    marker_num: Option<u64>,
) -> AnyResult<()> {
    let edition_num = if let Some(num) = edition_num {
        num
    } else if let Some(num) = marker_num {
        num * 248
    } else {
        return Err(anyhow!("Edition or marker number is required"));
    };

    let edition_marker = decode_edition_marker_from_mint(client, mint_account, edition_num).await?;
    log::debug!("{:?}", edition_marker);

    Ok(())
}

pub async fn decode_raw(client: &RpcClient, mint_account: &str) -> Result<Vec<u8>, DecodeError> {
    let pubkey = match Pubkey::from_str(mint_account) {
        Ok(pubkey) => pubkey,
        Err(_) => return Err(DecodeError::PubkeyParseFailed(mint_account.to_string())),
    };
    let metadata_pda = get_metadata_pda(pubkey);

    // let account_data = match retry(
    //     Exponential::from_millis_with_factor(250, 2.0).take(3),
    //     || client.get_account_data(&metadata_pda),
    // ) {
    //     Ok(data) => data,
    //     Err(err) => {
    //         return Err(DecodeError::NetworkError(err.to_string()));
    //     }
    // };
    let account_data = match client.get_account_data(&metadata_pda).await {
        Ok(data) => data,
        Err(err) => {
            return Err(DecodeError::NetworkError(err.to_string()));
        }
    };

    Ok(account_data)
}

pub async fn decode(client: &RpcClient, mint_account: &str) -> Result<Metadata, DecodeError> {
    let pubkey = match Pubkey::from_str(mint_account) {
        Ok(pubkey) => pubkey,
        Err(_) => return Err(DecodeError::PubkeyParseFailed(mint_account.to_string())),
    };
    let metadata_pda = get_metadata_pda(pubkey);

    let account_data = match client.get_account_data(&metadata_pda).await {
        Ok(data) => data,
        Err(err) => {
            return Err(DecodeError::NetworkError(err.to_string()));
        }
    };

    let metadata: Metadata = match Metadata::deserialize(&mut account_data.as_slice()) {
        Ok(m) => m,
        Err(err) => return Err(DecodeError::DecodeMetadataFailed(err.to_string())),
    };

    Ok(metadata)
}

pub fn get_metadata_pda(pubkey: Pubkey) -> Pubkey {
    let metaplex_pubkey = METAPLEX_PROGRAM_ID
        .parse::<Pubkey>()
        .expect("Failed to parse Metaplex Program Id");

    let seeds = &[
        "metadata".as_bytes(),
        metaplex_pubkey.as_ref(),
        pubkey.as_ref(),
    ];

    let (pda, _) = Pubkey::find_program_address(seeds, &metaplex_pubkey);
    pda
}

pub async fn decode_metadata_from_mint(
    client: &RpcClient,
    mint_address: &str,
) -> Result<Metadata, DecodeError> {
    let pubkey = match Pubkey::from_str(mint_address) {
        Ok(pubkey) => pubkey,
        Err(_) => return Err(DecodeError::PubkeyParseFailed(mint_address.to_string())),
    };
    let metadata_pda = derive_metadata_pda(&pubkey);

    let account_data = match client.get_account_data(&metadata_pda).await {
        Ok(data) => data,
        Err(err) => {
            return Err(DecodeError::ClientError(err.kind));
        }
    };

    let metadata: Metadata = match try_from_slice_unchecked(&account_data) {
        Ok(m) => m,
        Err(err) => return Err(DecodeError::DecodeMetadataFailed(err.to_string())),
    };

    Ok(metadata)
}

pub async fn decode_master_edition_from_mint(
    client: &RpcClient,
    mint_address: &str,
) -> Result<MasterEditionV2, DecodeError> {
    let pubkey = match Pubkey::from_str(mint_address) {
        Ok(pubkey) => pubkey,
        Err(_) => return Err(DecodeError::PubkeyParseFailed(mint_address.to_string())),
    };

    let edition_pda = derive_edition_pda(&pubkey);

    let account_data = match client.get_account_data(&edition_pda).await {
        Ok(data) => data,
        Err(err) => {
            return Err(DecodeError::ClientError(err.kind));
        }
    };

    let master_edition: MasterEditionV2 = match try_from_slice_unchecked(&account_data) {
        Ok(e) => e,
        Err(err) => return Err(DecodeError::DecodeMetadataFailed(err.to_string())),
    };

    Ok(master_edition)
}

pub async fn decode_edition_from_mint(
    client: &RpcClient,
    mint_address: &str,
) -> Result<Edition, DecodeError> {
    let pubkey = match Pubkey::from_str(mint_address) {
        Ok(pubkey) => pubkey,
        Err(_) => return Err(DecodeError::PubkeyParseFailed(mint_address.to_string())),
    };

    let edition_pda = derive_edition_pda(&pubkey);

    let account_data = match client.get_account_data(&edition_pda).await {
        Ok(data) => data,
        Err(err) => {
            return Err(DecodeError::ClientError(err.kind));
        }
    };

    let edition: Edition = match try_from_slice_unchecked(&account_data) {
        Ok(e) => e,
        Err(err) => return Err(DecodeError::DecodeMetadataFailed(err.to_string())),
    };

    Ok(edition)
}

pub async fn decode_edition_marker_from_mint(
    client: &RpcClient,
    mint_address: &str,
    edition_num: u64,
) -> Result<EditionMarker, DecodeError> {
    let pubkey = match Pubkey::from_str(mint_address) {
        Ok(pubkey) => pubkey,
        Err(_) => return Err(DecodeError::PubkeyParseFailed(mint_address.to_string())),
    };

    let edition_marker_pda = derive_edition_marker_pda(&pubkey, edition_num);

    let account_data = match client.get_account_data(&edition_marker_pda).await {
        Ok(data) => data,
        Err(err) => {
            return Err(DecodeError::ClientError(err.kind));
        }
    };

    let edition_marker: EditionMarker = match try_from_slice_unchecked(&account_data) {
        Ok(e) => e,
        Err(err) => return Err(DecodeError::DecodeMetadataFailed(err.to_string())),
    };

    Ok(edition_marker)
}
