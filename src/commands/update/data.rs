use anyhow::Result;
use log::info;
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
    mint_account: &str,
    data: DataV2,
) -> Result<()> {
    let program_id = Pubkey::from_str(METAPLEX_PROGRAM_ID)?;
    let mint_pubkey = Pubkey::from_str(mint_account)?;
    let metadata_account = get_metadata_pda(mint_pubkey);

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

    let sig = client.send_and_confirm_transaction(&tx).await?;

    info!("Mint: {:?}, Tx sig: {:?}", mint_account, sig);

    Ok(())
}