use crate::chart::{grouped_secret_key_refs, grouped_secret_refs, referenced_k8s_secret_names};
use crate::haystack::Corpus;
use crate::SecretMapping;
use std::collections::HashMap;
use vault::VaultClient;

fn verify_paths_exist<T: AsRef<str>>(
    k8s_secret_names: &[String],
    engine: T,
    path: T,
    client: &mut VaultClient,
) -> Result<Vec<String>, Vec<String>> {
    let mut messages = Vec::new();
    let mut verified_paths = Vec::new();
    match client.list_kv_keys(&engine, &path) {
        Ok(keys) => {
            for secret in k8s_secret_names {
                if keys.contains(secret) {
                    verified_paths.push(format!(
                        "Secret '{}' maps to {}:{}/{}",
                        secret,
                        engine.as_ref(),
                        path.as_ref(),
                        secret
                    ));
                }
            }
        }
        Err(e) => {
            messages.push(format!("Client Error: {}", e));
        }
    }

    if !messages.is_empty() {
        Err(messages)
    } else {
        Ok(verified_paths)
    }
}

pub fn verify_mapping<T: AsRef<str>>(
    corpus: &Corpus,
    engine: T,
    path: T,
    client: &mut VaultClient,
) -> Result<Vec<String>, Vec<String>> {
    let secrets = referenced_k8s_secret_names(&corpus);
    verify_paths_exist(
        &secrets.into_iter().collect::<Vec<String>>(),
        engine,
        path,
        client,
    )
}

pub fn verify_secrets_exist_in_vault(
    secret_mappings: &[SecretMapping],
    corpus: &Corpus,
    client: &mut VaultClient,
) -> Result<Vec<String>, Vec<String>> {
    let env_secrets: HashMap<String, Vec<String>> = grouped_secret_key_refs(&corpus);
    let secret_refs = grouped_secret_refs(&corpus);
    let mut messages = Vec::new();
    let mut verified_paths = Vec::new();
    for secret_name in secret_refs {
        if let Some(m) = secret_mappings
            .iter()
            .find(|m| m.kubernetes_name == secret_name)
        {
            match client.get_kv_secret(&m.vault_path.engine, &m.vault_path.path) {
                Ok(mapping) => {
                    if mapping.is_empty() {
                        messages.push(format!(
                            "No secrets for '{}' found at {}:{}",
                            secret_name, m.vault_path.engine, m.vault_path.path
                        ));
                    } else {
                        verified_paths.push(format!(
                            "{} maps to {}:{}",
                            secret_name, m.vault_path.engine, m.vault_path.path
                        ));
                    }
                }
                Err(e) => messages.push(format!("Vault client error: {}", e)),
            }
        }
    }
    for (secret_name, keys) in env_secrets {
        if let Some(m) = secret_mappings
            .iter()
            .find(|m| m.kubernetes_name == secret_name)
        {
            match client.get_kv_secret(&m.vault_path.engine, &m.vault_path.path) {
                Ok(mapping) => {
                    for key in keys {
                        if mapping.contains_key(&key) {
                            verified_paths.push(format!(
                                "{}:{} maps to {}:{}/{}",
                                secret_name, key, m.vault_path.engine, m.vault_path.path, key
                            ));
                        } else {
                            messages.push(format!(
                                "Key '{}' for secret '{}' not found in {}:{}",
                                key, secret_name, m.vault_path.engine, m.vault_path.path
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
