mod errors;
mod state;
mod tasks;

use crate::state::{AppState, State, StateVersion};
use actix_web::dev::Server;
use dotenv::dotenv;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use structopt::StructOpt;
use tokio::task::JoinHandle;

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
    let (tx, rx) = mpsc::channel::<Server>();

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
        Duration::from_secs(5),
        cli.chain_id,
        cli.lcd_endpoint,
        cli.rpc_endpoint,
        // cli.fcd_endpoint,
        //   "ukrw".into(),
        //   1.4,
    )));
    tasks.push(tokio::task::spawn(tasks::web::run(state.clone(), tx)));
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
    Ok(())
}
#[tokio::main]
async fn main() {
    dotenv().ok(); // this fails if .env isn't present
    env_logger::init();

    if let Err(ref err) = run().await {
        log::error!("{}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| log::error!("because: {}", cause));

        // The backtrace is not always generated. Try to run this example
        // with `$env:RUST_BACKTRACE=1`.
        /*
        if let Some(backtrace) = err.backtrace() {
            log::debug!("backtrace: {:?}", backtrace);
        }

         */

        ::std::process::exit(1);
    }
}
