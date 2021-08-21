use crate::errors::ConstellationError::BadIp;
use crate::state::{AppState, IpAsnMapping, ASN};
use chrono::prelude::*;
use chrono::Duration;
use std::collections::{HashMap, HashSet};
use tokio::time::sleep;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::TokioAsyncResolver;

pub async fn run(state: AppState, period: Duration) -> anyhow::Result<()> {
    let resolver = &TokioAsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())?;

    loop {
        let start: DateTime<Utc> = Utc::now(); // e.g. `2014-11-28T12:45:59.324310806Z`
        let mut ips_tbd: Vec<String> = vec![];
        let mut work_ip_asn: HashMap<String, IpAsnMapping> = HashMap::new();
        let mut work_asn: HashMap<String, ASN>;
        {
            let the_state = state.lock().unwrap();
            for ip in &the_state.new_ips_bgp {
                if !the_state.ip_asn.contains_key(ip) {
                    ips_tbd.push(ip.to_string());
                }
            }
            work_asn = the_state.asn.clone();
        }
        if !ips_tbd.is_empty() {
            {
                log::info!("New IPS = {}", ips_tbd.len());
                for ip in ips_tbd {
                    match grab_asn_for_ip(resolver, &ip).await {
                        Ok(ip_asn_det) => match ip_asn_det {
                            Some(det) => {
                                log::info!("Filled in IP {}", ip);
                                work_ip_asn.insert(ip.to_string(), det.clone());
                                if !work_asn.contains_key(&det.asn) {
                                    match grab_asn_details(resolver, &det.asn).await {
                                        Ok(asn_det) => match asn_det {
                                            Some(a) => {
                                                let mut the_state = state.lock().unwrap();
                                                work_asn.insert(det.asn.clone(), a.clone());
                                                the_state.asn.insert(det.asn.clone(), a);
                                            }
                                            None => {
                                                log::info!("ASN - no response {}", &det.asn);
                                            }
                                        },
                                        Err(e) => {
                                            log::error!(
                                                "Fetching info for ASN AS{} - {}",
                                                &det.asn,
                                                e.to_string()
                                            );
                                        }
                                    };
                                }
                                {
                                    let mut the_state = state.lock().unwrap();
                                    the_state.ip_asn.insert(ip.clone(), det.clone());
                                    the_state.new_ips_bgp.remove(&ip);
                                    match the_state.asn_ip.get(&det.asn) {
                                        Some(set) => {
                                            let mut new_set = set.clone();
                                            new_set.insert(ip);
                                            the_state.asn_ip.insert(det.asn, new_set.clone());
                                        }
                                        None => {
                                            let mut set: HashSet<String> = HashSet::new();
                                            set.insert(ip);
                                            the_state.asn_ip.insert(det.asn, set.clone());
                                        }
                                    }
                                }
                            }
                            None => {
                                log::info!("Filled in IP {} - no response", ip);
                            }
                        },
                        Err(e) => {
                            log::error!("Fetching info for IP {} - {}", ip, e.to_string())
                        }
                    }
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
        } else {
            log::info!("No new IPs to scan");
            sleep(period.to_std()?).await;
        }
    }
}

async fn grab_asn_for_ip(
    resolver: &TokioAsyncResolver,
    ip: &str,
) -> anyhow::Result<Option<IpAsnMapping>> {
    log::info!("Grabbing ASN for IP {}", ip);
    let bits = ip.split(".").collect::<Vec<&str>>();
    if bits.len() == 4 {
        let hostname = format!(
            "{}.{}.{}.{}.origin.asn.cymru.com.",
            bits[3], bits[2], bits[1], bits[0]
        );
        Ok(match dns_resolve_txt(resolver, &hostname).await? {
            Some(ip_asn_mapping) => {
                let bits = ip_asn_mapping.split("|").collect::<Vec<_>>();
                let asn_num = bits[0].trim().split(" ").collect::<Vec<&str>>();
                Some(IpAsnMapping {
                    asn: asn_num[0].trim().to_string(),
                    range: bits[1].trim().to_string(),
                    country: bits[2].trim().to_string(),
                    network: bits[3].trim().to_string(),
                    last_updated: Utc::now(),
                })
            }
            None => {
                log::info!("Unable to resolve {} via {}", ip, hostname);
                None
            }
        })
    } else {
        Err(BadIp(ip.to_string()).into())
    }
}
async fn grab_asn_details(resolver: &TokioAsyncResolver, asn: &str) -> anyhow::Result<Option<ASN>> {
    log::info!("Grabbing ASN details for AS{}", asn);

    let hostname = format!("as{}.asn.cymru.com.", asn);
    Ok(match dns_resolve_txt(resolver, &hostname).await? {
        Some(ip_asn_mapping) => {
            let bits = ip_asn_mapping.split("|").collect::<Vec<_>>();
            Some(ASN {
                asn: bits[0].trim().to_string(),

                country: bits[1].trim().to_string(),
                net: bits[2].trim().to_string(),
                desc: bits[4].trim().to_string(),
                last_updated: Utc::now(),
            })
        }
        None => {
            log::info!("Unable to resolve AS{} via {}", asn, hostname);
            None
        }
    })
}
async fn dns_resolve_txt(
    resolver: &TokioAsyncResolver,
    hostname: &str,
) -> anyhow::Result<Option<String>> {
    let txt_lookup = resolver.txt_lookup(hostname.clone()).await?;
    match txt_lookup
        .iter()
        .map(|f| f.to_string())
        .collect::<Vec<_>>()
        .first()
    {
        Some(txt_return) => Ok(Some(txt_return.to_string())),
        None => Ok(None),
    }
}
