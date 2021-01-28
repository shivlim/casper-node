use thiserror::Error;
use uint::FromDecStrErr;

use casper_types::account::ACCOUNT_HASH_LENGTH;

use crate::utils::ReadFileError;

/// Error while encoding or decoding the chainspec.
#[derive(Debug, Error)]
pub enum Error {
    /// Error while decoding the chainspec from TOML format.
    #[error("decoding from TOML error: {0}")]
    DecodingFromToml(#[from] toml::de::Error),

    /// Error while decoding Motes from a decimal format.
    #[error("decoding motes from base-10 error: {0}")]
    DecodingMotes(#[from] FromDecStrErr),

    /// Error loading the chainspec.
    #[error("could not load chainspec: {0}")]
    LoadChainspec(ReadFileError),

    /// Error loading the upgrade point.
    #[error("could not load upgrade point: {0}")]
    LoadUpgradePoint(ReadFileError),

    /// Error loading the chainspec accounts.
    #[error("could not load chainspec accounts: {0}")]
    LoadChainspecAccounts(ChainspecAccountsLoadError),
}

/// Error loading chainspec accounts file.
#[derive(Debug, Error)]
pub enum ChainspecAccountsLoadError {
    /// Error while decoding the chainspec accounts from CSV format.
    #[error("decoding from CSV error: {0}")]
    DecodingFromCsv(#[from] csv::Error),

    /// Error while decoding a chainspec account's key hash from hex format.
    #[error("decoding from hex error: {0}")]
    DecodingFromHex(#[from] hex::FromHexError),

    /// Error while decoding Motes from a decimal format.
    #[error("decoding motes from base-10 error: {0}")]
    DecodingMotes(#[from] FromDecStrErr),

    /// Decoding a chainspec account's key hash yielded an invalid length byte array.
    #[error("expected hash length of {}, got {0}", ACCOUNT_HASH_LENGTH)]
    InvalidHashLength(usize),

    /// Error while decoding a chainspec account's key hash from base-64 format.
    #[error("crypto module error: {0}")]
    Crypto(#[from] crate::crypto::Error),
}
