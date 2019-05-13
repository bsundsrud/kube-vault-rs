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

#[derive(Debug)]
pub struct SecretMapping {
    pub kubernetes_name: String,
    pub vault_engine: String,
    pub vault_path: String,
}

impl SecretMapping {
    pub fn new<S: Into<String>>(
        kubernetes_name: S,
        vault_engine: S,
        vault_path: S,
    ) -> SecretMapping {
        SecretMapping {
            kubernetes_name: kubernetes_name.into(),
            vault_engine: vault_engine.into(),
            vault_path: vault_path.into(),
        }
    }
}
fn read_from_stdin() -> Result<Corpus, Error> {
    let stdin = io::stdin();
    let handle = stdin.lock();
    Corpus::from_reader(handle)
}

fn validate_mapping(m: String) -> Result<(), String> {
    let split: Vec<&str> = m.splitn(2, '=').collect();
    if split.len() != 2 {
        return Err(format!("Invalid mapping (missing =): {}", m));
    }
    let (kube_part, vault_part) = (split[0], split[1]);
    if !vault_part.contains(':') {
        return Err(format!(
            "Invalid mapping (missing vault engine): {}.  Kube secret name was {}",
            m, kube_part
        ));
    }
    Ok(())
}

fn parse_mappings<'a>(map_strs: impl Iterator<Item = &'a str>) -> Vec<SecretMapping> {
    map_strs
        .map(|s| {
            let split: Vec<&str> = s.splitn(2, '=').collect();
            (split[0], split[1])
        })
        .map(|(kube_part, vault_part)| {
            let split: Vec<&str> = vault_part.splitn(2, ':').collect();
            SecretMapping::new(kube_part, split[0], split[1])
        })
        .collect()
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
                        .help("Maps k8s secret name to vault path (ex. my-secrets=engine-name:/apps/my-app/)"),
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
                        .help("Maps k8s secret name to vault path (ex. my-secrets=engine-name:/apps/my-app/)"),
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
        let mappings = subcommand
            .values_of("mapping")
            .map(parse_mappings)
            .unwrap_or_else(Vec::new);
        let client = VaultClient::from_env();
        let mut client = match client {
            Ok(c) => c,
            Err(e) => bail!("Could not create vault client: {}", e),
        };

        verify_secrets(&mappings, &corpus, &mut client)?;
    } else if let Some(subcommand) = matches.subcommand_matches("generate") {
        let corpus = read_from_stdin()?;
        let mappings = subcommand
            .values_of("mapping")
            .map(parse_mappings)
            .unwrap_or_else(Vec::new);
        let namespace = subcommand.value_of("namespace").unwrap(); // Is a required field
        let client = VaultClient::from_env();
        let mut client = match client {
            Ok(c) => c,
            Err(e) => bail!("Could not create vault client: {}", e),
        };
        verify_secrets(&mappings, &corpus, &mut client)?;
        generate::create_secret_template(&mappings, &namespace, &mut client)?;
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
