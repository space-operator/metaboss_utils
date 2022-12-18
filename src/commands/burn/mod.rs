use anyhow::Result as AnyResult;
use borsh::BorshDeserialize;
use mpl_token_metadata::{
    id,
    instruction::{burn_edition_nft, burn_nft},
    state::{Edition, Metadata, TokenMetadataAccount},
};
pub use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token;
use std::sync::Arc;

use crate::{
    derive::{derive_edition_marker_pda, derive_edition_pda, derive_metadata_pda},
    utils::get_largest_token_account_owner,
};

pub struct BurnArgs<'a> {
    pub client: &'a RpcClient,
    pub keypair: Arc<Keypair>,
    pub mint_pubkey: Pubkey,
}

pub async fn burn<'a>(args: &BurnArgs<'a>) -> AnyResult<(Signature, Transaction)> {
    let assoc = get_associated_token_address(&args.keypair.pubkey(), &args.mint_pubkey);
    let spl_token_program_id = spl_token::id();
    let metadata_pubkey = derive_metadata_pda(&args.mint_pubkey);
    let master_edition = derive_edition_pda(&args.mint_pubkey);

    let md_account = args.client.get_account_data(&metadata_pubkey).await?;
    let metadata = Metadata::deserialize(&mut md_account.as_slice())?;

    // Is it a verified collection item?
    let collection_md = if let Some(collection) = metadata.collection {
        if collection.verified {
            let collection_metadata_pubkey = derive_metadata_pda(&collection.key);
            Some(collection_metadata_pubkey)
        } else {
            None
        }
    } else {
        None
    };

    let burn_ix = burn_nft(
        id(),
        metadata_pubkey,
        args.keypair.pubkey(),
        args.mint_pubkey,
        assoc,
        master_edition,
        spl_token_program_id,
        collection_md,
    );

    let instructions = vec![burn_ix];

    let recent_blockhash = args.client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&args.keypair.pubkey()),
        &[&*args.keypair],
        recent_blockhash,
    );

    let sig = args.client.send_and_confirm_transaction(&tx).await?;

    Ok((sig, tx))
}

pub struct BurnPrintArgs<'a> {
    pub client: &'a RpcClient,
    pub keypair: Arc<Keypair>,
    pub mint_pubkey: Pubkey,
    pub master_mint_pubkey: Pubkey,
}

pub async fn burn_print<'a>(args: BurnPrintArgs<'a>) -> AnyResult<(Signature, Transaction)> {
    let print_edition_token =
        get_associated_token_address(&args.keypair.pubkey(), &args.mint_pubkey);

    // Find the master edition holder.
    let master_edition_owner =
        get_largest_token_account_owner(&args.client, args.master_mint_pubkey).await?;
    let master_edition_token =
        get_associated_token_address(&master_edition_owner, &args.master_mint_pubkey);

    let spl_token_program_id = spl_token::id();
    let metadata_pubkey = derive_metadata_pda(&args.mint_pubkey);

    let master_edition_pda = derive_edition_pda(&args.master_mint_pubkey);
    let print_edition_pda = derive_edition_pda(&args.mint_pubkey);

    let data = args.client.get_account_data(&print_edition_pda).await?;
    let print_edition = Edition::safe_deserialize(data.as_slice())?;

    let edition_marker_pda =
        derive_edition_marker_pda(&args.master_mint_pubkey, print_edition.edition);

    let burn_ix = burn_edition_nft(
        id(),
        metadata_pubkey,
        args.keypair.pubkey(),
        args.mint_pubkey,
        args.master_mint_pubkey,
        print_edition_token,
        master_edition_token,
        master_edition_pda,
        print_edition_pda,
        edition_marker_pda,
        spl_token_program_id,
    );

    let instructions = vec![burn_ix];

    let recent_blockhash = args.client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &instructions,
        Some(&args.keypair.pubkey()),
        &[&*args.keypair],
        recent_blockhash,
    );

    let sig = args.client.send_and_confirm_transaction(&tx).await?;

    Ok((sig, tx))
}
