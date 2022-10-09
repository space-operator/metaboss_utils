use solana_sdk::signature::Signature;

use super::common::*;

pub struct SetImmutableArgs<'a> {
    pub client: &'a RpcClient,
    pub keypair: Arc<Keypair>,
    pub mint_account: Pubkey,
}

pub async fn set_immutable<'a>(args: SetImmutableArgs<'a>) -> Result<Signature, ActionError> {
    let mint_pubkey = args.mint_account;
    let update_authority = args.keypair.pubkey();
    let metadata_account = get_metadata_pda(&mint_pubkey);

    let ix = update_metadata_accounts_v2(
        TOKEN_METADATA_PROGRAM_ID,
        metadata_account,
        update_authority,
        None,
        None,
        None,
        Some(false),
    );
    let recent_blockhash = args
        .client
        .get_latest_blockhash()
        .await
        .map_err(|e| ActionError::ActionFailed(args.mint_account.to_string(), e.to_string()))?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&update_authority),
        &[&*args.keypair],
        recent_blockhash,
    );

    let sig = args
        .client
        .send_and_confirm_transaction(&tx)
        .await
        .map_err(|e| ActionError::ActionFailed(args.mint_account.to_string(), e.to_string()))?;

    Ok(sig)
}
