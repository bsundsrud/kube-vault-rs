use crate::SecretMapping;
use crate::VaultPath;
use failure::Error;
use vault::VaultClient;

fn join_path(path: &VaultPath, key: &str) -> VaultPath {
    let mut path = path.clone();
    if path.path.ends_with('/') {
        path.path = format!("{}{}", path.path, key);
    } else {
        path.path = format!("{}/{}", path.path, key);
    }
    path
}

pub fn secrets_in_path(
    client: &mut VaultClient,
    path: &VaultPath,
) -> Result<Vec<SecretMapping>, Error> {
    let keys = client.list_kv_keys(&path.engine, &path.path)?;
    Ok(keys
        .iter()
        .map(|k| SecretMapping::new(k, join_path(&path, k)))
        .collect())
}
