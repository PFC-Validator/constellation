use chrono::Utc;
use constellation_shared::state::AppState;
use std::time::Duration;
use tokio::time;
pub async fn run(state: AppState, period: Duration, checkpoint_file: String) {
    let mut interval = time::interval(period);

    loop {
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

        interval.tick().await;
    }
}
