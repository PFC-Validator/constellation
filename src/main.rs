use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

use actix::prelude::*;

use actix_web::dev::Server;
use dotenv::dotenv;
use structopt::StructOpt;
use tokio::task::JoinHandle;

use constellation_shared::state::{AppState, State, StateVersion};

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
        short,
        long = "chain",
        help = "tequila-0004 is testnet, columbus-4 is main-net"
    )]
    chain_id: String,
    #[structopt(
        name = "address-book",
        env = "ADDRESS_BOOK_MAIN",
        default_value = "https://network.terra.dev/addrbook.json",
        short,
        help = "default address book for columbus"
    )]
    // Terra cli Client daemon
    address_book: String,

    #[structopt(
        name = "state-file",
        default_value = "state.json",
        short,
        help = "where to store state to survive restarts"
    )]
    // state file for checkpoints/backups
    state_file: String,
    #[structopt(
        name = "geodb-file",
        default_value = "db/GeoLite2-City.mmdb",
        short,
        help = "maxmind city db file"
    )]
    // state file for checkpoints/backups
    db_file: String,
}

async fn run() -> anyhow::Result<()> {
    let cli: Cli = Cli::from_args();
    //    let local = tokio::task::LocalSet::new();

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
    let mut tasks: Vec<JoinHandle<anyhow::Result<()>>> = vec![];
    let (tx_web, _rx_web) = mpsc::channel::<Server>();
    //    let (tx_observer, rx_observer) = mpsc::channel::<()>();

    tasks.push(tokio::task::spawn(tasks::address_book::run(
        state.clone(),
        Duration::from_secs(60 * 5),
        cli.address_book,
    )));
    tasks.push(tokio::task::spawn(tasks::bgp_filler::run(
        state.clone(),
        Duration::from_secs(60 * 5),
    )));
    tasks.push(tokio::task::spawn(tasks::state_checkpoint::run(
        state.clone(),
        Duration::from_secs(60),
        cli.state_file,
    )));
    tasks.push(tokio::task::spawn(tasks::geo_filler::run(
        state.clone(),
        Duration::from_secs(60 * 5),
        cli.db_file,
    )));
    tasks.push(tokio::task::spawn(tasks::rpc_crawler::run(
        state.clone(),
        Duration::from_secs(60 * 5),
        cli.chain_id.clone(),
        cli.lcd_endpoint.clone(),
        cli.rpc_endpoint.clone(),
    )));
    tasks.push(tokio::task::spawn(constellation_validator::run(
        state.clone(),
        Duration::from_secs(60 * 5),
        cli.chain_id.clone(),
        cli.lcd_endpoint.clone(),
    )));

    tasks.push(tokio::task::spawn(constellation_observer::run(
        state.clone(),
        //tx_observer,
        "wss://observer.terra.dev/".into(),
    )));

    let web_join = actix_rt::spawn(tasks::web::run(state.clone(), tx_web));

    let oracle_actor =
        constellation_observer::actor::OracleActor::create(&cli.lcd_endpoint, &cli.chain_id)
            .await?;
    oracle_actor.start();
    let validator_actor =
        constellation_validator::actor::ValidatorActor::create(&cli.lcd_endpoint, &cli.chain_id)
            .await?;
    validator_actor.start();
    // TODO - respawn failed tasks
    let returns = futures::future::join_all(tasks).await;
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
    web_join.await?;
    Ok(())
}
//#[tokio::main]
#[actix_web::main]
async fn main() {
    dotenv().ok(); // this fails if .env isn't present
    env_logger::init();
    //let mut rt = tokio::runtime::Runtime::new().unwrap();
    if let Err(ref err) = run().await {
        log::error!("{}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| log::error!("because: {}", cause));
    }
}
