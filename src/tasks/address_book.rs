use crate::state::AppState;
use chrono::prelude::*;
use chrono::Duration;
use terra_rust_api::AddressBook;

use std::collections::HashSet;
use tokio::time::sleep;

pub async fn run(
    state: AppState,
    period: Duration,
    address_book_url: String,
) -> anyhow::Result<()> {
    loop {
        let start: DateTime<Utc> = Utc::now(); // e.g. `2014-11-28T12:45:59.324310806Z`
        match grab_address_book(&address_book_url).await {
            Ok(the_book) => {
                let mut nodes = state.lock().unwrap();
                the_book.addrs.iter().for_each(|entry| {
                    if !nodes.nodes.contains_key(&entry.addr.to_string()) {
                        nodes.nodes.insert(entry.addr.to_string(), entry.clone());
                        nodes.new_nodes.insert(entry.addr.to_string());
                        nodes.new_ips_bgp.insert(entry.addr.ip.clone());
                        nodes.new_ips_geo.insert(entry.addr.ip.clone());
                    }

                    let mut s = match nodes.id_ip_addr.get(&entry.addr.id) {
                        Some(set) => set.clone(),
                        None => HashSet::new(),
                    };
                    s.insert(entry.addr.clone());
                    nodes
                        .id_ip_addr
                        .insert((&entry.addr.id.clone()).to_string(), s);
                    let mut s = match nodes.ip_ip_addr.get(&entry.addr.ip) {
                        Some(set) => set.clone(),
                        None => HashSet::new(),
                    };
                    s.insert(entry.addr.clone());
                    nodes
                        .ip_ip_addr
                        .insert((&entry.addr.ip.clone()).to_string(), s);
                })
            }
            Err(e) => {
                log::error!("Error: {}", e);
            }
        }
        let now = Utc::now();
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

async fn grab_address_book(address_book_url: &str) -> anyhow::Result<AddressBook> {
    log::info!("Grabbing {}", address_book_url);
    let addresses = terra_rust_api::Terra::address_book(address_book_url).await?;
    Ok(addresses)
}
