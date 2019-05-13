use failure::{Error, Fail};
use reqwest::Error as HttpError;
use reqwest::StatusCode;
use serde_json::Error as JsonError;
use std::convert::From;
use std::string::ToString;
use url::ParseError;

#[derive(Debug, Fail)]
pub enum VaultClientError {
    #[fail(display = "Not Found: {}", _0)]
    NotFound(String),
    #[fail(display = "Not Authorized: {}", _0)]
    NotAuthorized(Error),
    #[fail(display = "Invalid Url: {}", _0)]
    InvalidUrl(Error),
    #[fail(display = "Invalid Payload: {}", _0)]
    InvalidPayload(Error),
    #[fail(display = "Unknown Client error: {}", _0)]
    Unknown(Error),
}

impl From<ParseError> for VaultClientError {
    fn from(e: ParseError) -> VaultClientError {
        VaultClientError::InvalidUrl(e.into())
    }
}

impl From<JsonError> for VaultClientError {
    fn from(e: JsonError) -> VaultClientError {
        VaultClientError::InvalidPayload(e.into())
    }
}

impl From<HttpError> for VaultClientError {
    fn from(e: HttpError) -> VaultClientError {
        match e.status() {
            Some(StatusCode::UNAUTHORIZED) | Some(StatusCode::FORBIDDEN) => {
                VaultClientError::NotAuthorized(e.into())
            }
            Some(StatusCode::NOT_FOUND) => {
                let url = e.url().map(ToString::to_string).unwrap_or_else(String::new);
                VaultClientError::NotFound(url)
            }
            _ => VaultClientError::Unknown(e.into()),
        }
    }
}
