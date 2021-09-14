use chrono::Utc;
use constellation_shared::state::{AppState, GeoCity, GeoContinent, GeoCountry};
use maxminddb::geoip2::City;
use maxminddb::MaxMindDBError;
use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::net::{AddrParseError, IpAddr};
use std::time::Duration;
use tokio::time;

pub async fn run(state: AppState, period: Duration, db_filename: String) -> anyhow::Result<()> {
    let mut interval = time::interval(period);

    loop {
        let maxmind = maxminddb::Reader::open_readfile(&db_filename)?;

        let mut ips_tbd: Vec<String> = vec![];

        {
            let the_state = state.lock().unwrap();
            for ip in &the_state.new_ips_geo {
                if !the_state.geo_ip_country.contains_key(ip) {
                    ips_tbd.push(ip.to_string());
                }
            }
        }
        if !ips_tbd.is_empty() {
            {
                log::info!("New IPS = {}", ips_tbd.len());
                for ip in ips_tbd {
                    let ip_addr_r: Result<IpAddr, AddrParseError> = ip.parse();
                    match ip_addr_r {
                        Ok(ip_add) => {
                            let city_r: Result<City, MaxMindDBError> = maxmind.lookup(ip_add);
                            match city_r {
                                Ok(city) => {
                                    let city_id = city
                                        .city
                                        .as_ref()
                                        .map(|f| f.geoname_id.unwrap_or(0))
                                        .unwrap_or(0);
                                    let country_id = city
                                        .country
                                        .as_ref()
                                        .map(|f| f.geoname_id.unwrap_or(0))
                                        .unwrap_or(0);
                                    let continent_id = city
                                        .continent
                                        .as_ref()
                                        .map(|f| f.geoname_id.unwrap_or(0))
                                        .unwrap_or(0);

                                    let mut the_state = state.lock().unwrap();
                                    if let Some(cx) = city.city {
                                        let name = cx
                                            .names
                                            .map(|b| *b.get("en").unwrap_or(&"-none-"))
                                            .map(|f| f.to_string());
                                        if let Entry::Vacant(e) =
                                            the_state.geo_city.entry(cx.geoname_id.unwrap_or(0))
                                        {
                                            e.insert(
                                                //cx.geoname_id.unwrap_or(0),
                                                GeoCity {
                                                    geoname_id: cx.geoname_id.unwrap_or(0),
                                                    name,
                                                    country: country_id,
                                                    continent: continent_id,
                                                    last_updated: Utc::now(),
                                                },
                                            );
                                        }
                                        the_state.geo_ip_city.insert(ip.clone(), city_id);
                                        let mut s = match the_state.geo_city_ip.get(&city_id) {
                                            Some(set) => set.clone(),
                                            None => HashSet::new(),
                                        };
                                        s.insert(ip.clone());
                                        the_state.geo_city_ip.insert(city_id, s);
                                    };
                                    if let Some(cx) = city.country {
                                        let name = cx
                                            .names
                                            .map(|b| *b.get("en").unwrap_or(&"-none-"))
                                            .map(|f| f.to_string());
                                        if let Entry::Vacant(e) =
                                            the_state.geo_country.entry(cx.geoname_id.unwrap_or(0))
                                        {
                                            e.insert(GeoCountry {
                                                geoname_id: cx.geoname_id.unwrap_or(0),
                                                name,
                                                is_in_european_union: cx.is_in_european_union,
                                                iso_code: cx.iso_code.map(|f| f.to_string()),
                                                last_updated: Utc::now(),
                                            });
                                        }
                                        the_state.geo_ip_country.insert(ip.clone(), country_id);
                                        let mut s = match the_state.geo_country_ip.get(&country_id)
                                        {
                                            Some(set) => set.clone(),
                                            None => HashSet::new(),
                                        };
                                        s.insert(ip.clone());
                                        the_state.geo_country_ip.insert(country_id, s);
                                    };
                                    if let Some(cx) = city.continent {
                                        let name = cx
                                            .names
                                            .map(|b| *b.get("en").unwrap_or(&"-none-"))
                                            .map(|f| f.to_string());
                                        if let Entry::Vacant(e) = the_state
                                            .geo_continent
                                            .entry(cx.geoname_id.unwrap_or(0))
                                        {
                                            e.insert(GeoContinent {
                                                geoname_id: cx.geoname_id.unwrap_or(0),
                                                name,
                                                code: cx.code.map(|f| f.to_string()),
                                                last_updated: Utc::now(),
                                            });
                                        }
                                        the_state.geo_ip_continent.insert(ip.clone(), continent_id);
                                        let mut s =
                                            match the_state.geo_continent_ip.get(&continent_id) {
                                                Some(set) => set.clone(),
                                                None => HashSet::new(),
                                            };
                                        s.insert(ip.clone());
                                        the_state.geo_continent_ip.insert(continent_id, s);
                                    };
                                    the_state.new_ips_geo.remove(&ip);
                                }
                                Err(e) => {
                                    log::error!("DB Error {} {}", ip, e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Unable to parse IP#{} {}", ip, e);
                        }
                    }
                }
            }
        } else {
            log::info!("No new IPs to scan");
        }

        interval.tick().await;
    }
}
