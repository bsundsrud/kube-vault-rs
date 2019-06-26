use crate::haystack::Corpus;
use serde_yaml::Mapping;
use serde_yaml::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct SecretRef(String);

#[derive(Debug)]
pub struct SecretKeyRef {
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

fn filter_map_secret_ref(m: &Mapping) -> Option<SecretRef> {
    let k = "secretRef".into();
    let secrets = m.get(&k)?.as_mapping()?;
    let name = secrets.get(&"name".into())?.as_str()?.to_string();
    Some(SecretRef(name))
}

fn filter_map_secret_key_ref(m: &Mapping) -> Option<SecretKeyRef> {
    let k = "secretKeyRef".into();
    let secrets = m.get(&k)?.as_mapping()?;
    let name = secrets.get(&"name".into())?.as_str()?.to_string();
    let key = secrets.get(&"key".into())?.as_str()?.to_string();
    Some(SecretKeyRef { name, key })
}

fn find_secret_refs(corpus: &Corpus) -> Vec<SecretRef> {
    corpus.filter_map_mappings(&filter_map_secret_ref)
}

fn find_secret_key_refs(corpus: &Corpus) -> Vec<SecretKeyRef> {
    corpus.filter_map_mappings(&filter_map_secret_key_ref)
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

fn print_secret_refs<T: AsRef<str>>(secret_refs: &[T], map: &HashMap<String, Vec<String>>) {
    println!("REFERENCES TO WHOLE SECRETS");
    if secret_refs.is_empty() {
        println!("(None)");
    }
    for s in secret_refs {
        println!("  Secret '{}'", s.as_ref());
    }
    println!();
    println!("REFERENCES TO SECRET KEYS");
    if map.is_empty() {
        println!("(None)");
    }
    for (k, v) in map {
        println!("  Secret '{}':", k);
        for s in v {
            println!("    {}", s);
        }
    }
}

fn print_vol_secrets(map: &HashMap<String, Vec<VolumeUsage>>) {
    println!("VOLUME SECRETS");
    if map.is_empty() {
        println!("(None)");
    }
    for (k, v) in map {
        println!("   Secret '{}':", k);
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

pub fn grouped_secret_key_refs(corpus: &Corpus) -> HashMap<String, Vec<String>> {
    let env_secrets = find_secret_key_refs(&corpus);
    env_secrets
        .into_iter()
        .map(|s| (s.name, s.key))
        .fold(HashMap::new(), |mut acc, (n, k)| {
            let v = acc.entry(n).or_insert_with(Vec::new);
            v.push(k);
            acc
        })
}

pub fn grouped_secret_refs(corpus: &Corpus) -> Vec<String> {
    find_secret_refs(&corpus).into_iter().map(|r| r.0).collect()
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

pub fn referenced_k8s_secret_names(corpus: &Corpus) -> HashSet<String> {
    let mut res = HashSet::new();
    let secret_refs = find_secret_refs(&corpus);
    let secret_key_refs = find_secret_key_refs(&corpus);
    let vol_refs = find_vol_secrets(&corpus);
    res.extend(secret_refs.into_iter().map(|r| r.0));
    res.extend(secret_key_refs.into_iter().map(|r| r.name));
    res.extend(vol_refs.into_iter().map(|r| r.secret_name));
    res
}

pub fn list_secrets(corpus: &Corpus) {
    let grouped_secret_refs = grouped_secret_refs(&corpus);
    let grouped_secret_key_refs = grouped_secret_key_refs(&corpus);
    let grouped_vols = grouped_vol_secrets(&corpus);
    print_secret_refs(&grouped_secret_refs, &grouped_secret_key_refs);
    println!();
    print_vol_secrets(&grouped_vols);
}
