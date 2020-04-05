use std::collections::HashMap;
use std::time::{Instant};
use std::sync::{ Arc };

use scraper::{Selector, Html};
use serde_json::{Value};
// use rand::Rng;

use crate::input::{Extract, ExtractType};
use crate::storage::{ push_data, request_text};
use crate::request::Request;
// use crate::errors::CrawlerError;


pub async fn extract_data_from_url(
    req: Request, // immutable here
    extract: Vec<Extract>,
    client: reqwest::Client,
    proxy_client: reqwest::Client,
    push_data_buffer: Arc<futures::lock::Mutex<Vec<serde_json::Value>>>,
    force_cloud: bool,
    debug_log: bool
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = req.url;
    if debug_log {
        println!("Started extraction --- {}", url);
    }

    // Random fail for testing errors
    /*
    {
        let mut rng = rand::thread_rng();
        if rng.gen::<bool>() {
            return Err(Box::new(CrawlerError::new(String::from("Testing error"))));
        }
    }
    */

    let now = Instant::now();
    let html = request_text(&url, &proxy_client).await?;
    let request_time = now.elapsed().as_millis();

    // println!("Reqwest retuned");
    let mut map: HashMap<String, Value> = HashMap::new();
    let parse_time;
    let extract_time;
    {
        let now = Instant::now();
        let dom = Html::parse_document(&html).clone();
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

    let now = Instant::now();

    {
        let mut locked_vec = push_data_buffer.lock().await;
        locked_vec.push(value);
        let vec_len = locked_vec.len();
        if debug_log {
            println!("Push data buffer length:{}", vec_len);
        }
        if vec_len >= locked_vec.capacity() { // Capacity should never grow over original push_data_size
            println!("Flushing data buffer --- length: {}", locked_vec.len());
            push_data(locked_vec.clone(), &client, force_cloud).await?; 
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
            url,
            request_time,
            parse_time,
            extract_time,
            push_time
        );
    }
    Ok(())
}