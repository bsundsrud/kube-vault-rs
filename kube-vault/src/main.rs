use clap::{App, AppSettings, Arg, SubCommand};
use dotenv;
use failure::{bail, Error};
use openssl_probe;
use std::io;
use vault::VaultClient;

mod chart;
mod generate;
pub mod haystack;
mod verify;

use haystack::Corpus;

#[derive(Debug, Clone)]
pub struct VaultPath {
    pub engine: String,
    pub path: String,
}

#[derive(Debug)]
pub struct SecretMapping {
    pub kubernetes_name: String,
    pub vault_path: VaultPath,
}

impl SecretMapping {
    pub fn new<S: Into<String>>(kubernetes_name: S, vault_path: VaultPath) -> SecretMapping {
        SecretMapping {
            kubernetes_name: kubernetes_name.into(),
            vault_path,
        }
    }

    pub fn from_secret_names_and_vault_path<I>(
        secret_names: I,
        vault_path: VaultPath,
    ) -> Vec<SecretMapping>
    where
        I: IntoIterator<Item = String>,
    {
        secret_names
            .into_iter()
            .map(|n| {
                let mut path = vault_path.clone();
                if path.path.ends_with('/') {
                    path.path = format!("{}{}", path.path, n);
                } else {
                    path.path = format!("{}/{}", path.path, n);
                }
                SecretMapping::new(n, path)
            })
            .collect()
    }
}
fn read_from_stdin() -> Result<Corpus, Error> {
    let stdin = io::stdin();
    let handle = stdin.lock();
    Corpus::from_reader(handle)
}

fn validate_vault_path<T: AsRef<str>>(m: T) -> Result<(), String> {
    let m = m.as_ref();
    if !m.contains(':') {
        return Err(format!(
            "Invalid vault path: {}.  Path should have the pattern 'engine:path/to/secret'",
            m
        ));
    }
    Ok(())
}

fn validate_mapping(m: String) -> Result<(), String> {
    let split: Vec<&str> = m.splitn(2, '=').collect();
    if split.len() != 2 {
        return Err(format!("Invalid mapping (missing =): {}", m));
    }
    let (_kube_part, vault_part) = (split[0], split[1]);
    validate_vault_path(&vault_part)
}

fn parse_vault_path(s: &str) -> VaultPath {
    let mut split = s.splitn(2, ':');
    VaultPath {
        engine: split
            .next()
            .expect("Invalid vault path, empty string")
            .to_string(),
        path: split
            .next()
            .expect("Invalid vault path, missing :")
            .to_string(),
    }
}

fn parse_mappings<'a>(map_strs: impl Iterator<Item = &'a str>) -> Vec<SecretMapping> {
    map_strs
        .map(|s| {
            let split: Vec<&str> = s.splitn(2, '=').collect();
            (split[0], split[1])
        })
        .map(|(kube_part, vault_part)| {
            let vault_path = parse_vault_path(&vault_part);
            SecretMapping::new(kube_part, vault_path)
        })
        .collect()
}

fn verify_secrets_in_path(
    vault_path: &VaultPath,
    corpus: &Corpus,
    client: &mut VaultClient,
) -> Result<(), Error> {
    let messages = verify::verify_mapping(&corpus, &vault_path.engine, &vault_path.path, client);
    match messages {
        Ok(msgs) => {
            msgs.iter().for_each(|msg| eprintln!("Verified {}", msg));
        }
        Err(msgs) => {
            msgs.iter().for_each(|msg| eprintln!("ERROR: {}", msg));
            bail!("Missing secrets in vault, exiting...");
        }
    }
    Ok(())
}

fn verify_secrets(
    mappings: &[SecretMapping],
    corpus: &Corpus,
    client: &mut VaultClient,
) -> Result<(), Error> {
    let messages = verify::verify_secrets_exist_in_vault(&mappings, &corpus, client);
    match messages {
        Ok(msgs) => {
            msgs.iter().for_each(|msg| eprintln!("Verified {}", msg));
        }
        Err(msgs) => {
            msgs.iter().for_each(|msg| eprintln!("ERROR: {}", msg));
            bail!("Missing secrets in vault, exiting...");
        }
    }
    Ok(())
}

fn cli_main() -> Result<(), Error> {
    let app = App::new("kube-vault")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .author("Benn Sundsrud <benn.sundsrud@gmail.com>")
        .about("Manage k8s secrets with vault as the source-of-truth")
        .subcommand(SubCommand::with_name("list").about("Lists secrets accessed by a chart"))
        .subcommand(
            SubCommand::with_name("verify")
                .about("Verify secrets used by a chart exist in vault")
                .arg(
                    Arg::with_name("mapping")
                        .short("m")
                        .takes_value(true)
                        .multiple(true)
                        .number_of_values(1)
                        .validator(validate_mapping)
                        .required_unless("vault-path")
                        .conflicts_with("vault-path")
                        .help("Maps k8s secret name to vault path (ex. my-secrets=engine-name:/apps/my-app/secret)"),
                )
                .arg(
                    Arg::with_name("vault-path")
                        .short("p")
                        .takes_value(true)
                        .validator(validate_vault_path)
                        .required_unless("mapping")
                        .conflicts_with("mapping")
                        .help("Vault path to source secrets from (ex. engine-name:/apps/my-app)")
                ),
            )
        .subcommand(
            SubCommand::with_name("generate")
                .about("Create k8s secrets from vault")
                .arg(
                    Arg::with_name("mapping")
                        .short("m")
                        .takes_value(true)
                        .multiple(true)
                        .number_of_values(1)
                        .validator(validate_mapping)
                        .required_unless("vault-path")
                        .conflicts_with("vault-path")
                        .help("Maps k8s secret name to vault path (ex. my-secrets=engine-name:/apps/my-app/secret)"),
                )
                .arg(
                    Arg::with_name("vault-path")
                        .short("p")
                        .takes_value(true)
                        .validator(validate_vault_path)
                        .required_unless("mapping")
                        .conflicts_with("mapping")
                        .help("Vault path to source secrets from (ex. engine-name:/apps/my-app)")
                )
                .arg(
                    Arg::with_name("namespace")
                        .short("N")
                        .required(true)
                        .takes_value(true)
                        .help("k8s namespace for generated secrets"),
                ),
        );
    let matches = app.get_matches();

    if let Some(_subcommand) = matches.subcommand_matches("list") {
        let corpus = read_from_stdin()?;
        chart::list_secrets(&corpus);
    } else if let Some(subcommand) = matches.subcommand_matches("verify") {
        let corpus = read_from_stdin()?;
        let client = VaultClient::from_env();
        let mut client = match client {
            Ok(c) => c,
            Err(e) => bail!("Could not create vault client: {}", e),
        };
        if subcommand.is_present("mapping") {
            let mappings = subcommand
                .values_of("mapping")
                .map(parse_mappings)
                .unwrap_or_else(Vec::new);
            verify_secrets(&mappings, &corpus, &mut client)?;
        } else if subcommand.is_present("vault-path") {
            let vault_path = subcommand
                .value_of("vault-path")
                .map(parse_vault_path)
                .unwrap();
            verify_secrets_in_path(&vault_path, &corpus, &mut client)?;
        }
    } else if let Some(subcommand) = matches.subcommand_matches("generate") {
        let corpus = read_from_stdin()?;
        let namespace = subcommand.value_of("namespace").unwrap(); // Is a required field
        let client = VaultClient::from_env();
        let mut client = match client {
            Ok(c) => c,
            Err(e) => bail!("Could not create vault client: {}", e),
        };
        if subcommand.is_present("mapping") {
            let mappings = subcommand
                .values_of("mapping")
                .map(parse_mappings)
                .unwrap_or_else(Vec::new);
            verify_secrets(&mappings, &corpus, &mut client)?;
            generate::create_secret_template(&mappings, &namespace, &mut client)?;
        } else if subcommand.is_present("vault-path") {
            let vault_path = subcommand
                .value_of("vault-path")
                .map(parse_vault_path)
                .unwrap();
            verify_secrets_in_path(&vault_path, &corpus, &mut client)?;
            let secrets = chart::referenced_k8s_secret_names(&corpus);
            let mappings = SecretMapping::from_secret_names_and_vault_path(secrets, vault_path);
            generate::create_secret_template(&mappings, &namespace, &mut client)?;
        }
    }

    Ok(())
}

fn main() {
    openssl_probe::init_ssl_cert_env_vars();
    dotenv::dotenv().ok();
    if let Err(e) = cli_main() {
        eprintln!("ERROR: {}", e);
        ::std::process::exit(1);
    }
}
