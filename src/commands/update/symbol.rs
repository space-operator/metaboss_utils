use super::{common::*, update_data};

pub struct UpdateSymbolAllArgs {
    pub client: RpcClient,
    pub keypair: Option<String>,
    pub mint_list: Option<String>,
    pub cache_file: Option<String>,
    pub new_symbol: String,
    pub batch_size: usize,
    pub retries: u8,
}

pub struct UpdateSymbolArgs {
    pub client: Arc<RpcClient>,
    pub keypair: Arc<Keypair>,
    pub mint_account: Pubkey,
    pub new_symbol: String,
}

pub async fn update_symbol(args: UpdateSymbolArgs) -> Result<(), ActionError> {
    let old_md = decode(&args.client, &args.mint_account)
        .await
        .map_err(|e| ActionError::ActionFailed(args.mint_account.to_string(), e.to_string()))?;
    let data_with_old_symbol = old_md.data;

    let new_data = DataV2 {
        creators: data_with_old_symbol.creators,
        seller_fee_basis_points: data_with_old_symbol.seller_fee_basis_points,
        name: data_with_old_symbol.name,
        symbol: args.new_symbol.to_owned(),
        uri: data_with_old_symbol.uri,
        collection: old_md.collection,
        uses: old_md.uses,
    };

    let mint_account = args.mint_account;
    update_data(&args.client, &args.keypair, &mint_account, new_data)
        .await
        .map_err(|e| ActionError::ActionFailed(args.mint_account.to_string(), e.to_string()))?;
    Ok(())
}
