use crate::api::*;
use crate::error::VaultClientError;
use chrono::{DateTime, Duration, Utc};
use failure::err_msg;
use serde_json::{self, Value};
use std::convert::From;

pub struct Credentials {
    expires: Option<DateTime<Utc>>,
    client_token: String,
}

impl Credentials {
    pub fn from_auth_info(auth_info: AuthInfo) -> Credentials {
        auth_info.into()
    }
}

impl From<AuthInfo> for Credentials {
    fn from(auth_info: AuthInfo) -> Credentials {
        let expires = match auth_info.lease_duration {
            Some(d) if d > 0 => Some(Utc::now() + Duration::seconds(d)),
            _ => None,
        };

        Credentials {
            expires,
            client_token: auth_info.client_token,
        }
    }
}

pub enum BackendType {
    ClientToken(String),
    GitHub(String),
    AppRole { role_id: String, secret_id: String },
}

impl BackendType {
    pub fn login_url(&self) -> &str {
        use BackendType::*;
        match self {
            ClientToken(_) => "",
            GitHub(_) => "/v1/auth/github/login",
            AppRole { .. } => "/v1/auth/approle/login",
        }
    }

    pub fn login_payload(&self) -> Result<Value, VaultClientError> {
        use BackendType::*;
        match self {
            ClientToken(_) => Err(VaultClientError::InvalidPayload(err_msg(
                "Can't log in with a client token",
            ))),
            GitHub(t) => Ok(serde_json::to_value(GitHubToken::new(t.to_string()))?),
            AppRole { role_id, secret_id } => Ok(serde_json::to_value(AppRoleToken::new(
                role_id.as_str(),
                secret_id.as_str(),
            ))?),
        }
    }

    pub fn can_expire(&self) -> bool {
        use BackendType::*;
        match self {
            ClientToken(_) => false,
            GitHub(_) => true,
            AppRole { .. } => true,
        }
    }
}

pub struct Backend {
    ty: BackendType,
    creds: Option<Credentials>,
}

impl Backend {
    pub fn new_from_client_token<S: Into<String>>(token: S) -> Backend {
        Backend {
            ty: BackendType::ClientToken(token.into()),
            creds: None,
        }
    }

    pub fn new_from_github_token<S: Into<String>>(token: S) -> Backend {
        Backend {
            ty: BackendType::GitHub(token.into()),
            creds: None,
        }
    }

    pub fn new_from_app_role<S: Into<String>>(role_id: S, secret_id: S) -> Backend {
        Backend {
            ty: BackendType::AppRole {
                role_id: role_id.into(),
                secret_id: secret_id.into(),
            },
            creds: None,
        }
    }

    pub fn login_url(&self) -> &str {
        self.ty.login_url()
    }

    pub fn login_payload(&self) -> Result<Value, VaultClientError> {
        self.ty.login_payload()
    }

    pub fn can_expire(&self) -> bool {
        self.ty.can_expire()
    }

    pub fn set_credentials(&mut self, creds: Credentials) {
        self.creds = Some(creds);
    }

    pub fn expires(&self) -> Option<DateTime<Utc>> {
        self.creds.as_ref().and_then(|c| c.expires)
    }

    pub fn client_token(&self) -> Option<&str> {
        self.creds.as_ref().map(|c| c.client_token.as_str())
    }

    pub fn is_expired(&self) -> bool {
        if !self.has_credentials() {
            return true;
        }
        self.can_expire() && self.expires().map(|e| e < Utc::now()).unwrap_or(true)
    }

    pub fn has_credentials(&self) -> bool {
        self.client_token().is_some()
    }
}

#[derive(Debug, Serialize)]
pub struct GitHubToken {
    token: String,
}

impl GitHubToken {
    pub fn new<S: Into<String>>(token: S) -> GitHubToken {
        GitHubToken {
            token: token.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AppRoleToken {
    role_id: String,
    secret_id: String,
}

impl AppRoleToken {
    pub fn new<S: Into<String>>(role_id: S, secret_id: S) -> AppRoleToken {
        AppRoleToken {
            role_id: role_id.into(),
            secret_id: secret_id.into(),
        }
    }
}
