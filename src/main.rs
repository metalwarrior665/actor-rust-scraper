#![allow(dead_code)]

extern crate reqwest;
extern crate serde_json;
extern crate scraper;
extern crate serde;
extern crate tokio;
extern crate futures;
extern crate rand;

#[macro_use] extern crate serde_derive;


use std::time::{Instant};
use std::clone::Clone;
use std::collections::HashMap;

use std::sync::{ Arc };
use futures::lock::{Mutex};
// use async_std::prelude::*;
use async_std::task;

use scraper::{Selector, Html};
use serde_json::{Value};

mod crawler;
mod requestlist;
mod request;
mod input;
mod storage;
mod proxy;

use request::Request;
use requestlist::RequestList;
use crate::crawler::Crawler;
use input::{Input, Extract, ExtractType};
use storage::{ push_data_async, request_text_async, get_value}; 

// To not compile libraries on Apify, it is important to not commit Cargo.lock

#[tokio::main]
async fn main() {
    let input: Input = get_value("INPUT");
    println!("STATUS --- Loaded Input");

    let sources = input.urls.iter().map(|req| Request::new(req.url.clone())).collect();

    let req_list = RequestList::new(sources);
    println!("STATUS --- Initialized RequestList Input");

    let crwl  = Crawler::new(req_list, input.extract, input.proxy_settings, input.push_data_size,
        input.force_cloud, input.debug_log, input.max_concurrency);

    println!("STATUS --- Starting Async Crawler");
    // Comment on/off depending on using tokio
    // task::block_on(async {
    crwl.run_async().await;
    // })
    
}

async fn extract_data_from_url_async(
        req: Request,
        extract: Vec<Extract>,
        client: reqwest::Client,
        proxy_client: reqwest::Client,
        push_data_size: usize,
        push_data_buffer: Arc<futures::lock::Mutex<Vec<serde_json::Value>>>,
        force_cloud: bool,
        debug_log: bool
) {

    let url = req.url.clone();
    if debug_log {
        println!("Started async extraction --- {}", url);
    }

    let now = Instant::now();
    let response = request_text_async(url, &proxy_client).await;
    let request_time = now.elapsed().as_millis();

    // println!("Reqwest retuned");
    match response {
        Ok(html) => {
            let mut map: HashMap<String, Value> = HashMap::new();
            let parse_time;
            let extract_time;
            {
                let now = Instant::now();
                let dom = Html::parse_document(&html).clone();
                parse_time = now.elapsed().as_millis();
            
                let now = Instant::now();
                extract.iter().for_each(|extr| {
                    let selector_bind = &extr.selector.clone();
                    let selector = Selector::parse(selector_bind).unwrap();
                    let element = dom.select(&selector).next();
                    let val = match element {
                        Some(element) => {
                            // println!("matched element");
                            let extracted_value = match &extr.extract_type {
                                ExtractType::Text => element.text().fold(String::from(""), |acc, s| acc + s).trim().to_owned(),
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
                });
                extract_time = now.elapsed().as_millis();
            }
            
            let map_size = map.len();
        
            let value = serde_json::to_value(map).unwrap();
    
            let now = Instant::now();
        
            {
                let mut locked_vec = push_data_buffer.lock().await;
                locked_vec.push(value);
                let vec_len = locked_vec.len();
                if debug_log {
                    println!("Push  data buffer length:{}", vec_len);
                }
                if vec_len >= push_data_size {
                    println!("Flushing data buffer --- length: {}", locked_vec.len());
                    push_data_async(locked_vec.clone(), &client, force_cloud).await; 
                    locked_vec.truncate(0);
                    println!("Flushed data buffer --- length: {}", locked_vec.len());
                }
            }
            
            let push_time = now.elapsed().as_millis();
        
            if debug_log {
                println!(
                    "SUCCESS({}/{}) - {} - timings (in ms) - request: {}, parse: {}, extract: {}, push: {}",
                    map_size,
                    extract.len(),
                    req.url,
                    request_time,
                    parse_time,
                    extract_time,
                    push_time
                );
            }
        },
        Err(err) => {
            println!(
                "FAILURE({} - timings (in ms) - request: {} --- {}",
                err,
                request_time,
                req.url,
            );
        }
    }
    
}
