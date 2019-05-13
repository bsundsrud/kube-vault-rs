use crate::chart::grouped_env_secrets;
use crate::haystack::Corpus;
use crate::SecretMapping;
use std::collections::HashMap;
use vault::VaultClient;

pub fn verify_secrets_exist_in_vault(
    secret_mappings: &[SecretMapping],
    corpus: &Corpus,
    client: &mut VaultClient,
) -> Result<Vec<String>, Vec<String>> {
    let env_secrets: HashMap<String, Vec<String>> = grouped_env_secrets(&corpus);
    let mut messages = Vec::new();
    let mut verified_paths = Vec::new();
    for (secret_name, keys) in env_secrets {
        if let Some(m) = secret_mappings
            .iter()
            .find(|m| m.kubernetes_name == secret_name)
        {
            match client.get_kv_secret(&m.vault_engine, &m.vault_path) {
                Ok(mapping) => {
                    for key in keys {
                        if mapping.contains_key(&key) {
                            verified_paths.push(format!(
                                "{}:{} maps to {}:{}/{}",
                                secret_name, key, m.vault_engine, m.vault_path, key
                            ));
                        } else {
                            messages.push(format!(
                                "Key '{}' for secret '{}' not found in {}:{}",
                                key, secret_name, m.vault_engine, m.vault_path
                            ));
                        }
                    }
                }
                Err(e) => messages.push(format!("Vault client error: {}", e)),
            }
        } else {
            messages.push(format!(
                "Couldn't find a vault mapping for kubernetes secret {}",
                secret_name
            ));
        }
    }

    if !messages.is_empty() {
        Err(messages)
    } else {
        Ok(verified_paths)
    }
}
