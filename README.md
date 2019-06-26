# kube-vault

[![Build Status](https://travis-ci.org/bsundsrud/kube-vault-rs.svg?branch=master)](https://travis-ci.org/bsundsrud/kube-vault-rs)

List, Verify, and Generate Kubernetes Secrets based on
[Vault](https://www.vaultproject.io/) as the source of truth.

Intended to be used with either [Helm](https://helm.sh/) or loose kubernetes files.

## Installing

See the [releases](https://github.com/bsundsrud/kube-vault-rs/releases)
page to download a precompiled executable for your platform.

## Vault Env Variables

`kube-vault` will detect standard Vault environment variables and
use them for its internal client.

Important variables:

* `VAULT_ADDR` - **Required**. URL of the vault server (ex. `https://vault.mycompany.com`)
* `VAULT_TOKEN` - A Vault client token to use for the requests.
  This is obtained via the `vault login` command.
* `VAULT_GITHUB_TOKEN` - A GitHub token that `kube-vault` can use to log in
  to the vault instance and obtain its own client token.
* `VAULT_ROLE_TOKEN` - Must be specified with `VAULT_SECRET_TOKEN`.
  App Role ID when using App Role Authentication.
* `VAULT_SECRET_TOKEN` - Must be specified with `VAULT_ROLE_TOKEN`.
  App Secret ID when using App Role Authentication.

One of (`VAULT_TOKEN`, `VAULT_GITHUB_TOKEN`,
`VAULT_ROLE_TOKEN` + `VAULT_SECRET_TOKEN`) must be supplied.

`kube-vault` has support for `.env` files and will use values in a `.env` file
if they are not already present in the environment.

## Commands

```
$ kube-vault -h
kube-vault
Manage k8s secrets with vault as the source-of-truth

USAGE:
    kube-vault <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    generate    Create k8s secrets from vault
    help        Prints this message or the help of the given subcommand(s)
    list        Lists secrets accessed by a chart
    verify      Verify secrets used by a chart exist in vault
```

### `list`

```
$ kube-vault list -h
kube-vault-list
Lists secrets accessed by a chart

USAGE:
    kube-vault list
```

`list` will read kube files from stdin (say, from `helm get`, `helm template`,
or `cat *.yaml`) and list environment and volume secrets that
reference kubernetes secrets.

### `verify`

```
$ kube-vault verify -h
kube-vault-verify
Verify secrets used by a chart exist in vault

USAGE:
    kube-vault verify -m <mapping>... -p <vault-path>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m <mapping>...        Maps k8s secret name to vault path (ex. my-secrets=engine-name:/apps/my-app/secret)
    -p <vault-path>        Vault path to source secrets from (ex. engine-name:/apps/my-app)
```

`verify` will read kube files from stdin (say, from `helm get`, `helm template`,
or `cat *.yaml`) and ensure that secrets exist at the corresponding vault path.

`-m` path mappings can be specified multiple times if there are multiple
kubernetes secrets referenced.

The `-m` option will map from vault secret to kubernetes secret directly,
`-p` specifies a k8s namespace to vault path mapping, where every secret
referenced in the kubefiles is assumed to correspond to a secret in
the given vault path. `-m` and `-p` are mutually exclusive.

### `generate`

```

$ kube-vault generate -h
kube-vault-generate
Create k8s secrets from vault

USAGE:
    kube-vault generate -m <mapping>... -N <namespace> -p <vault-path>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -m <mapping>...        Maps k8s secret name to vault path (ex. my-secrets=engine-name:/apps/my-app/secret)
    -N <namespace>         k8s namespace for generated secrets
    -p <vault-path>        Vault path to source secrets from (ex. engine-name:/apps/my-app)

```

`generate` will read kube files from stdin (say, from `helm get`, `helm template`,
or `cat *.yaml`) and output a yaml kubernetes secret definition on stdout.
This can be applied to your cluster with `kubectl apply`. `-m` path mappings
can be specified multiple times if there are multiple kubernetes secrets referenced.

`-N` is required, even if you just use `default`
(please properly namespace your secrets).

The `-m` option will map from vault secret to kubernetes secret directly,
`-p` specifies a k8s namespace to vault path mapping, where every secret
referenced in the kubefiles is assumed to correspond to a secret in
the given vault path. `-m` and `-p` are mutually exclusive.

## Building

A 2018-Edition Rust is required, and optionally Make.
The Makefile builds the release with musl on Linux so it is fully statically linked.

Running `make && sudo make install` will install kube-vault as `/usr/local/bin/kube-vault`.
If you want to install it elsewhere or not system-wide you can set the
`PREFIX` make variable:

`make && make PREFIX=$HOME install` will install to `$HOME/bin`.
