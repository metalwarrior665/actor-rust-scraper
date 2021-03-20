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

use crate::input::{Extract, ProxySettings, ExtractType};
use basic_crawler::BasicCrawler;
use requestlist::RequestList;
use crate::http_crawler::HttpCrawler;
use crate::basic_crawler::{BasicCrawlerOptions};
use input::{Input};
use storage::{get_value}; 
// use apify::actor::Actor;

// To not compile libraries on Apify, it is important to not commit Cargo.lock

use scraper::{Selector, Html};
use serde_json::{Value};
use std::collections::HashMap;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let input: Input = get_value("INPUT").await.unwrap();
    println!("STATUS --- Loaded Input");

    let sources = input.urls.clone();

    let req_list = RequestList::new(sources, input.debug_log.unwrap_or(false));
    println!("STATUS --- Initialized RequestList Input");

    let options: BasicCrawlerOptions = input.to_options();

    // let crawler = BasicCrawler::new(req_list, options);

    let crawler = HttpCrawler::new(req_list, options);

    println!("STATUS --- Starting Crawler");

    let mut scope = unsafe {
        async_scoped::TokioScope::create()
    };
    
    scope.spawn(async {
        crawler.run(handle_page_function).await;
    });

    let _ = scope.collect();
}

fn handle_page_function (dom: &Html, extract: &Vec<Extract>) 
-> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut map: HashMap<String, Value> = HashMap::new();
    let parse_time;
    let extract_time;
    {
        let now = Instant::now();
        
        parse_time = now.elapsed().as_millis();
    
        let now = Instant::now();
        for extr in extract.iter() {
            let selector_bind = &extr.selector.clone();
            // TODO: Implement std::Error
            let selector = Selector::parse(selector_bind).unwrap();
            let element = dom.select(&selector).next();
            let val = match element {
                Some(element) => {
                    // println!("matched element");
                    let extracted_value = match &extr.extract_type {
                        ExtractType::Text => element.text().fold(String::from(""), |acc, s| acc + s).trim().to_owned(),
                        // TODO: Implement std::Error
                        ExtractType::Attribute(at) => element.value().attr(&at).unwrap().to_owned()
                    };
                    Some(extracted_value)
                },
                None => None
            };
            let insert_value = match val {
                Some(string) => Value::String(string),
                None => Value::Null,
            };
            map.insert(extr.field_name.clone(), insert_value);
        }
        extract_time = now.elapsed().as_millis();
    }
    
    let map_size = map.len();

    let value = serde_json::to_value(map)?;
    Ok(value)
}
