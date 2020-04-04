#![allow(dead_code)]

extern crate reqwest;
extern crate serde_json;
extern crate scraper;
extern crate serde;
extern crate tokio;
extern crate futures;
extern crate rand;

#[macro_use] extern crate serde_derive;

mod crawler;
mod requestlist;
mod request;
mod input;
mod storage;
mod proxy;
mod extract_fn;
mod errors;

use request::Request;
use requestlist::RequestList;
use crate::crawler::Crawler;
use input::{Input};
use storage::{get_value}; 

// To not compile libraries on Apify, it is important to not commit Cargo.lock

#[tokio::main]
async fn main() {
    let input: Input = get_value("INPUT").await.unwrap();
    println!("STATUS --- Loaded Input");

    let sources = input.urls.iter().map(|req| Request::new(req.url.clone())).collect();

    let req_list = RequestList::new(sources);
    println!("STATUS --- Initialized RequestList Input");

    let crawler  = Crawler::new(req_list, input.extract, input.proxy_settings, input.push_data_size,
        input.force_cloud, input.debug_log, input.max_concurrency);

    println!("STATUS --- Starting Async Crawler");
    
    crawler.run().await;
}
