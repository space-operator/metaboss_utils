pub const MAX_NAME_LENGTH: usize = 32;
pub const MAX_URI_LENGTH: usize = 200;
pub const MAX_SYMBOL_LENGTH: usize = 10;
pub const MAX_CREATOR_LEN: usize = 32 + 1 + 1;

pub const METADATA_PREFIX: &str = "metadata";
pub const MASTER_EDITION_PREFIX: &str = "edition";
pub const USER_PREFIX: &str = "user";
pub const ERROR_FILES_DIR: &str = ".error_files";

pub const METAPLEX_PROGRAM_ID: &str = "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s";
pub const CANDY_MACHINE_PROGRAM_ID: &str = "cndyAnrLdpjq1Ssp1z8xxDsB8dxe7u4HL5Nxi2K5WXZ";

pub const PUBLIC_RPC_URLS: &[&str] = &[
    "https://api.devnet.solana.com",
    "https://api.testnet.solana.com",
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com",
];

pub const DEFAULT_RPC_DELAY_MS: u32 = 200;

// This is a str so it can be used in Structopt arguments
pub const DEFAULT_BATCH_SIZE: &str = "10";

pub const ERROR_FILE_BEGIN: &str = r#"#![allow(unused)]
use phf::phf_map;

"#;

pub const MINT_LAYOUT: u64 = 82;
