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

use basic_crawler::CrawlingContext;
use requestlist::RequestList;
// use crate::http_crawler::HttpCrawler;
use crate::basic_crawler::{BasicCrawler,BasicCrawlerOptions};
use input::{Input};
use storage::{get_value}; 
use tokio::time::sleep;
// use apify::actor::Actor;

// To not compile libraries on Apify, it is important to not commit Cargo.lock

/*
Stuck on this error
implementation of `FnOnce` is not general enough
...`FnOnce<(&'0 request::Request, CrawlingContext<'_>)>` would have to be implemented for the type `for<'_, 'a> fn(&request::Request, CrawlingContext<'a>) -> impl futures::Future {my_innocent_fn}`, for some specific lifetime `'0`...
...but `FnOnce<(&request::Request, CrawlingContext<'a>)>` is actually implemented for the type `for<'_, 'a> fn(&request::Request, CrawlingContext<'a>) -> impl futures::Future {my_innocent_fn}`
*/

#[tokio::main]
async fn main() {
    let input: Input = get_value("INPUT").await.unwrap();
    println!("STATUS --- Loaded Input");

    let sources = input.urls.clone();

    let req_list = RequestList::new(sources, input.debug_log.unwrap_or(false));
    println!("STATUS --- Initialized RequestList Input");

    let options: BasicCrawlerOptions = input.to_options();

    let crawler = BasicCrawler::new(req_list, options, my_innocent_fn);
    // let crawler = HttpCrawler::new(req_list, options);

    println!("STATUS --- Starting Crawler");
    
    crawler.run().await;
}

use request::Request;
use basic_crawler::HandleRequestOutput;

async fn my_innocent_fn(
    req: &Request,
    //context: CrawlingContext<'a>
) -> HandleRequestOutput {
    sleep(std::time::Duration::from_millis(5000)).await;
    println!("Running with {}", req.url);
    Ok(())
}
