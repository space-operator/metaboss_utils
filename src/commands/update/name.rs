use solana_sdk::signature::Signature;

use super::{common::*, update_data};

pub async fn update_name(
    client: &RpcClient,
    keypair: Keypair,
    mint_account: &str,
    new_name: &str,
) -> AnyResult<Signature> {
    let old_md = decode(client, mint_account).await?;
    let data_with_old_name = old_md.data;

    let new_data = DataV2 {
        creators: data_with_old_name.creators,
        seller_fee_basis_points: data_with_old_name.seller_fee_basis_points,
        name: new_name.to_owned(),
        symbol: data_with_old_name.symbol,
        uri: data_with_old_name.uri,
        collection: old_md.collection,
        uses: old_md.uses,
    };

    let mint_account = Pubkey::from_str(&mint_account)?;
    let sig = update_data(client, &keypair, &mint_account, new_data).await?;
    Ok(sig)
}
