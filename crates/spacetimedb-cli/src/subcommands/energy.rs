// use clap::Arg;
use clap::{value_parser, Arg, ArgMatches};

use crate::config::Config;

pub fn cli() -> clap::Command {
    clap::Command::new("energy")
        .about("Invokes commands related to database budgets")
        .args_conflicts_with_subcommands(true)
        .subcommand_required(true)
        .subcommands(get_energy_subcommands())
}

fn get_energy_subcommands() -> Vec<clap::Command> {
    vec![
        clap::Command::new("status")
            .about("Show current energy balance for an identity")
            .arg(
                Arg::new("identity")
                    .help("The identity to check the balance for")
                    .long_help(
                    "The identity to check the balance for. If no identity is provided, the default one will be used.",
                ),
            ),
        clap::Command::new("set-balance")
            .about("Update the current budget balance for a database")
            .arg(
                Arg::new("balance")
                    .required(true)
                    .value_parser(value_parser!(u64))
                    .help("The balance value to set"),
            )
            .arg(
                Arg::new("identity")
                    .help("The identity to set a balance for")
                    .long_help(
                        "The identity to set a balance for. If no identity is provided, the default one will be used.",
                    ),
            )
            .arg(
                Arg::new("quiet")
                    .long("quiet")
                    .short('q')
                    .action(clap::ArgAction::SetTrue)
                    .help("Runs command in silent mode"),
            ),
    ]
}

async fn exec_subcommand(config: Config, cmd: &str, args: &ArgMatches) -> Result<(), anyhow::Error> {
    match cmd {
        "status" => exec_status(config, args).await,
        "set-balance" => exec_update_balance(config, args).await,
        unknown => Err(anyhow::anyhow!("Invalid subcommand: {}", unknown)),
    }
}

pub async fn exec(config: Config, args: &ArgMatches) -> Result<(), anyhow::Error> {
    let (cmd, subcommand_args) = args.subcommand().expect("Subcommand required");
    exec_subcommand(config, cmd, subcommand_args).await
}

async fn exec_update_balance(config: Config, args: &ArgMatches) -> Result<(), anyhow::Error> {
    // let project_name = args.value_of("project name").unwrap();
    let hex_identity = args.get_one::<String>("identity");
    let balance = *args.get_one::<u64>("balance").unwrap();
    let quiet = args.get_flag("quiet");

    let hex_identity = if let Some(hex_identity) = hex_identity {
        hex_identity
    } else {
        config.get_default_identity_config().unwrap().identity.as_str()
    };

    let client = reqwest::Client::new();

    let res = set_balance(&client, &config, hex_identity, balance).await?;

    if !quiet {
        let body = res.bytes().await?;
        let str = String::from_utf8(body.to_vec())?;
        println!("{}", str);
    }

    Ok(())
}

async fn exec_status(config: Config, args: &ArgMatches) -> Result<(), anyhow::Error> {
    // let project_name = args.value_of("project name").unwrap();
    let hex_identity = args.get_one::<String>("identity");

    let hex_identity = if let Some(hex_identity) = hex_identity {
        hex_identity
    } else {
        config.get_default_identity_config().unwrap().identity.as_str()
    };

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/energy/{}", config.get_host_url(), hex_identity))
        .send()
        .await?;

    let res = res.error_for_status()?;
    let body = res.bytes().await?;
    let str = String::from_utf8(body.to_vec())?;
    println!("{}", str);
    Ok(())
}

pub(super) async fn set_balance(
    client: &reqwest::Client,
    config: &Config,
    hex_identity: &str,
    balance: u64,
) -> anyhow::Result<reqwest::Response> {
    // TODO: this really should be form data in POST body, not query string parameter, but gotham
    // does not support that on the server side without an extension.
    // see https://github.com/gotham-rs/gotham/issues/11
    let url = format!("{}/energy/{}", config.get_host_url(), hex_identity);
    let res = client.post(url).query(&[("balance", balance)]).send().await?;
    let res = res.error_for_status()?;
    Ok(res)
}
