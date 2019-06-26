use crate::SecretMapping;
use askama::Template;
use base64;
use failure::Error;
use std::collections::HashMap;
use vault::VaultClient;

#[derive(Template)]
#[template(path = "secret.yaml", escape = "none")]
pub struct SecretsTemplate {
    secret_name: String,
    namespace: String,
    vault_addr: String,
    vault_engine: String,
    vault_path: String,
    encoded_data: HashMap<String, String>,
}

impl SecretsTemplate {
    pub fn new(
        vault_addr: &str,
        secret_name: &str,
        namespace: &str,
        vault_engine: &str,
        vault_path: &str,
        data: HashMap<String, String>,
    ) -> SecretsTemplate {
        SecretsTemplate {
            vault_addr: vault_addr.into(),
            secret_name: secret_name.into(),
            namespace: namespace.into(),
            vault_engine: vault_engine.into(),
            vault_path: vault_path.into(),
            encoded_data: data
                .into_iter()
                .map(|(k, v)| (k, base64::encode(&v)))
                .collect(),
        }
    }
}

pub fn create_secret_template(
    mappings: &[SecretMapping],
    namespace: &str,
    client: &mut VaultClient,
) -> Result<(), Error> {
    for mapping in mappings {
        let data = client.get_kv_secret(&mapping.vault_path.engine, &mapping.vault_path.path)?;
        let template = SecretsTemplate::new(
            client.vault_addr().as_str(),
            &mapping.kubernetes_name,
            &namespace,
            &mapping.vault_path.engine,
            &mapping.vault_path.path,
            data,
        );
        println!("{}", template.render().unwrap());
    }
    Ok(())
}

pub mod filters {
    use askama::Result;
    use std::fmt;

    pub fn with_leading_slash(s: &dyn fmt::Display) -> Result<String> {
        let mut s = s.to_string();
        if !s.starts_with('/') {
            s.insert(0, '/');
        }
        Ok(s)
    }
}
