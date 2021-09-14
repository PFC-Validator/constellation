use actix_web::dev::Server;
use actix_web::{middleware, web, App, Error as AWError, HttpRequest, HttpResponse, HttpServer};
use constellation_shared::state::{
    AppState, GeoCity, GeoContinent, GeoCountry, GeoID, IpAsnMapping, ASN,
};

use serde::Serialize;
use std::collections::HashSet;
use std::sync::mpsc;
use terra_rust_api::addressbook::{NodeAddr, NodeIDIPPort};

pub async fn run(state: AppState, _tx: mpsc::Sender<Server>) {
    // srv is server controller type, `dev::Server`
    let local = tokio::task::LocalSet::new();

    let srv = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            // enable logger
            .wrap(middleware::Logger::default())
            .service(web::resource("/index.html").to(|| async { "Hello world!" }))
            .service(web::resource("/city").route(web::get().to(cities)))
            .service(web::resource("/city/{id:\\d+}").route(web::get().to(city_detail)))
            .service(web::resource("/country").route(web::get().to(countries)))
            .service(web::resource("/country/{id:\\d+}").route(web::get().to(country_detail)))
            .service(web::resource("/continent").route(web::get().to(continent)))
            .service(web::resource("/continent/{id:\\d+}").route(web::get().to(continent_detail)))
            .service(web::resource("/asn").route(web::get().to(asns)))
            .service(web::resource("/asn/{asn:\\d+}").route(web::get().to(asn_detail)))
            .service(web::resource("/node").route(web::get().to(nodes)))
            .service(web::resource("/node/{node:\\w+}").route(web::get().to(node_detail)))
            .service(
                web::resource("/ip/{ip:\\d+\\.\\d+\\.\\d+\\.\\d+}").route(web::get().to(ip_detail)),
            )
    })
    .bind("0.0.0.0:8080");
    match srv {
        Ok(server) => {
            let s = server.run();
            //   let _ = tx.send(s);
            local.spawn_local(s);
            // s.await?;
        }
        Err(e) => {
            log::error!("Fail to start webserver {}", e);
        }
    }

    // run future
    //  sys.block_on(srv)?;
    // Ok(())
}

async fn cities(req: HttpRequest) -> Result<HttpResponse, AWError> {
    let r = req.app_data::<AppState>().unwrap().lock().unwrap();
    Ok(HttpResponse::Ok().json(&r.geo_city))
}

#[derive(Serialize)]
struct CityIP<'a> {
    city: Option<&'a GeoCity>,
    ip: Option<&'a HashSet<String>>,
}
async fn city_detail(req: HttpRequest) -> Result<HttpResponse, AWError> {
    match req.match_info().get("id").unwrap_or("0").parse::<GeoID>() {
        Ok(id) => {
            let r = req.app_data::<AppState>().unwrap().lock().unwrap();
            let city = r.geo_city.get(&id);
            let city_ip = r.geo_city_ip.get(&id);
            Ok(HttpResponse::Ok().json(CityIP { city, ip: city_ip }))
        }
        Err(_e) => Ok(HttpResponse::NotAcceptable().body("bad id")),
    }
}

async fn countries(req: HttpRequest) -> Result<HttpResponse, AWError> {
    let r = req.app_data::<AppState>().unwrap().lock().unwrap();
    Ok(HttpResponse::Ok().json(&r.geo_country))
}

#[derive(Serialize)]
struct CountryIP<'a> {
    country: Option<&'a GeoCountry>,
    ip: Option<&'a HashSet<String>>,
}
async fn country_detail(req: HttpRequest) -> Result<HttpResponse, AWError> {
    match req.match_info().get("id").unwrap_or("0").parse::<GeoID>() {
        Ok(id) => {
            let r = req.app_data::<AppState>().unwrap().lock().unwrap();
            let country = r.geo_country.get(&id);
            let country_ip = r.geo_country_ip.get(&id);
            Ok(HttpResponse::Ok().json(CountryIP {
                country,
                ip: country_ip,
            }))
        }
        Err(_e) => Ok(HttpResponse::NotAcceptable().body("bad id")),
    }
}
async fn continent(req: HttpRequest) -> Result<HttpResponse, AWError> {
    let r = req.app_data::<AppState>().unwrap().lock().unwrap();
    Ok(HttpResponse::Ok().json(&r.geo_continent))
}

#[derive(Serialize)]
struct ContinentIP<'a> {
    continent: Option<&'a GeoContinent>,
    ip: Option<&'a HashSet<String>>,
}
async fn continent_detail(req: HttpRequest) -> Result<HttpResponse, AWError> {
    match req.match_info().get("id").unwrap_or("0").parse::<GeoID>() {
        Ok(id) => {
            let r = req.app_data::<AppState>().unwrap().lock().unwrap();
            let continent = r.geo_continent.get(&id);
            let continent_ip = r.geo_continent_ip.get(&id);
            Ok(HttpResponse::Ok().json(ContinentIP {
                continent,
                ip: continent_ip,
            }))
        }
        Err(_e) => Ok(HttpResponse::NotAcceptable().body("bad id")),
    }
}

async fn asns(req: HttpRequest) -> Result<HttpResponse, AWError> {
    let r = req.app_data::<AppState>().unwrap().lock().unwrap();
    Ok(HttpResponse::Ok().json(&r.asn))
}

#[derive(Serialize)]
struct ASNDetail<'a> {
    asn: Option<&'a ASN>,
    ip: Option<&'a HashSet<String>>,
}
async fn asn_detail(req: HttpRequest) -> Result<HttpResponse, AWError> {
    match req.match_info().get("asn").unwrap_or("0").parse::<usize>() {
        Ok(id) => {
            let r = req.app_data::<AppState>().unwrap().lock().unwrap();
            let asn = r.asn.get(&id.to_string());
            let asn_ip = r.asn_ip.get(&id.to_string());
            Ok(HttpResponse::Ok().json(ASNDetail { asn, ip: asn_ip }))
        }
        Err(_e) => Ok(HttpResponse::NotAcceptable().body("bad asn")),
    }
}

async fn nodes(req: HttpRequest) -> Result<HttpResponse, AWError> {
    let r = req.app_data::<AppState>().unwrap().lock().unwrap();
    Ok(HttpResponse::Ok().json(&r.nodes))
}

#[derive(Serialize)]
struct NodeDetail {
    id: String,
    id_ip_port: HashSet<NodeIDIPPort>,
    nodes: Vec<NodeAddr>,
}
async fn node_detail(req: HttpRequest) -> Result<HttpResponse, AWError> {
    match req
        .match_info()
        .get("node")
        .unwrap_or("0")
        .parse::<String>()
    {
        Ok(node) => {
            let r = req.app_data::<AppState>().unwrap().lock().unwrap();
            let empty: HashSet<NodeIDIPPort> = HashSet::new();
            let id_ip_port = r.id_ip_addr.get(&node).unwrap_or(&empty);
            let mut nodes: Vec<NodeAddr> = vec![];
            id_ip_port.iter().for_each(|f| {
                if let Some(n) = r.nodes.get(&f.to_string()) {
                    nodes.push(n.clone())
                }
            });
            Ok(HttpResponse::Ok().json(NodeDetail {
                id: node,
                id_ip_port: id_ip_port.clone(),
                nodes,
            }))
        }
        Err(_e) => Ok(HttpResponse::NotAcceptable().body("bad id")),
    }
}

#[derive(Serialize)]
struct IPDetail {
    ip: String,
    ip_id_port: HashSet<NodeIDIPPort>,
    nodes: Vec<NodeAddr>,
    asn_ip: Option<IpAsnMapping>,
    asn: Option<ASN>,
    city: Option<GeoCity>,
    country: Option<GeoCountry>,
    continent: Option<GeoContinent>,
}
async fn ip_detail(req: HttpRequest) -> Result<HttpResponse, AWError> {
    match req.match_info().get("ip").unwrap_or("0").parse::<String>() {
        Ok(ip) => {
            let r = req.app_data::<AppState>().unwrap().lock().unwrap();

            let empty: HashSet<NodeIDIPPort> = HashSet::new();
            let ip_id_port = r.ip_ip_addr.get(&ip).unwrap_or(&empty);
            let mut nodes: Vec<NodeAddr> = vec![];
            ip_id_port.iter().for_each(|f| {
                if let Some(n) = r.nodes.get(&f.to_string()) {
                    nodes.push(n.clone())
                }
            });
            let city = match r.geo_ip_city.get(&ip) {
                Some(geoid) => r.geo_city.get(geoid),
                None => None,
            }
            .cloned();
            let country = match r.geo_ip_country.get(&ip) {
                Some(geoid) => r.geo_country.get(geoid),
                None => None,
            }
            .cloned();
            let continent = match r.geo_ip_continent.get(&ip) {
                Some(geoid) => r.geo_continent.get(geoid),
                None => None,
            }
            .cloned();
            let asn_ip = r.ip_asn.get(&ip).cloned();
            let asn = match &asn_ip {
                Some(aa) => r.asn.get(&aa.asn),
                None => None,
            }
            .cloned();

            Ok(HttpResponse::Ok().json(IPDetail {
                asn_ip,
                asn,
                city,
                country,
                continent,
                ip,
                ip_id_port: ip_id_port.clone(),
                nodes,
            }))
        }
        Err(_e) => Ok(HttpResponse::NotAcceptable().body("bad ip")),
    }
}
