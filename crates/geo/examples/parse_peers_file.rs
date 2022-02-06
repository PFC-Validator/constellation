extern crate core;

use anyhow::Result;
use dotenv::dotenv;
use maxminddb::geoip2::Country;
use std::fs::File;
use std::io::BufRead;
use std::net::IpAddr;
use std::path::Path;
use std::{env, io};

pub fn main() -> Result<()> {
    dotenv().ok(); // this fails if .env isn't present. It is safe to be ignored
    env_logger::init();
    let maxminddb_file = env::args().nth(1).expect("missing a maxmind db file");

    let maxminddb = maxminddb::Reader::open_readfile(&maxminddb_file)?;

    let file_name = env::args().nth(2).expect("missing a input file");
    let path = Path::new(&file_name);
    let file = File::open(path).expect("Could not open file ");
    let buf = io::BufReader::new(file).lines();
    println!("Peerid\tEU\tCountry");
    for line in buf {
        let line_string = line?;
        if !line_string.starts_with("Peerlist for") {
            let node_ip_port_triples = line_string.split(",");
            for triple_str in node_ip_port_triples {
                if let Some(triple) = parse_triple(triple_str) {
                    if !triple.ip.starts_with("[") {
                        match triple.ip.parse::<IpAddr>() {
                            Ok(ip_addr) => match maxminddb.lookup::<Country>(ip_addr) {
                                Ok(country) => {
                                    if let Some(country_x) = country.country {
                                        let name = match country_x.names {
                                            Some(names_bt) => {
                                                names_bt.get("en").unwrap_or(&"?noen?")
                                            }
                                            None => country_x.iso_code.unwrap_or(&"NOISO"),
                                        };
                                        println!(
                                            "{}\t{}\t{}",
                                            triple_str,
                                            country_x.is_in_european_union.unwrap_or(false),
                                            name
                                        )
                                    } else {
                                        log::warn!("{:?} no country info?", triple,)
                                    }
                                }
                                Err(e) => log::error!("MaxMind: {} {}", ip_addr, e),
                            },
                            Err(e) => log::error!("IP4 Parsing:{} {}", triple.ip, e),
                        }
                    } else {
                        log::info!("IP6 not supported {:?}", triple)
                    }
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug)]
pub struct NodeTriple {
    pub node_id: String,
    pub ip: String,
    pub port: String,
}
pub fn parse_triple(in_triple: &str) -> Option<NodeTriple> {
    if let Some(node_end) = in_triple.find("@") {
        if let Some(ip_end) = in_triple.rfind(":") {
            return Some(NodeTriple {
                node_id: in_triple[..node_end].to_string(),
                ip: in_triple[node_end + 1..ip_end].to_string(),
                port: in_triple[ip_end + 1..].to_string(),
            });
        }
    }

    return None;
}
