mod errors;
mod state;
mod tasks;

use crate::state::{AppState, State, StateVersion};
use dotenv::dotenv;
use std::sync::{Arc, Mutex};
use structopt::StructOpt;
use tokio::task::JoinHandle;

/// VERSION number of package
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
/// NAME of package
pub const NAME: Option<&'static str> = option_env!("CARGO_PKG_NAME");
#[derive(StructOpt)]
struct Cli {
    #[structopt(
        name = "node",
        env = "TERRAD_ENDPOINT",
        default_value = "http://localhost:26657",
        short,
        help = "terrad RPC endpoint is main-net"
    )]
    // Terra cli Client daemon
    _node: String,
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
        name = "lcd",
        env = "TERRARUST_LCD",
        default_value = "https://lcd.terra.dev",
        short,
        long = "lcd-client-url",
        help = "https://lcd.terra.dev is main-net"
    )]
    // Terra cli Client daemon
    _lcd: String,
    #[structopt(
        name = "state-file",
        default_value = "state.json",
        short,
        help = "where to store state to survive restarts"
    )]
    // state file for checkpoints/backups
    state_file: String,
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

    tasks.push(tokio::task::spawn(tasks::address_book::run(
        state.clone(),
        chrono::Duration::minutes(5),
        cli.address_book,
    )));
    tasks.push(tokio::task::spawn(tasks::bgp_filler::run(
        state.clone(),
        chrono::Duration::seconds(5),
    )));
    tasks.push(tokio::task::spawn(tasks::state_checkpoint::run(
        state.clone(),
        chrono::Duration::seconds(5),
        cli.state_file,
    )));
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
    dotenv().ok();
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
