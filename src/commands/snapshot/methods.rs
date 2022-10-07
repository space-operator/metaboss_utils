use super::common::*;
use super::data::*;

use crate::commands::theindexio;
use crate::commands::theindexio::GPAResult;
use crate::derive::derive_cmv2_pda;
use crate::parse::{creator_is_verified, is_only_one_option};
use crate::{commands::decode::get_metadata_pda, constants::*};

pub async fn snapshot_mints(client: &RpcClient, args: SnapshotMintsArgs) -> Result<Vec<String>> {
    if !is_only_one_option(&args.creator, &args.update_authority) {
        return Err(anyhow!(
            "Please specify either a candy machine id or an update authority, but not both."
        ));
    }

    let mut mint_addresses = get_mint_accounts(
        client,
        &args.creator,
        args.position,
        args.update_authority,
        args.allow_unverified,
        args.v2,
    )
    .await?;

    mint_addresses.sort_unstable();

    Ok(mint_addresses)
}

pub async fn snapshot_indexed_mints(api_key: String, creator: &str) -> Result<Vec<String>> {
    let results =
        crate::commands::theindexio::get_verified_creator_accounts(api_key, creator).await?;

    let mut mint_addresses = Vec::new();

    for result in results {
        let bs64_data = &result.account.data.as_array().unwrap()[0];
        let data = base64::decode(bs64_data.as_str().unwrap())?;
        let metadata: Metadata = match try_from_slice_unchecked(&data) {
            Ok(metadata) => metadata,
            Err(_) => {
                error!("Failed to parse metadata for account {}", result.pubkey);
                continue;
            }
        };
        mint_addresses.push(metadata.mint.to_string());
    }

    mint_addresses.sort_unstable();

    Ok(mint_addresses)
}

pub async fn get_mint_accounts(
    client: &RpcClient,
    creator: &Option<String>,
    position: usize,
    update_authority: Option<String>,
    allow_unverified: bool,
    v2: bool,
) -> Result<Vec<String>> {
    let accounts = if let Some(ref update_authority) = update_authority {
        get_mints_by_update_authority(client, update_authority).await?
    } else if let Some(ref creator) = creator {
        // Support v2 cm ids
        if v2 {
            let creator_pubkey =
                Pubkey::from_str(creator).expect("Failed to parse pubkey from creator!");
            let cmv2_creator = derive_cmv2_pda(&creator_pubkey);
            get_cm_creator_accounts(client, &cmv2_creator.to_string(), position).await?
        } else {
            get_cm_creator_accounts(client, creator, position).await?
        }
    } else {
        return Err(anyhow!(
            "Please specify either a candy machine id or an update authority, but not both."
        ));
    };

    info!("Getting metadata and writing to file...");
    let mut mint_accounts: Vec<String> = Vec::new();

    for (pubkey, account) in accounts {
        let metadata: Metadata = match try_from_slice_unchecked(&account.data) {
            Ok(metadata) => metadata,
            Err(_) => {
                error!("Failed to parse metadata for account {}", pubkey);
                continue;
            }
        };

        if creator_is_verified(&metadata.data.creators, position) || allow_unverified {
            mint_accounts.push(metadata.mint.to_string());
        }
    }

    Ok(mint_accounts)
}

pub async fn snapshot_holders(
    client: &RpcClient,
    args: SnapshotHoldersArgs,
) -> Result<Vec<Holder>> {
    let accounts = if let Some(ref update_authority) = args.update_authority {
        get_mints_by_update_authority(client, update_authority).await?
    } else if let Some(ref creator) = args.creator {
        // Support v2 cm ids
        if args.v2 {
            let creator_pubkey =
                Pubkey::from_str(creator).expect("Failed to parse pubkey from creator!");
            let cmv2_creator = derive_cmv2_pda(&creator_pubkey);
            get_cm_creator_accounts(client, &cmv2_creator.to_string(), args.position).await?
        } else {
            get_cm_creator_accounts(client, creator, args.position).await?
        }
    } else if let Some(ref _mint_accounts_file) = args.mint_accounts_file {
        unimplemented!();
        /*
        let file = File::open(mint_accounts_file)?;
        let mint_accounts: Vec<String> = serde_json::from_reader(&file)?;
        get_mint_account_infos(client, mint_accounts).await?
        */
    } else {
        return Err(anyhow!(
            "Must specify either --update-authority or --candy-machine-id or --mint-accounts-file"
        ));
    };

    info!("Finding current holders...");
    let nft_holders: Arc<Mutex<Vec<Holder>>> = Arc::new(Mutex::new(Vec::new()));

    for (metadata_pubkey, account) in accounts.iter() {
        let nft_holders = nft_holders.clone();

        let metadata: Metadata = match try_from_slice_unchecked(&account.data) {
            Ok(metadata) => metadata,
            Err(_) => {
                error!("Account {} has no metadata", metadata_pubkey);
                continue;
            }
        };

        // Check that first creator is verified
        if !creator_is_verified(&metadata.data.creators, args.position) && !args.allow_unverified {
            continue;
        }

        // let token_accounts = match retry(
        //     Exponential::from_millis_with_factor(250, 2.0).take(3),
        //     async {get_holder_token_accounts(client, metadata.mint.to_string()).await},
        // ) {
        //     Ok(token_accounts) => token_accounts,
        //     Err(_) => {
        //         error!("Account {} has no token accounts", metadata_pubkey);
        //         continue;
        //     }
        // };

        let token_accounts =
            match get_holder_token_accounts(client, metadata.mint.to_string()).await {
                Ok(token_accounts) => token_accounts,
                Err(_) => {
                    error!("Account {} has no token accounts", metadata_pubkey);
                    continue;
                }
            };

        for (associated_token_address, account) in token_accounts {
            let data = match parse_account_data(
                &metadata.mint,
                &TOKEN_PROGRAM_ID,
                &account.data,
                Some(AccountAdditionalData {
                    spl_token_decimals: Some(0),
                }),
            ) {
                Ok(data) => data,
                Err(err) => {
                    error!("Account {} has no data: {}", associated_token_address, err);
                    continue;
                }
            };

            let amount = match parse_token_amount(&data) {
                Ok(amount) => amount,
                Err(err) => {
                    error!(
                        "Account {} has no amount: {}",
                        associated_token_address, err
                    );
                    continue;
                }
            };

            // Only include current holder of the NFT.
            if amount == 1 {
                let owner_wallet = match parse_owner(&data) {
                    Ok(owner_wallet) => owner_wallet,
                    Err(err) => {
                        error!("Account {} has no owner: {}", associated_token_address, err);
                        continue;
                    }
                };
                let associated_token_address = associated_token_address.to_string();
                let holder = Holder {
                    owner_wallet,
                    associated_token_address,
                    mint_account: metadata.mint.to_string(),
                    metadata_account: metadata_pubkey.to_string(),
                };
                nft_holders.lock().unwrap().push(holder);
            }
        }
    }

    nft_holders.lock().unwrap().sort_unstable();
    let nft_holders = nft_holders.lock().unwrap().clone();

    Ok(nft_holders)
}

pub async fn snapshot_indexed_holders(api_key: String, creator: &str) -> Result<Vec<Holder>> {
    info!("creator: {}", creator);
    let md_results = theindexio::get_verified_creator_accounts(api_key.clone(), creator).await?;

    info!("Found {} mints", md_results.len());

    // Create a vector of futures to execute.
    let mut tasks = Vec::new();
    for md in md_results {
        tasks.push(tokio::spawn(get_holder_from_gpa_result(
            api_key.clone(),
            md,
        )));
    }

    log::debug!("Tasks created: {}", tasks.len());

    // Wait for all the tasks to resolve and push the results to our results vector
    let mut task_results = Vec::new();

    for task in tasks {
        task_results.push(task.await.unwrap());
    }

    // Partition decode results.
    let (successful_results, failed_results): (HolderResults, HolderResults) =
        task_results.into_iter().partition(Result::is_ok);
    info!("Found {} successful results", successful_results.len());
    error!("Found {} failed results", failed_results.len());

    if !failed_results.is_empty() {
        error!("Failed results: {:?}", failed_results[0]);
        failed_results
            .into_iter()
            .map(Result::unwrap_err)
            .for_each(|e| log::error!("Failed result: {}", e));
    }

    // Unwrap sucessful
    let nft_holders: Vec<Holder> = successful_results.into_iter().map(Result::unwrap).collect();
    info!("Found {} holders", nft_holders.len());

    Ok(nft_holders)
}

pub async fn get_holder_from_gpa_result(api_key: String, result: GPAResult) -> Result<Holder> {
    let bs64_data = &result.account.data.as_array().unwrap()[0];
    let data = base64::decode(bs64_data.as_str().unwrap())?;
    let metadata: Metadata = match try_from_slice_unchecked(&data) {
        Ok(metadata) => metadata,
        Err(_) => {
            return Err(anyhow!(
                "Failed to parse metadata for account {}",
                result.pubkey
            ));
        }
    };

    let token_results =
        match theindexio::get_holder_token_accounts(&api_key, &metadata.mint.to_string()).await {
            Ok(token_accounts) => token_accounts,
            Err(e) => {
                return Err(anyhow!(
                    "Mint Account {} has no token accounts: {:?}",
                    metadata.mint,
                    e
                ));
            }
        };

    for token_result in token_results {
        let bs64_data = &token_result.account.data.as_array().unwrap()[0];
        let data = base64::decode(bs64_data.as_str().unwrap())?;

        let parsed_account = match parse_account_data(
            &metadata.mint,
            &TOKEN_PROGRAM_ID,
            &data,
            Some(AccountAdditionalData {
                spl_token_decimals: Some(0),
            }),
        ) {
            Ok(data) => data,
            Err(err) => {
                error!("Account {} has no data: {}", token_result.pubkey, err);
                continue;
            }
        };

        let amount = match parse_token_amount(&parsed_account) {
            Ok(amount) => amount,
            Err(err) => {
                error!("Account {} has no amount: {}", token_result.pubkey, err);
                continue;
            }
        };

        // Only include current holder of the NFT.
        if amount == 1 {
            let owner_wallet = match parse_owner(&parsed_account) {
                Ok(owner_wallet) => owner_wallet,
                Err(err) => {
                    error!("Account {} has no owner: {}", token_result.pubkey, err);
                    continue;
                }
            };
            let associated_token_address = token_result.pubkey;
            let holder = Holder {
                owner_wallet,
                associated_token_address,
                mint_account: metadata.mint.to_string(),
                metadata_account: result.pubkey,
            };

            return Ok(holder);
        }
    }
    Err(anyhow!("No holder found for mint {}", metadata.mint))
}

pub async fn get_mint_account_infos(
    client: &RpcClient,
    mint_accounts: Vec<String>,
) -> Result<Vec<(Pubkey, Account)>> {
    let address_account_pairs: Arc<Mutex<Vec<(Pubkey, Account)>>> =
        Arc::new(Mutex::new(Vec::new()));

    for mint_account in mint_accounts.iter() {
        let mint_pubkey = match Pubkey::from_str(mint_account) {
            Ok(pubkey) => pubkey,
            Err(_) => {
                error!("Invalid mint address {}", mint_account);
                continue;
            }
        };

        let metadata_pubkey = get_metadata_pda(mint_pubkey);

        let account_info = match client.get_account(&metadata_pubkey).await {
            Ok(account) => account,
            Err(_) => {
                error!("Error in fetching metadata for mint {}", mint_account);
                continue;
            }
        };

        address_account_pairs
            .lock()
            .unwrap()
            .push((mint_pubkey, account_info))
    }

    let res = address_account_pairs.lock().unwrap().clone();
    Ok(res)
}

async fn get_mints_by_update_authority(
    client: &RpcClient,
    update_authority: &str,
) -> Result<Vec<(Pubkey, Account)>> {
    #[allow(deprecated)]
    let filter = RpcFilterType::Memcmp(Memcmp {
        offset: 1, // key
        bytes: MemcmpEncodedBytes::Base58(update_authority.to_string()),
        encoding: None,
    });
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: Some(CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            }),
            min_context_slot: None,
        },
        with_context: None,
    };

    let accounts = client
        .get_program_accounts_with_config(&TOKEN_METADATA_PROGRAM_ID, config)
        .await?;

    Ok(accounts)
}

pub async fn snapshot_cm_accounts(
    client: &RpcClient,
    update_authority: &str,
) -> Result<CandyMachineProgramAccounts> {
    let accounts = get_cm_accounts_by_update_authority(client, update_authority).await?;

    let mut config_accounts = Vec::new();
    let mut candy_machine_accounts = Vec::new();

    for (pubkey, account) in accounts {
        let length = account.data.len();

        // Candy machine accounts have a fixed length, config accounts do not.
        if length == 529 {
            candy_machine_accounts.push(CandyMachineAccount {
                address: pubkey.to_string(),
                data_len: length,
            });
        } else {
            config_accounts.push(ConfigAccount {
                address: pubkey.to_string(),
                data_len: length,
            });
        }
    }
    let candy_machine_program_accounts = CandyMachineProgramAccounts {
        config_accounts,
        candy_machine_accounts,
    };

    Ok(candy_machine_program_accounts)
}

async fn get_cm_accounts_by_update_authority(
    client: &RpcClient,
    update_authority: &str,
) -> Result<Vec<(Pubkey, Account)>> {
    let candy_machine_program_id = Pubkey::from_str(CANDY_MACHINE_PROGRAM_ID)?;
    #[allow(deprecated)]
    let filter = RpcFilterType::Memcmp(Memcmp {
        offset: 8, // key
        bytes: MemcmpEncodedBytes::Base58(update_authority.to_string()),
        encoding: None,
    });
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: Some(CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            }),
            min_context_slot: None,
        },
        with_context: None,
    };

    let accounts = client
        .get_program_accounts_with_config(&candy_machine_program_id, config)
        .await?;

    Ok(accounts)
}

pub async fn get_cm_creator_accounts(
    client: &RpcClient,
    creator: &str,
    position: usize,
) -> Result<Vec<(Pubkey, Account)>> {
    if position > 4 {
        error!("CM Creator position cannot be greator than 4");
        std::process::exit(1);
    }
    #[allow(deprecated)]
    let filter = RpcFilterType::Memcmp(Memcmp {
        offset: 1 + // key
            32 + // update auth
            32 + // mint
            4 + // name string length
            MAX_NAME_LENGTH + // name
            4 + // uri string length
            MAX_URI_LENGTH + // uri*
            4 + // symbol string length
            MAX_SYMBOL_LENGTH + // symbol
            2 + // seller fee basis points
            1 + // whether or not there is a creators vec
            4 + // creators
            position * // index for each creator
            (
                32 + // address
                1 + // verified
                1 // share
            ),
        bytes: MemcmpEncodedBytes::Base58(creator.to_string()),
        encoding: None,
    });

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: Some(CommitmentConfig {
                commitment: CommitmentLevel::Confirmed,
            }),
            min_context_slot: None,
        },
        with_context: None,
    };

    let accounts = client
        .get_program_accounts_with_config(&TOKEN_METADATA_PROGRAM_ID, config)
        .await?;

    Ok(accounts)
}

async fn get_holder_token_accounts(
    client: &RpcClient,
    mint_account: String,
) -> Result<Vec<(Pubkey, Account)>> {
    #[allow(deprecated)]
    let filter1 = RpcFilterType::Memcmp(Memcmp {
        offset: 0,
        bytes: MemcmpEncodedBytes::Base58(mint_account),
        encoding: None,
    });
    let filter2 = RpcFilterType::DataSize(165);
    let account_config = RpcAccountInfoConfig {
        encoding: Some(UiAccountEncoding::Base64),
        data_slice: None,
        commitment: Some(CommitmentConfig {
            commitment: CommitmentLevel::Confirmed,
        }),
        min_context_slot: None,
    };

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![filter1, filter2]),
        account_config,
        with_context: None,
    };

    let holders = client
        .get_program_accounts_with_config(&TOKEN_PROGRAM_ID, config)
        .await?;

    Ok(holders)
}

fn parse_token_amount(data: &ParsedAccount) -> Result<u64> {
    let amount = data
        .parsed
        .get("info")
        .ok_or_else(|| anyhow!("Invalid data account!"))?
        .get("tokenAmount")
        .ok_or_else(|| anyhow!("Invalid token amount!"))?
        .get("amount")
        .ok_or_else(|| anyhow!("Invalid token amount!"))?
        .as_str()
        .ok_or_else(|| anyhow!("Invalid token amount!"))?
        .parse()?;
    Ok(amount)
}

fn parse_owner(data: &ParsedAccount) -> Result<String> {
    let owner = data
        .parsed
        .get("info")
        .ok_or_else(|| anyhow!("Invalid owner account!"))?
        .get("owner")
        .ok_or_else(|| anyhow!("Invalid owner account!"))?
        .as_str()
        .ok_or_else(|| anyhow!("Invalid owner amount!"))?
        .to_string();
    Ok(owner)
}
