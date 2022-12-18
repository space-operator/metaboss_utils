use anyhow::Result;
use mpl_token_metadata::{instruction::update_metadata_accounts_v2, state::DataV2};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use std::str::FromStr;

use crate::commands::decode::get_metadata_pda;
use crate::constants::*;

pub async fn update_data(
    client: &RpcClient,
    keypair: &Keypair,
    mint_account: &Pubkey,
    data: DataV2,
) -> Result<Transaction> {
    let program_id = Pubkey::from_str(METAPLEX_PROGRAM_ID)?;
    let metadata_account = get_metadata_pda(mint_account);

    let update_authority = keypair.pubkey();

    let ix = update_metadata_accounts_v2(
        program_id,
        metadata_account,
        update_authority,
        None,
        Some(data),
        None,
        None,
    );
    let recent_blockhash = client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&update_authority),
        &[keypair],
        recent_blockhash,
    );

    Ok(tx)
}
