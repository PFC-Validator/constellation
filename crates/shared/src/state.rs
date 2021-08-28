use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::BufReader;
use std::sync::{Arc, Mutex};
use terra_rust_api::addressbook::{NodeAddr, NodeIDIPPort};
use terra_rust_api::terra_datetime_format;

#[derive(Deserialize, Serialize, Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct ASN {
    pub asn: String,
    pub country: String,
    pub net: String,
    pub desc: String,
    #[serde(with = "terra_datetime_format")]
    pub last_updated: DateTime<Utc>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct IpAsnMapping {
    pub asn: String,
    pub range: String,
    pub country: String,
    pub network: String,
    #[serde(with = "terra_datetime_format")]
    pub last_updated: DateTime<Utc>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GeoCity {
    pub geoname_id: GeoID,
    pub name: Option<String>,
    pub country: GeoID,
    pub continent: GeoID,

    #[serde(with = "terra_datetime_format")]
    pub last_updated: DateTime<Utc>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GeoContinent {
    pub geoname_id: GeoID,
    pub name: Option<String>,
    pub code: Option<String>,

    #[serde(with = "terra_datetime_format")]
    pub last_updated: DateTime<Utc>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GeoCountry {
    pub geoname_id: GeoID,
    pub name: Option<String>,
    pub is_in_european_union: Option<bool>,
    pub iso_code: Option<String>,

    #[serde(with = "terra_datetime_format")]
    pub last_updated: DateTime<Utc>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ValidatorStat {
    pub validator_address: String,
    //pub commits: [i8; 60], // -1 not set, 0 fail, 1 commit
    pub commit_index: usize,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct State {
    #[serde(with = "terra_datetime_format")]
    pub last_saved: DateTime<Utc>,
    pub nodes: HashMap<String, NodeAddr>,
    pub ip_ip_addr: HashMap<String, HashSet<NodeIDIPPort>>,
    pub id_ip_addr: HashMap<String, HashSet<NodeIDIPPort>>,
    pub new_nodes: HashSet<String>,
    pub new_ips_bgp: HashSet<String>,
    pub new_ips_geo: HashSet<String>,
    pub ip_asn: HashMap<String, IpAsnMapping>,
    /// ips in a ASN (1->Many)
    pub asn_ip: HashMap<String, HashSet<String>>,
    pub asn: HashMap<String, ASN>,
    /// geo based info
    pub geo_city: HashMap<GeoID, GeoCity>,
    pub geo_country: HashMap<GeoID, GeoCountry>,
    pub geo_continent: HashMap<GeoID, GeoContinent>,
    pub geo_ip_city: HashMap<String, GeoID>,
    pub geo_ip_country: HashMap<String, GeoID>,
    pub geo_ip_continent: HashMap<String, GeoID>,
    pub geo_city_ip: HashMap<GeoID, HashSet<String>>,
    pub geo_country_ip: HashMap<GeoID, HashSet<String>>,
    pub geo_continent_ip: HashMap<GeoID, HashSet<String>>,
    //  pub validator_stats: HashMap<String, ValidatorStat>,
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
            ip_ip_addr: Default::default(),
            id_ip_addr: Default::default(),
            new_nodes: HashSet::new(),
            new_ips_bgp: HashSet::new(),
            new_ips_geo: HashSet::new(),
            ip_asn: HashMap::new(),
            asn: HashMap::new(),
            geo_city: Default::default(),
            geo_country: Default::default(),
            geo_continent: Default::default(),
            geo_ip_city: Default::default(),
            geo_ip_country: Default::default(),
            geo_ip_continent: Default::default(),
            geo_city_ip: Default::default(),
            geo_country_ip: Default::default(),
            asn_ip: HashMap::new(),
            geo_continent_ip: Default::default(),
            //        validator_stats: Default::default(),
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
pub type AppState = Arc<Mutex<State>>;
pub type GeoID = u32;
