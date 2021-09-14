use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use actix::prelude::*;

use actix_web::dev::Server;
use dotenv::dotenv;
use structopt::StructOpt;
use tokio::task::JoinHandle;

use constellation_shared::state::{AppState, State, StateVersion};
//use futures::FutureExt;
use std::collections::HashSet;

mod errors;
mod tasks;

/// VERSION number of package
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
/// NAME of package
pub const NAME: Option<&'static str> = option_env!("CARGO_PKG_NAME");
#[derive(StructOpt)]
struct Cli {
    #[structopt(
        name = "lcd",
        env = "TERRARUST_LCD",
        default_value = "https://lcd.terra.dev",
        short,
        long = "lcd-client-url",
        help = "https://lcd.terra.dev is main-net"
    )]
    // Terra cli Client daemon
    lcd_endpoint: String,
    #[structopt(
        name = "rpc",
        about = "RPC endpoint",
        env = "TERRARUST_RPC_ENDPOINT",
        default_value = "http://public-node.terra.dev:26657"
    )]
    rpc_endpoint: String,
    #[structopt(
        name = "fcd",
        about = "fcd endpoint",
        env = "TERRARUST_FCD_ENDPOINT",
        default_value = "https://fcd.terra.dev"
    )]
    _fcd_endpoint: String,
    #[structopt(
        name = "chain",
        env = "TERRARUST_CHAIN",
        default_value = "columbus-4",
        long = "chain",
        help = "tequila-0004 is testnet, columbus-4 is main-net"
    )]
    chain_id: String,
    #[structopt(
        name = "address-book",
        env = "ADDRESS_BOOK_MAIN",
        default_value = "https://network.terra.dev/addrbook.json",
        help = "default address book for columbus"
    )]
    // Terra cli Client daemon
    address_book: String,

    #[structopt(
        name = "state-file",
        default_value = "state.json",
        help = "where to store state to survive restarts"
    )]
    // state file for checkpoints/backups
    state_file: String,
    #[structopt(
        name = "geodb-file",
        default_value = "db/GeoLite2-City.mmdb",
        help = "maxmind city db file"
    )]
    // state file for checkpoints/backups
    db_file: String,
    #[structopt(
        name = "run-modules",
        env = "CONSTELLATION_RUN",
        default_value = "all",
        help = "what modules to run"
    )]
    run_modules: String,
    #[structopt(
        name = "discord-url",
        env = "DISCORD_URL",
        long,
        default_value = "https://discordapp.com/",
        help = "endpoint for discord api"
    )]
    discord_url: String,
    #[structopt(
        name = "discord-token",
        env = "DISCORD_TOKEN",
        long,
        help = "token for discord api"
    )]
    discord_token: String,
    #[structopt(
        name = "discord-retries",
        env = "DISCORD_RETRIES",
        default_value = "4",
        long,
        help = "#retries for discord api, when we hit discord API caps"
    )]
    discord_retries: usize,
}

async fn run() -> anyhow::Result<()> {
    let cli: Cli = Cli::from_args();
    let discord_token = cli.discord_token;
    //env::var("DISCORD_TOKEN").expect("Expected a discord token in the environment");
    let discord_url = cli.discord_url; // env::var("DISCORD_URL").expect("Expected a discord URL in the environment");

    let discord_retries: usize = cli.discord_retries;
    let mut modules: HashSet<_> = cli.run_modules.split(',').collect();
    if modules.contains("validator") {
        modules.insert("observer");
    }

    for module in modules.iter() {
        log::info!("Module {} enabled", module)
    }

    let state_data = match State::restore(&cli.state_file) {
        Ok(versioned) => match versioned {
            StateVersion::StateVersion1(state_v1) => state_v1,
        },
        Err(e) => {
            log::info!(
                "State file {} unable to be read. ({}) starting new",
                cli.state_file,
                e
            );
            State::new()?
        }
    };
    let state: AppState = Arc::new(Mutex::new(state_data));
    let mut tasks: Vec<JoinHandle<_>> = vec![];
    // let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = vec![];
    let (tx_web, _rx_web) = mpsc::channel::<Server>();
    //    let (tx_observer, rx_observer) = mpsc::channel::<()>();

    if modules.contains("all") || modules.contains("address-book") {
        tasks.push(actix_rt::spawn(constellation_address_book::run(
            //tasks.push(tokio::task::spawn(constellation_address_book::run(
            state.clone(),
            Duration::from_secs(60 * 5),
            cli.address_book,
        )));
    }
    if modules.contains("all") || modules.contains("bgp") {
        tasks.push(actix_rt::spawn(constellation_bgp::run(
            state.clone(),
            Duration::from_secs(60 * 5),
        )));
    }
    if modules.contains("all") || modules.contains("checkpoint") {
        tasks.push(actix_rt::spawn(constellation_state_checkpoint::run(
            state.clone(),
            Duration::from_secs(60),
            cli.state_file,
        )));
    }
    if modules.contains("all") || modules.contains("geo") {
        tasks.push(actix_rt::spawn(constellation_geo::run(
            state.clone(),
            Duration::from_secs(60 * 5),
            cli.db_file,
        )));
    }
    if modules.contains("all") || modules.contains("rpc") {
        tasks.push(actix_rt::spawn(constellation_rpc_crawler::run(
            state.clone(),
            Duration::from_secs(60 * 5),
            cli.chain_id.clone(),
            cli.lcd_endpoint.clone(),
            cli.rpc_endpoint.clone(),
        )));
    }
    if modules.contains("all") || modules.contains("validator") {
        log::info!("Validator turned on");
        tasks.push(actix_rt::spawn(constellation_validator::run(
            state.clone(),
            Duration::from_secs(60 * 5),
            cli.chain_id.clone(),
            cli.lcd_endpoint.clone(),
        )));
        let validator_actor = constellation_validator::actor::ValidatorActor::create(
            &cli.lcd_endpoint,
            &cli.chain_id,
        )
        .await?;
        validator_actor.start();
    }

    if modules.contains("all") || modules.contains("observer") {
        tasks.push(actix_rt::spawn(constellation_observer::run(
            state.clone(),
            //tx_observer,
            "wss://observer.terra.dev/".into(),
        )));
        let oracle_actor =
            constellation_observer::actor::OracleActor::create(&cli.lcd_endpoint, &cli.chain_id)
                .await?;

        oracle_actor.start();
    }

    if modules.contains("all") || modules.contains("discord") {
        let discord_actor = constellation_discord::actor::DiscordValidatorActor::create(
            &discord_token,
            &discord_url,
            discord_retries,
        )
        .await?;
        discord_actor.start();

        let bot = actix_rt::spawn(constellation_discord::run(
            state.clone(),
            discord_token.clone(),
            //  discord_category_name.clone(),
            discord_url.clone(),
            discord_retries,
        ));

        tasks.push(bot);
        //bot.await?;
    }
    if modules.contains("all") || modules.contains("web") {
        let web_join = actix_rt::spawn(constellation_web::run(
            state.clone(),
            tx_web,
            NAME.unwrap_or("constellation"),
            VERSION.unwrap_or("dev"),
        ));
        tasks.push(web_join);
    }
    // TODO - respawn failed tasks

    let returns = futures::future::join_all(tasks).await;

    for result in returns {
        if let Err(e) = result {
            log::error!("task failed? {}", e)
        }
    }
    /*
        returns
            .iter()
            .for_each(|task_fut_result| match task_fut_result {
                Ok(task) => match task {
                    /* this would be where something inside the task killed itself */
                    Ok(_task_return) => {}
                    Err(e) => {
                        log::error!("Task internal fail: {}", e)
                    }
                },
                Err(e) => {
                    log::error!("Task fail: {}", e)
                }
            });
    */
    //   web_join.await?;

    Ok(())
}
//#[tokio::main]
#[actix_web::main]
async fn main() {
    dotenv().ok(); // this fails if .env isn't present. It is safe to be ignored
    env_logger::init();
    if let Err(ref err) = run().await {
        log::error!("{}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| log::error!("because: {}", cause));
    }
}
