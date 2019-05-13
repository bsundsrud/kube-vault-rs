//! # VaultClient
//!
//! An HTTP client to the Vault API that also contains an authentication backend instance that manages
//! logging in to obtain client tokens and also refreshing client tokens, if possible.
use crate::api::*;
use failure::{bail, Error};
use reqwest::Client as HttpClient;
use reqwest::Method;
use reqwest::{Request, Url};
use serde::de::DeserializeOwned;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::auth::Backend;
use crate::error::VaultClientError;
use lazy_static::lazy_static;

lazy_static! {
    static ref LIST: Method = Method::from_bytes(b"LIST").unwrap();
}

pub struct VaultClient {
    client: HttpClient,
    vault_addr: Url,
    auth_backend: Backend,
}

impl VaultClient {
    /// Creates a `VaultClient` with a renewable login method that uses a github token for authentication.
    pub fn github<S: Into<String>>(vault_addr: Url, github_token: S) -> VaultClient {
        VaultClient::new(vault_addr, Backend::new_from_github_token(github_token))
    }

    /// Creates a `VaultClient` with a non-renewable client token.
    pub fn from_client_token<S: Into<String>>(vault_addr: Url, client_token: S) -> VaultClient {
        VaultClient::new(vault_addr, Backend::new_from_client_token(client_token))
    }

    /// Creates a `VaultClient` with a renewable login method that uses an app
    /// role (role_id + secret_id) for authentication.
    pub fn app_role<S: Into<String>>(vault_addr: Url, role_id: S, secret_id: S) -> VaultClient {
        VaultClient::new(vault_addr, Backend::new_from_app_role(role_id, secret_id))
    }

    /// Creates a `VaultClient` based on environment vars.
    ///
    /// `VAULT_ADDR` - **Required**. Specifies the base URL of the vault instance.
    ///
    /// Authentication methods:
    /// * Client Token - Specify the token with the `VAULT_TOKEN` env var.
    /// * Github Token - Specify the github token with the `VAULT_GITHUB_TOKEN` env var.
    /// * App Role - Specify the Role ID and Secret ID with the vars `VAULT_ROLE_TOKEN`
    ///   and `VAULT_SECRET_TOKEN`, respectively.
    ///
    /// Returns an `Err` result if the `VAULT_ADDR` is unspecified or an invalid URL, or
    /// if none of the authentication method vars are specified.
    pub fn from_env() -> Result<VaultClient, Error> {
        use std::env;
        let vault_addr = env::var("VAULT_ADDR")?.parse()?;
        if let Some(t) = env::var_os("VAULT_TOKEN") {
            let token = t.to_string_lossy().to_owned();
            Ok(VaultClient::from_client_token(vault_addr, token))
        } else if let Some(t) = env::var_os("VAULT_GITHUB_TOKEN") {
            let token = t.to_string_lossy().to_owned();
            Ok(VaultClient::github(vault_addr, token))
        } else if let (Some(r), Some(s)) = (
            env::var_os("VAULT_ROLE_TOKEN"),
            env::var_os("VAULT_SECRET_TOKEN"),
        ) {
            let role_id = r.to_string_lossy().to_owned();
            let secret_id = s.to_string_lossy().to_owned();
            Ok(VaultClient::app_role(vault_addr, role_id, secret_id))
        } else {
            bail!("Could not find a token of a known type in environment")
        }
    }

    pub fn new(vault_addr: Url, auth_backend: Backend) -> VaultClient {
        let client = HttpClient::new();

        VaultClient {
            client,
            vault_addr: vault_addr.into(),
            auth_backend,
        }
    }

    /// Base Vault URL
    pub fn vault_addr(&self) -> &Url {
        &self.vault_addr
    }

    fn refresh_credentials(&mut self) -> Result<(), VaultClientError> {
        if !self.auth_backend.is_expired() {
            return Ok(());
        }
        let url = self.vault_addr().join(self.auth_backend.login_url())?;
        let req = self
            .client
            .post(url)
            .json(&self.auth_backend.login_payload()?);
        let resp: VaultResponse<()> = req.send()?.error_for_status()?.json()?;
        self.auth_backend.set_credentials(resp.auth.unwrap().into());

        Ok(())
    }
    /// Perform the HTTP request while first ensuring that we have valid credentials,
    /// and refresh them if needed.
    fn request<P: DeserializeOwned>(&mut self, mut req: Request) -> Result<P, VaultClientError> {
        self.refresh_credentials()?;
        req.headers_mut().insert(
            "X-Vault-Token",
            self.auth_backend.client_token().unwrap().parse().unwrap(),
        );

        Ok(self.client.execute(req)?.error_for_status()?.json()?)
    }

    /// Get the KV secret from the specified `engine` and the specified `path`.
    ///
    /// Will perform a login if using an appropriate authentication
    /// method and no currently-valid client token.
    pub fn get_kv_secret<S: AsRef<str>>(
        &mut self,
        engine: S,
        path: S,
    ) -> Result<HashMap<String, String>, VaultClientError> {
        let engine_path = format!("/v1/{}/data/", engine.as_ref());
        let secret_path = strip_leading_slash(path.as_ref());
        let url = self.vault_addr().join(&engine_path)?.join(&secret_path)?;
        let req = self.client.get(url).build()?;
        let resp: VaultResponse<KvData> = self.request(req)?;
        Ok(resp.data.unwrap().data)
    }

    /// List secret key names from the specified `engine` and the specified `path`.
    ///
    /// Will perform a login if using an appropriate authentication
    /// method and no currently-valid client token.
    pub fn list_kv_keys<S: AsRef<str>>(
        &mut self,
        engine: S,
        path: S,
    ) -> Result<Vec<String>, VaultClientError> {
        let engine_path = format!("/v1/{}/metadata/", engine.as_ref());
        let secret_path = strip_leading_slash(path.as_ref());
        let url = self.vault_addr().join(&engine_path)?.join(&secret_path)?;
        let mut req = self.client.get(url).build()?;
        *req.method_mut() = LIST.clone();
        let resp: VaultResponse<KvKeys> = self.request(req)?;
        Ok(resp.data.unwrap().keys)
    }
}

fn strip_leading_slash<'a>(p: &'a str) -> Cow<'a, str> {
    if p.starts_with('/') {
        Cow::Owned(p.chars().skip_while(|c| *c == '/').collect())
    } else {
        Cow::Borrowed(p)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_strip_leading_slash() {
        use super::strip_leading_slash;
        use std::borrow::Cow;
        assert_eq!(&strip_leading_slash("/abc/123"), &"abc/123");
        match strip_leading_slash("abc/123") {
            Cow::Borrowed(s) => assert_eq!(s, "abc/123"),
            _ => panic!("Was not borrowed"),
        }
        match strip_leading_slash("/abc/123") {
            Cow::Owned(s) => assert_eq!(s, "abc/123"),
            _ => panic!("Should be owned"),
        }

        match strip_leading_slash("///abc/123") {
            Cow::Owned(s) => assert_eq!(s, "abc/123"),
            _ => panic!("Should be owned"),
        }
    }
}
