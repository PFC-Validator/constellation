use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use terra_rust_api::addressbook::NodeAddr;
use terra_rust_api::terra_datetime_format;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ASN {
    pub asn: String,
    pub country: String,
    pub net: String,
    pub desc: String,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct IpAsnMapping {
    pub asn: String,
    pub range: String,
    pub country: String,
    pub network: String,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct State {
    #[serde(with = "terra_datetime_format")]
    pub last_saved: DateTime<Utc>,
    pub nodes: HashMap<String, NodeAddr>,
    pub new_nodes: HashSet<String>,
    pub new_ips: HashSet<String>,
    pub ip_asn: HashMap<String, IpAsnMapping>,
    pub asn: HashMap<String, ASN>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum StateVersion {
    StateVersion1(State),
}

impl State {
    pub fn new() -> anyhow::Result<State> {
        Ok(State {
            last_saved: DateTime::from(DateTime::parse_from_rfc3339("2019-10-12T07:20:50.52Z")?),
            nodes: HashMap::new(),
            new_nodes: HashSet::new(),
            new_ips: HashSet::new(),
            ip_asn: HashMap::new(),
            asn: HashMap::new(),
        })
    }

    pub fn restore(restore_file_name: &str) -> anyhow::Result<StateVersion> {
        let br = BufReader::new(fs::File::open(restore_file_name)?);
        let versioned: StateVersion = serde_json::from_reader(br)?;
        match &versioned {
            StateVersion::StateVersion1(s) => {
                log::info!("Restoring from V1 checkpoint taken on {}", s.last_saved);
            }
        }

        Ok(versioned)
    }
    pub fn save(self, checkpoint_file: &str) -> anyhow::Result<()> {
        let check_point = &self.last_saved.clone();
        let versioned = StateVersion::StateVersion1(self);
        let json = serde_json::to_string_pretty(&versioned)?;
        fs::write(checkpoint_file, json)?;
        log::info!("V1 checkpoint taken on {}", check_point);
        Ok(())
    }
}
pub(crate) type AppState = Arc<Mutex<State>>;
