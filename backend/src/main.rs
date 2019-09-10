#[macro_use]
extern crate log;

use actix::prelude::*;
use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Error};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

mod types;
mod aggregator;
mod chain;
mod node;
mod feed;
mod util;

use node::connector::NodeConnector;
use feed::connector::FeedConnector;
use aggregator::Aggregator;
use crate::util::Locator;

/// Entry point for connecting nodes
fn node_route(
    req: HttpRequest,
    stream: web::Payload,
    aggregator: web::Data<Addr<Aggregator>>,
    locator: web::Data<Addr<crate::util::Locator>>,
) -> Result<HttpResponse, Error> {
    let mut ip = String::from("127.0.0.1");
    if let Some(remote) = req.connection_info().remote() {
        let v: Vec<&str> = remote.split_terminator(':').collect();
        if !v.is_empty() {
            ip = v[0].to_string();
        }
    };

    ws::start(
        NodeConnector::new(aggregator.get_ref().clone(), locator.get_ref().clone(), ip),
        &req,
        stream,
    )
}

/// Entry point for connecting feeds
fn feed_route(
    req: HttpRequest,
    stream: web::Payload,
    aggregator: web::Data<Addr<Aggregator>>,
    _locator: web::Data<Addr<crate::util::Locator>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        FeedConnector::new(aggregator.get_ref().clone()),
        &req,
        stream,
    )
}

fn main() -> std::io::Result<()> {
    simple_logger::init_with_level(log::Level::Info).expect("Must be able to start a logger");

    let sys = System::new("substrate-telemetry");
    let aggregator = Aggregator::new().start();
    let cache = Arc::new(RwLock::new(HashMap::new()));
    let locator = SyncArbiter::start(4, move || Locator::new(cache.clone()));

    HttpServer::new(move || {
        App::new()
            .data(aggregator.clone())
            .data(locator.clone())
            .service(web::resource("/submit").route(web::get().to(node_route)))
            .service(web::resource("/feed").route(web::get().to(feed_route)))

    })
    .bind("127.0.0.1:8080")?
    .start();

    sys.run()
}
