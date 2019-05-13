use crate::haystack::Corpus;
use serde_yaml::Mapping;
use serde_yaml::Value;
use std::collections::HashMap;

#[derive(Debug)]
pub struct EnvSecret {
    name: String,
    key: String,
}

#[derive(Debug)]
pub struct VolumeSecret {
    volume_name: String,
    secret_name: String,
}

#[derive(Debug)]
pub struct VolumeUsage {
    volume_name: String,
    secret_name: String,
    mounted_in: String,
    mount_paths: Vec<String>,
    usages: Vec<String>,
}

fn mapping_has_key(m: &Value, key: &str) -> bool {
    match m {
        Value::Mapping(m) => m.contains_key(&key.into()),
        _ => false,
    }
}

fn mapping_has_value(m: &Value, key: &str, val: &str) -> bool {
    match m {
        Value::Mapping(m) => m
            .iter()
            .any(|(k, v)| k.as_str().unwrap() == key && v.as_str().unwrap() == val),
        _ => false,
    }
}

fn value_contains(v: &Value, substr: &str) -> bool {
    match v {
        Value::String(s) => s.contains(substr),
        _ => false,
    }
}

fn filtermap_env_secret(m: &Mapping) -> Option<EnvSecret> {
    let k = "secretKeyRef".into();
    let secrets = m.get(&k)?.as_mapping()?;
    let name = secrets.get(&"name".into())?.as_str()?.to_string();
    let key = secrets.get(&"key".into())?.as_str()?.to_string();

    Some(EnvSecret { name, key })
}

fn find_env_secrets(corpus: &Corpus) -> Vec<EnvSecret> {
    corpus.filter_map_mappings(&filtermap_env_secret)
}

fn filter_map_vol_secrets(m: &Mapping) -> Option<Vec<VolumeSecret>> {
    Some(
        m.get(&"volumes".into())?
            .as_sequence()?
            .iter()
            .filter(|v| v.is_mapping() && mapping_has_key(v, "secret"))
            .map(|v| v.as_mapping().unwrap())
            .map(|m| {
                let name = m.get(&"name".into()).unwrap().as_str().unwrap();
                let secret = m.get(&"secret".into()).unwrap().as_mapping().unwrap();
                let secret_name = secret.get(&"secretName".into()).unwrap().as_str().unwrap();
                VolumeSecret {
                    volume_name: name.to_string(),
                    secret_name: secret_name.to_string(),
                }
            })
            .collect(),
    )
}

fn find_vol_secrets(corpus: &Corpus) -> Vec<VolumeSecret> {
    corpus
        .filter_map_mappings(filter_map_vol_secrets)
        .into_iter()
        .flat_map(|v| v)
        .collect()
}

fn filter_map_vol_usages(m: &Mapping, secret: &VolumeSecret) -> Option<Vec<VolumeUsage>> {
    let containers = m.get(&"containers".into())?.as_sequence()?;
    let mut res = Vec::new();
    for container_v in containers {
        let container = container_v.as_mapping()?;
        let name = container
            .get(&"name".into())
            .or_else(|| container.get(&"image".into()))?
            .as_str()?;
        let mount_paths: Vec<String> = container
            .get(&"volumeMounts".into())?
            .as_sequence()?
            .iter()
            .filter(|m| m.is_mapping() && mapping_has_value(m, "name", &secret.volume_name))
            .map(|m| m.as_mapping().unwrap())
            .map(|m| {
                m.get(&"mountPath".into())
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        if mount_paths.is_empty() {
            continue;
        }

        let usages = Corpus::filter_map_values_from(&container_v, |v| {
            if mount_paths
                .iter()
                .any(|p| value_contains(v, &format!("{}/", p)))
            {
                Some(v.as_str()?.to_string())
            } else {
                None
            }
        });

        res.push(VolumeUsage {
            volume_name: secret.volume_name.to_string(),
            secret_name: secret.secret_name.to_string(),
            mounted_in: name.to_string(),
            mount_paths,
            usages,
        });
    }
    Some(res)
}

fn find_vol_usages(corpus: &Corpus, volume_secret: &VolumeSecret) -> Vec<VolumeUsage> {
    corpus
        .filter_map_mappings(|m| filter_map_vol_usages(m, &volume_secret))
        .into_iter()
        .flat_map(|v| v)
        .collect()
}

fn find_all_vol_usages(corpus: &Corpus, secrets: &[VolumeSecret]) -> Vec<VolumeUsage> {
    let mut res = Vec::new();
    for s in secrets {
        res.extend(find_vol_usages(corpus, &s));
    }
    res
}

fn print_env_secrets(map: &HashMap<String, Vec<String>>) {
    println!("ENVIRONMENT SECRETS");
    for (k, v) in map {
        println!("  Secret '{}'", k);
        for s in v {
            println!("    {}", s);
        }
    }
}

fn print_vol_secrets(map: &HashMap<String, Vec<VolumeUsage>>) {
    println!("VOLUME SECRETS");
    for (k, v) in map {
        println!("   Secret '{}'", k);
        for usage in v {
            println!("    Container: {}", usage.mounted_in);
            println!("    Volume Name: {}", usage.volume_name);
            println!("    Mount Paths:");
            for p in usage.mount_paths.iter() {
                println!("      {}", p);
            }
            println!("    Usages in deployment:");
            for u in usage.usages.iter() {
                println!("      {}", u);
            }
        }
    }
}

pub fn grouped_env_secrets(corpus: &Corpus) -> HashMap<String, Vec<String>> {
    let env_secrets = find_env_secrets(&corpus);
    env_secrets
        .into_iter()
        .map(|s| (s.name, s.key))
        .fold(HashMap::new(), |mut acc, (n, k)| {
            let v = acc.entry(n).or_insert_with(Vec::new);
            v.push(k);
            acc
        })
}

pub fn grouped_vol_secrets(corpus: &Corpus) -> HashMap<String, Vec<VolumeUsage>> {
    let vol_secrets = find_vol_secrets(&corpus);
    let vol_usages = find_all_vol_usages(&corpus, &vol_secrets);
    vol_usages
        .into_iter()
        .map(|s| (s.secret_name.to_string(), s))
        .fold(HashMap::new(), |mut acc, (n, s)| {
            let v = acc.entry(n).or_insert_with(Vec::new);
            v.push(s);
            acc
        })
}

pub fn list_secrets(corpus: &Corpus) {
    let grouped_env = grouped_env_secrets(&corpus);
    let grouped_vols = grouped_vol_secrets(&corpus);
    print_env_secrets(&grouped_env);
    println!();
    print_vol_secrets(&grouped_vols);
}
