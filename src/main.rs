#![allow(dead_code)]

#[macro_use] extern crate serde_derive;

mod basic_crawler;
mod http_crawler;
mod requestlist;
mod request;
mod input;
mod storage;
mod proxy;
mod errors;

// SDK testing here
mod actor;
mod dataset;
mod utils;

use requestlist::RequestList;
use crate::http_crawler::HttpCrawler;
use crate::basic_crawler::{BasicCrawlerOptions};
use input::{Input};
use storage::{get_value}; 
// use apify::actor::Actor;

// To not compile libraries on Apify, it is important to not commit Cargo.lock

#[tokio::main]
async fn main() {
    let input: Input = get_value("INPUT").await.unwrap();
    println!("STATUS --- Loaded Input");

    let sources = input.urls.clone();

    let req_list = RequestList::new(sources, input.debug_log.unwrap_or(false));
    println!("STATUS --- Initialized RequestList Input");

    let options: BasicCrawlerOptions = input.to_options();

    let crawler = HttpCrawler::new(req_list, options);

    println!("STATUS --- Starting Crawler");
    
    crawler.run().await;
}
