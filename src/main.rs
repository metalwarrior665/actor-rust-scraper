extern crate reqwest;
extern crate serde_json;
extern crate scraper;
extern crate chrono;
extern crate serde;
extern crate rayon;

#[macro_use] extern crate serde_derive;

use std::clone::Clone;
use std::collections::HashMap;

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
use storage::{get_value, push_data, request_text};
use proxy::{get_apify_proxy};

fn main() {
    let input: Input = get_value("INPUT");

    let sources = input.urls; //.iter().map(|req| Request::new(req.url.clone())).collect();

    let req_list = RequestList::new(sources);

    let crwl = Crawler::new(req_list, input.extract, input.proxy_settings);

    crwl.run(extract_data_from_url);
}

fn extract_data_from_url(req: &Request, extract: &Vec<Extract>, proxy_settings: &Option<ProxySettings>) {
    let proxy_url = get_apify_proxy(&proxy_settings);
    let html = request_text(&req.url, &proxy_url);

    let dom = Html::parse_document(&html);

    let mut map: HashMap<String, Value> = HashMap::new();

    // let tuples: Vec<(String, String)> = 

    extract.iter().for_each(|extr| {
        let selector_bind = &extr.selector.clone();
        let selector = Selector::parse(selector_bind).unwrap();
        let element = dom.select(&selector).next();
        let val = match element {
            Some(element) => {
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

    let value = serde_json::to_value(map).unwrap();
    push_data(&vec![value]);
}
