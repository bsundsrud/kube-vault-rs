//! # vault
//!
//! The `vault` crate provides a high level wrapper around the Vault HTTP API, via [`VaultClient`][client].
//!
//! ## Available auth methods
//! Auth methods are implemented via the [`Backend`][auth-backend] struct.
//! * [Client Token][client-token] - Provide a token that you've already obtained by logging in through other means.
//! * [Github Token][github-token] - Provide a github token that will be used to log in and obtain the client token.
//! * [App Role][app-role] - Provide a Role and Secret ID to use to log in to obtain the client token.
//!
//! The above methods can also source from the environment, see the [`from_env`][from-env] method.
//!
//! ## Available Secrets Engines
//!
//! Currently only K/V version 2 is supported.  This can be easily extended via adding methods to the [`VaultClient`][client].  Currently supports getting secrets for a path via [`get_kv_secret`][client-get-kv-secret] and listing secrets on a path via [`list_kv_keys`][client-list-kv-keys].
//!
//! [client]: ./client/struct.VaultClient.html
//! [auth-backend]: ./auth/struct.Backend.html
//! [client-token]: ./client/struct.VaultClient.html#method.from_client_token
//! [github-token]: ./client/struct.VaultClient.html#method.github
//! [app-role]: ./client/struct.VaultClient.html#method.app_role
//! [from-env]: ./client/struct.VaultClient.html#method.from_env
//! [client-get-kv-secret]: ./client/struct.VaultClient.html#method.get_kv_secret
//! [client-list-kv-keys]: ./client/struct.VaultClient.html#method.list_kv_keys
#[macro_use]
extern crate serde_derive;

pub mod api;
pub mod auth;
pub mod client;
pub mod error;

pub use client::VaultClient;
pub use error::VaultClientError;
