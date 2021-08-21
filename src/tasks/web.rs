use crate::state::{AppState, GeoCity, GeoContinent, GeoCountry, GeoID, ASN};
use actix_web::dev::Server;
use actix_web::{
    middleware, rt, web, App, Error as AWError, HttpRequest, HttpResponse, HttpServer,
};

use serde::Serialize;
use std::collections::HashSet;
use std::sync::mpsc;

pub async fn run(state: AppState, tx: mpsc::Sender<Server>) -> anyhow::Result<()> {
    let mut sys = rt::System::new("test");
    // srv is server controller type, `dev::Server`
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
    })
    .bind("127.0.0.1:8080")?
    .run();
    let _ = tx.send(srv.clone());

    // run future
    sys.block_on(srv)?;
    Ok(())
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
