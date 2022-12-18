use super::common::*;

pub struct SetUpdateAuthorityArgs<'a> {
    pub client: &'a RpcClient,
    pub keypair: Arc<Keypair>,
    pub payer: Arc<Keypair>,
    pub mint_account: Pubkey,
    pub new_authority: Pubkey,
}

pub async fn set_update_authority<'a>(
    args: &SetUpdateAuthorityArgs<'a>,
) -> Result<Transaction, ActionError> {
    let mint_pubkey = &args.mint_account;
    let update_authority = args.keypair.pubkey();
    let new_update_authority = args.new_authority;

    let metadata_account = get_metadata_pda(mint_pubkey);

    let ix = update_metadata_accounts_v2(
        TOKEN_METADATA_PROGRAM_ID,
        metadata_account,
        update_authority,
        Some(new_update_authority),
        None,
        None,
        None,
    );
    let recent_blockhash = args
        .client
        .get_latest_blockhash()
        .await
        .map_err(|e| ActionError::ActionFailed(args.mint_account.to_string(), e.to_string()))?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&args.keypair.pubkey()),
        &[&*args.keypair, &*args.payer],
        recent_blockhash,
    );

    Ok(tx)
}
