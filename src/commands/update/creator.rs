use crate::parse::{keypair::parse_keypair, solana_config::parse_solana_config};

use super::{common::*, update_data};

pub type UpdateResults = Vec<Result<(), ActionError>>;

pub async fn update_creator_by_position(
    client: &RpcClient,
    keypair_path: Option<String>,
    mint_account: &Pubkey,
    new_creators: &str,
    should_append: bool,
) -> AnyResult<Transaction> {
    let solana_opts = parse_solana_config();
    let keypair = parse_keypair(keypair_path, solana_opts)?;

    let old_md = decode(client, mint_account).await?;
    let data_with_old_creators = old_md.data;
    let parsed_creators = parse_cli_creators(new_creators.to_string(), should_append)?;

    let new_creators = if let Some(mut old_creators) = data_with_old_creators.creators {
        if !should_append {
            parsed_creators
        } else {
            let remaining_space = 5 - old_creators.len();
            warn!(
                "Appending {} new creators with old creators with shares of 0",
                parsed_creators.len()
            );
            let end_index = cmp::min(parsed_creators.len(), remaining_space);
            old_creators.append(&mut parsed_creators[0..end_index].to_vec());
            old_creators
        }
    } else {
        parsed_creators
    };

    let shares = new_creators.iter().fold(0, |acc, c| acc + c.share);
    if shares != 100 {
        return Err(anyhow!("Creators shares must sum to 100!"));
    }

    let new_data = DataV2 {
        creators: Some(new_creators),
        seller_fee_basis_points: data_with_old_creators.seller_fee_basis_points,
        name: data_with_old_creators.name,
        symbol: data_with_old_creators.symbol,
        uri: data_with_old_creators.uri,
        collection: old_md.collection,
        uses: old_md.uses,
    };
    let tx = update_data(client, &keypair, mint_account, new_data).await?;
    Ok(tx)
}

pub async fn update_creator(
    client: &RpcClient,
    keypair: Keypair,
    mint_account: Pubkey,
    new_creators: String,
    should_append: bool,
) -> Result<Transaction, ActionError> {
    let old_md = match decode(&client, &mint_account).await {
        Ok(md) => md,
        Err(e) => {
            return Err(ActionError::ActionFailed(
                mint_account.to_string(),
                e.to_string(),
            ))
        }
    };

    let data_with_old_creators = old_md.data;
    let parsed_creators = match parse_cli_creators(new_creators, should_append) {
        Ok(creators) => creators,
        Err(e) => {
            return Err(ActionError::ActionFailed(
                mint_account.to_string(),
                e.to_string(),
            ))
        }
    };

    let new_creators = if let Some(mut old_creators) = data_with_old_creators.creators {
        if !should_append {
            parsed_creators
        } else {
            let remaining_space = 5 - old_creators.len();
            warn!(
                "Appending {} new creators with old creators with shares of 0",
                parsed_creators.len()
            );
            let end_index = cmp::min(parsed_creators.len(), remaining_space);
            old_creators.append(&mut parsed_creators[0..end_index].to_vec());
            old_creators
        }
    } else {
        parsed_creators
    };

    let shares = new_creators.iter().fold(0, |acc, c| acc + c.share);
    if shares != 100 {
        return Err(ActionError::ActionFailed(
            mint_account.to_string(),
            "Creators shares must sum to 100!".to_string(),
        ));
    }

    let new_data = DataV2 {
        creators: Some(new_creators),
        seller_fee_basis_points: data_with_old_creators.seller_fee_basis_points,
        name: data_with_old_creators.name,
        symbol: data_with_old_creators.symbol,
        uri: data_with_old_creators.uri,
        collection: old_md.collection,
        uses: old_md.uses,
    };

    match update_data(&client, &keypair, &mint_account, new_data).await {
        Ok(tx) => Ok(tx),
        Err(e) => Err(ActionError::ActionFailed(
            mint_account.to_string(),
            e.to_string(),
        )),
    }
}
