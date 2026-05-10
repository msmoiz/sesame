mod client;
mod config;

use std::io::{self, Read};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use client::Client;
use config::Config;
use inquire::{Password, Select};

/// A secret manager.
///
/// Secrets are simple key-value pairs. The value for a secret can be an arbitrary
/// string. Secrets are hosted at secrets.msmoiz.com.
///
/// You can configure the following settings in a config file at
/// ~/.sesame/config.toml or using environment variables.
///
/// - password: The server password (SESAME_PASSWORD)
/// - url: The server url (SESAME_SERVER_URL)
#[derive(Debug, Parser)]
#[command(version, verbatim_doc_comment, max_term_width = 80)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Store a new secret.
    Publish {
        /// The name of the secret.
        name: String,
        /// The value of the secret.
        value: Option<String>,
        /// If specified, reads the secret value from standard input.
        #[arg(long, default_value_t = false)]
        stdin: bool,
    },
    /// List secret names.
    List,
    /// Fetch a secret and print it to stdout.
    Get { name: String },
    /// Set up server credentials.
    ///
    /// You can also use the SESAME_PASSWORD environment variable to override
    /// the password for individual commands.
    Login {
        /// The URL for the server (default: secrets.msmoiz.com).
        #[arg(long)]
        url: Option<String>,
    },
    /// Browse secrets using an interactive dialog.
    Browse,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error:?}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Publish { name, value, stdin } => publish(name, value, stdin),
        Command::List => list(),
        Command::Get { name } => get(name),
        Command::Login { url } => login(url),
        Command::Browse => browse(),
    }
}

/// Creates an API client.
fn configured_client() -> anyhow::Result<Client> {
    let config = Config::load()?;
    Ok(Client::new(config.url.clone(), config.password.to_owned()))
}

/// Publishes a secret to the store.
fn publish(name: String, value: Option<String>, stdin: bool) -> anyhow::Result<()> {
    let client = configured_client()?;
    let value = resolve_secret_value(value, stdin)?;
    client.publish_secret(&name, &value)?;
    Ok(())
}

/// Resolves a secret value.
///
/// If `value` is set, it uses that. Otherwise, if `stdin` is set, it reads the
/// password from standard input. If both are set, this method returns an error.
fn resolve_secret_value(value: Option<String>, stdin: bool) -> anyhow::Result<String> {
    match (value, stdin) {
        (Some(_), true) => bail!("pass either a value argument or --stdin, not both"),
        (Some(value), false) => Ok(value),
        (None, true) => read_stdin(),
        (None, false) => Password::new("Secret value")
            .without_confirmation()
            .prompt()
            .context("failed to read secret value"),
    }
}

/// Reads a value from standard input.
fn read_stdin() -> anyhow::Result<String> {
    let mut buffer = String::new();

    io::stdin()
        .read_to_string(&mut buffer)
        .context("failed to read stdin")?;

    if let Some(stripped) = buffer.strip_suffix('\n') {
        return Ok(stripped.strip_suffix('\r').unwrap_or(stripped).to_owned());
    }

    Ok(buffer)
}

/// Lists secrets in the store.
fn list() -> anyhow::Result<()> {
    let client = configured_client()?;
    let output = client.list_secrets()?;
    for secret in output.secrets {
        println!("{secret}");
    }
    Ok(())
}

/// Gets a secret from the store.
fn get(name: String) -> anyhow::Result<()> {
    let client = configured_client()?;
    let secret = client.get_secret(&name)?;
    println!("{}", secret.value);
    Ok(())
}

/// Sets up server credentials.
fn login(url: Option<String>) -> anyhow::Result<()> {
    let mut config = Config::load()?;

    let password = Password::new("Sesame password")
        .without_confirmation()
        .prompt()
        .context("failed to read password")?;

    if password.is_empty() {
        bail!("password cannot be empty");
    }

    if let Some(url) = url {
        config.url = url;
    }

    let client = Client::new(config.url.clone(), password.clone());
    client.list_secrets().context("login failed")?;

    config.password = password;
    config.store()?;
    Ok(())
}

/// Browse secrets using an interactive dialog.
fn browse() -> anyhow::Result<()> {
    let client = configured_client()?;

    loop {
        let output = client.list_secrets()?;

        let items = output.secrets;
        if items.is_empty() {
            bail!("there are no secrets to browse");
        }

        let name = Select::new("Select a secret", items)
            .prompt()
            .context("failed to read selection")?;

        let secret = client.get_secret(&name)?;
        println!("{}", secret.value);
    }
}
