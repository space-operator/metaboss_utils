use super::common::*;
use super::data::*;
use crate::commands::theindexio::THE_INDEX_MAINNET;
use crate::derive::derive_metadata_pda;
use borsh::BorshDeserialize;

pub async fn get_collection_items(
    collection_mint: String,
    method: GetCollectionItemsMethods,
    api_key: Option<String>,
) -> AnyResult<String> {
    match method {
        GetCollectionItemsMethods::TheIndexIO => {
            if let Some(key) = api_key {
                return Ok(get_collection_items_by_the_index_io(collection_mint, key).await?);
            } else {
                return Err(anyhow!(
                    "This method requires an index key for TheIndex.io."
                ));
            }
        }
    }
}

pub async fn get_collection_items_by_the_index_io(
    collection_mint: String,
    api_key: String,
) -> AnyResult<String> {
    let jrpc = JRPCRequest::new("getNFTsByCollection", vec![collection_mint.clone()]);
    let url = format!("{THE_INDEX_MAINNET}/{api_key}");
    let client = reqwest::Client::new();

    let response = client.post(url).json(&jrpc).send().await?;

    let res: RpcResponse = response.json().await?;

    let mut mints: Vec<String> = res
        .result
        .iter()
        .map(|nft| nft.metadata.mint.clone())
        .collect();

    mints.sort_unstable();

    Ok(serde_json::to_string_pretty(&mints)?)
}

pub async fn check_collection_items(
    async_client: RpcClient,
    collection_mint: String,
    mut mint_list: Vec<String>,
) -> AnyResult<()> {
    let mint_list_length = mint_list.len();

    let mut collections: HashMap<String, Vec<String>> = HashMap::new();
    let mut handles = Vec::new();
    let mut errors = Vec::new();

    let client = Arc::new(async_client);

    for mint in mint_list.drain(0..cmp::min(mint_list.len(), PARALLEL_LIMIT)) {
        let client = client.clone();
        handles.push(tokio::spawn(async move {
            get_mint_collection(&client, mint.to_string()).await
        }));
    }

    while !handles.is_empty() {
        match select_all(handles).await {
            (Ok(res), _index, remaining) => {
                handles = remaining;

                if res.is_ok() {
                    let (mint, collection_opt) = res.unwrap();
                    match collection_opt {
                        Some(collection) => {
                            collections
                                .entry(collection.key.to_string())
                                .or_insert_with(Vec::new)
                                .push(mint.to_string());
                        }
                        None => {
                            collections
                                .entry("none".to_string())
                                .or_insert_with(Vec::new)
                                .push(mint.to_string());
                        }
                    }
                } else {
                    errors.push(res.err().unwrap());
                }
            }
            (Err(err), _index, remaining) => {
                errors.push(err.into());
                // ignoring all errors
                handles = remaining;
            }
        }

        if !mint_list.is_empty() {
            // if we are half way through, let spawn more transactions
            if (PARALLEL_LIMIT - handles.len()) > (PARALLEL_LIMIT / 2) {
                // syncs cache (checkpoint)

                for mint in mint_list.drain(0..cmp::min(mint_list.len(), PARALLEL_LIMIT)) {
                    let client = client.clone();
                    handles.push(tokio::spawn(async move {
                        get_mint_collection(&client, mint.to_string()).await
                    }));
                }
            }
        }
    }

    let mint_items = collections.get(&collection_mint).ok_or_else(|| {
        anyhow!("No mints found for this parent. Run with --debug to see more details.")
    })?;
    let keys: Vec<&String> = collections.keys().collect();

    // Check if there's the only one and correct collection parent associated with the mint list and that all items in the list belong to it.
    if !keys.contains(&&collection_mint) || keys.len() != 1 || mint_items.len() != mint_list_length
    {
        return Err(anyhow!("Not all mints from the list belong to this parent. Run with --debug to see more details."));
    }

    println!("All mints in are the collection!");
    Ok(())
}

async fn get_mint_collection<'a>(
    client: &RpcClient,
    mint: String,
) -> AnyResult<(String, Option<MdCollection>)> {
    let mint_pubkey = Pubkey::from_str(&mint)?;
    let metadata_pubkey = derive_metadata_pda(&mint_pubkey);
    let data = client.get_account_data(&metadata_pubkey).await?;
    let md = <Metadata as BorshDeserialize>::deserialize(&mut data.as_slice())?;

    Ok((mint, md.collection))
}
