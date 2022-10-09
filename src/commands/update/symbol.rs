use solana_sdk::signature::Signature;

use super::{common::*, update_data};

pub async fn update_symbol(
    client: &RpcClient,
    keypair: Keypair,
    mint_account: &Pubkey,
    new_symbol: &str,
) -> AnyResult<Signature> {
    let old_md = decode(&client, &mint_account)
        .await
        .map_err(|e| ActionError::ActionFailed(mint_account.to_string(), e.to_string()))?;
    let data_with_old_symbol = old_md.data;

    let new_data = DataV2 {
        creators: data_with_old_symbol.creators,
        seller_fee_basis_points: data_with_old_symbol.seller_fee_basis_points,
        name: data_with_old_symbol.name,
        symbol: new_symbol.to_owned(),
        uri: data_with_old_symbol.uri,
        collection: old_md.collection,
        uses: old_md.uses,
    };

    let mint_account = mint_account;
    let sig = update_data(&client, &keypair, &mint_account, new_data)
        .await
        .map_err(|e| ActionError::ActionFailed(mint_account.to_string(), e.to_string()))?;
    Ok(sig)
}
