#![allow(dead_code)]

#[macro_use] extern crate serde_derive;

mod crawler;
mod requestlist;
mod request;
mod input;
mod storage;
mod proxy;
mod extract_fn;
mod errors;

// SDK testing here
mod actor;
mod dataset;
mod utils;

use requestlist::RequestList;
use crate::crawler::{Crawler, CrawlerOptions};
use input::{Input};
use storage::{get_value}; 
// use apify::actor::Actor;

// To not compile libraries on Apify, it is important to not commit Cargo.lock

#[tokio::main]
async fn main() {
    let input: Input = get_value("INPUT").await.unwrap();
    println!("STATUS --- Loaded Input");

    let sources = input.urls.clone();

    let req_list = RequestList::new(sources);
    println!("STATUS --- Initialized RequestList Input");

    let options: CrawlerOptions = input.to_options();

    let crawler = Crawler::new(req_list, options);

    println!("STATUS --- Starting Crawler");
    
    crawler.run().await;
}
