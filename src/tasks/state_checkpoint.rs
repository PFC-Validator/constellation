use crate::state::AppState;
use chrono::prelude::*;
use chrono::Duration;
use tokio::time::sleep;

pub async fn run(state: AppState, period: Duration, checkpoint_file: String) -> anyhow::Result<()> {
    loop {
        let start: DateTime<Utc> = Utc::now(); // e.g. `2014-11-28T12:45:59.324310806Z`
        let mut state_c = { state.lock().unwrap().clone() };

        let now = Utc::now();
        state_c.last_saved = now;
        match state_c.save(&checkpoint_file) {
            Ok(_) => {
                let mut the_state = state.lock().unwrap();
                the_state.last_saved = now;
            }
            Err(e) => {
                log::error!("Unable to save checkpoint file {} {}", checkpoint_file, e)
            }
        }

        let spent = now - start;
        if period - spent > Duration::seconds(1) {
            let sleep_time = period - spent;
            log::info!("Sleeping for {} ", sleep_time);

            sleep(sleep_time.to_std()?).await;
        } else {
            log::debug!("no rest for the wicked")
        }
    }
}
