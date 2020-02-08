extern crate reqwest;
extern crate serde_json;
extern crate scraper;
extern crate serde;
extern crate rayon;
extern crate tokio;
extern crate futures;

#[macro_use] extern crate serde_derive;

use std::time::{Instant};
use std::clone::Clone;
use std::collections::HashMap;

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
use input::{Input, Extract, ExtractType, ProxySettings};
use storage::{ push_data_async, request_text_async, push_data,request_text, get_value}; //
use proxy::{get_apify_proxy};


// To not compile libraries on Apify, it is important to not commit Cargo.lock

#[tokio::main]
async fn main() {
    let input: Input = get_value("INPUT");
    println!("STATUS --- Loaded Input");

    /*
    let input: Input = Input {
        run_async: true,
        urls: vec![Request { url: String::from("https://www.amazon.com/dp/B01CYYU8YW") }],
        extract: vec![Extract {field_name: String::from("field") , selector: String::from("#productTitle"), extract_type:  ExtractType::Text }],
        proxy_settings: Some(ProxySettings {useApifyProxy: true, apifyProxyGroups: None })
    };
    */

    let sources = input.urls.iter().map(|req| Request::new(req.url.clone())).collect();

    let req_list = RequestList::new(sources);

    let crwl  = Crawler::new(req_list, input.extract, input.proxy_settings);

    if input.run_async {
        println!("STATUS --- Starting Async Crawler");
        // Comment on/off depending on using tokio
        // task::block_on(async {
            crwl.run_async().await;
        // })
    } else {
        println!("STATUS --- Starting Sync Crawler");
        crwl.run();
    }
}

fn extract_data_from_url(req: &Request, extract: &Vec<Extract>, client: &reqwest::blocking::Client) {
    let now = Instant::now();
    let html = request_text(&req.url, &client);
    let request_time = now.elapsed().as_millis();

    let now = Instant::now();
    let dom = Html::parse_document(&html);
    let parse_time = now.elapsed().as_millis();

    let mut map: HashMap<String, Value> = HashMap::new();

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

    let mapSize = map.len();

    let value = serde_json::to_value(map).unwrap();
    let extractTime = now.elapsed().as_millis();

    let now = Instant::now();
    push_data(&vec![value]);
    let push_time = now.elapsed().as_millis();

    println!(
        "SUCCESS({}/{}) - {} - timings (in ms) - request: {}, parse: {}, extract: {}, push: {}",
        mapSize,
        extract.len(),
        &req.url,
        request_time,
        parse_time,
        extractTime,
        push_time
    );
}

async fn extract_data_from_url_async(req: Request, extract: Vec<Extract>, client: reqwest::Client) {
    // println!("started async extraction");

    let now = Instant::now();
    let url = req.url.clone();
    let response = request_text_async(url, &client).await;
    let request_time = now.elapsed().as_millis();

    // println!("Reqwest retuned");
    match response {
        Ok(html) => {
            let now = Instant::now();
            let dom = Html::parse_document(&html).clone();
            let parse_time = now.elapsed().as_millis();
        
            let mut map: HashMap<String, Value> = HashMap::new();
        
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
        
            let mapSize = map.len();
        
            let value = serde_json::to_value(map).unwrap();
            let extract_time = now.elapsed().as_millis();
        
            let now = Instant::now();
        
            // Should later convert to async string once figure out borrow checker
            push_data_async(vec![value], &client).await; 
            //push_data_async(vec![value].clone()).await;
            let push_time = now.elapsed().as_millis();
        
            println!(
                "SUCCESS({}/{}) - {} - timings (in ms) - request: {}, parse: {}, extract: {}, push: {}",
                mapSize,
                extract.len(),
                req.url,
                request_time,
                parse_time,
                extract_time,
                push_time
            );
        },
        Err(err) => {
            println!(
                "FAILURE({} - timings (in ms) - request: {} --- error: {}",
                req.url,
                request_time,
                err
            );
        }
    }
    
}
