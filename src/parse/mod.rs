use anyhow::{anyhow, Result};
use mpl_token_metadata::state::{Creator, Data};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

use crate::data::{NFTCreator, NFTData};
use crate::utils::find_errors;

pub mod keypair;
pub mod solana_config;

pub fn creator_is_verified(creators_opt: &Option<Vec<Creator>>, position: usize) -> bool {
    // Only add mints with a verified creator.
    if let Some(creators) = creators_opt {
        if creators[position].verified {
            return true;
        }
    }
    false
}

fn convert_creator(c: &NFTCreator) -> Result<Creator> {
    Ok(Creator {
        address: Pubkey::from_str(&c.address)?,
        verified: c.verified,
        share: c.share,
    })
}

pub fn parse_creators(creators_json: &Value) -> Result<Vec<NFTCreator>> {
    let mut creators = Vec::new();

    for creator in creators_json
        .as_array()
        .ok_or_else(|| anyhow!("Invalid creators array!"))?
    {
        let address = creator
            .get("address")
            .ok_or_else(|| anyhow!("Invalid address!"))?
            .as_str()
            .ok_or_else(|| anyhow!("Invalid address!"))?
            .to_string();
        let share = creator
            .get("share")
            .ok_or_else(|| anyhow!("Invalid share!"))?
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid share!"))? as u8;
        creators.push(NFTCreator {
            address,
            verified: false,
            share,
        });
    }
    Ok(creators)
}

pub fn parse_name(body: &Value) -> Result<String> {
    let name = body
        .get("name")
        .ok_or_else(|| anyhow!("Invalid name!"))?
        .as_str()
        .ok_or_else(|| anyhow!("Invalid name!"))?
        .to_string();
    Ok(name)
}

pub fn parse_symbol(body: &Value) -> Result<String> {
    let symbol = body
        .get("symbol")
        .ok_or_else(|| anyhow!("Invalid symbol!"))?
        .as_str()
        .ok_or_else(|| anyhow!("Invalid symbol!"))?
        .to_string();
    Ok(symbol)
}

pub fn parse_seller_fee_basis_points(body: &Value) -> Result<u16> {
    let seller_fee_basis_points =
        body.get("seller_fee_basis_points")
            .ok_or_else(|| anyhow!("Invalid seller_fee_basis_points!"))?
            .as_u64()
            .ok_or_else(|| anyhow!("Invalid seller_fee_basis_points!"))? as u16;
    Ok(seller_fee_basis_points)
}

pub fn convert_local_to_remote_data(local: NFTData) -> Result<Data> {
    let creators = match local.creators {
        Some(nft_creators) => Some(
            nft_creators
                .iter()
                .map(convert_creator)
                .collect::<Result<Vec<_>>>()?,
        ),
        _ => None,
    };

    let data = Data {
        name: local.name,
        symbol: local.symbol,
        uri: local.uri,
        seller_fee_basis_points: local.seller_fee_basis_points,
        creators,
    };
    Ok(data)
}

pub fn is_only_one_option<T, U>(option1: &Option<T>, option2: &Option<U>) -> bool {
    match (option1, option2) {
        (Some(_), None) | (None, Some(_)) => true,
        (Some(_), Some(_)) => false,
        (None, None) => false,
    }
}

pub fn parse_cli_creators(new_creators: String, should_append: bool) -> Result<Vec<Creator>> {
    let mut creators = Vec::new();

    for nc in new_creators.split(',') {
        let mut c = nc.split(':');
        let address = c.next().ok_or_else(|| anyhow!("Missing address!"))?;
        let address = Pubkey::from_str(address)
            .map_err(|_| anyhow!(format!("Invalid creator address: {:?}!", address)))?;
        let share = if should_append {
            c.next();
            0u8
        } else {
            c.next()
                .ok_or_else(|| anyhow!("Invalid creator share, must be 0-100!"))?
                .parse::<u8>()?
        };
        let verified = c
            .next()
            .ok_or_else(|| anyhow!("Missing creator verified: must be 'true' or 'false'!"))?
            .parse::<bool>()?;
        creators.push(Creator {
            address,
            share,
            verified,
        });
    }

    if creators.len() > 5 {
        return Err(anyhow!("Too many creators: maximum of five!"));
    }

    Ok(creators)
}

pub fn parse_errors_code(error_code: &str) -> Result<()> {
    let parsed_error_code = if error_code.contains("0x") {
        error_code.replace("0x", "")
    } else {
        format!("{:X}", error_code.parse::<i64>()?)
    };

    let errors = find_errors(&parsed_error_code);

    if errors.is_empty() {
        return Err(anyhow!("Invalid Error Code"));
    }

    for error in errors {
        log::debug!("\t{:<10} |\t{}", error.domain, error.message);
    }
    Ok(())
}
