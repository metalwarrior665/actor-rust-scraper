use std::env;
use std::fs;
use serde_json::{from_str, Value};
use crate::input::{Input};
use crate::proxy::Proxy;

pub fn get_is_on_apify() -> bool {
    match env::var("APIFY_IS_AT_HOME") {
        Ok(ref x) if x == "1"  => true,
        _ => false
    }
}

fn create_indexed_key (index: usize) -> String {
    let string = index.to_string();
    let mut key = string;
    while key.len() != 8 {
        key = String::from("0") + &key;
    }
    key
}

pub fn push_data (data: &Vec<Value>) {
    let is_on_apify = get_is_on_apify();
    if is_on_apify {
        let json = serde_json::to_string(&data).unwrap();
        let default_dataset = env::var("APIFY_DEFAULT_DATASET_ID").unwrap();
        let token = env::var("APIFY_TOKEN").unwrap();
        let url = format!("https://api.apify.com/v2/datasets/{}/items?token={}", default_dataset, token);
        let client = reqwest::blocking::Client::new();
        client.post(&url).body(json).header("Content-Type", "application/json").send().unwrap();
    } else {
        data.iter().enumerate().for_each(|(i, val)| {
            let json = serde_json::to_string(&val).unwrap();
            let key = create_indexed_key(i);
            let path = format!("apify_storage/datasets/default/{}.json", key);
            fs::write(path, json).unwrap();
        });    
    }
}

// I'm not using reference because trying to make borrow checker happy
pub async fn push_data_async (data: Vec<Value>) {
    let is_on_apify = get_is_on_apify();
    if is_on_apify {
        let json = serde_json::to_string(&data).unwrap();
        let default_dataset = env::var("APIFY_DEFAULT_DATASET_ID").unwrap();
        let token = env::var("APIFY_TOKEN").unwrap();
        let url = format!("https://api.apify.com/v2/datasets/{}/items?token={}", default_dataset, token);
        let client = reqwest::Client::new();
        client.post(&url).body(json).header("Content-Type", "application/json").send().await.unwrap();
    } else {
        data.iter().enumerate().for_each(|(i, val)| {
            let json = serde_json::to_string(&val).unwrap();
            let key = create_indexed_key(i);
            let path = format!("apify_storage/datasets/default/{}.json", key);
            fs::write(path, json).unwrap();
        });    
    }
}

pub fn get_value (key: &str) -> Input {
    let is_on_apify = get_is_on_apify();
    println!("Is on Apify? -> {}", is_on_apify);
    let json = if is_on_apify {
        let default_kv = env::var("APIFY_DEFAULT_KEY_VALUE_STORE_ID").unwrap();
        println!("Default KV -> {}", default_kv);
        let url = format!("https://api.apify.com/v2/key-value-stores/{}/records/{}", default_kv, key);
        let val = request_text(&url, &None);
        println!("Loaded value from KV -> {}", val);
        val
    } else {
        fs::read_to_string("apify_storage/key_value_stores/default/INPUT.JSON").unwrap()
    };

    match from_str(&json) {
        Ok(input) => {
            println!("Parsed input into: {:?}", input);
            input
        },
        Err(error) => {
            println!("Parsing failed with error: {}", error);
            panic!("");
        }
    }
}

#[allow(dead_code)]
pub fn set_value (key: &str, value: &Vec<Value>) {
    let is_on_apify = get_is_on_apify();
    let json = serde_json::to_string(&value).unwrap();
    if is_on_apify {
        let default_kv = env::var("APIFY_DEFAULT_KEY_VALUE_STORE_ID").unwrap();
        let token = env::var("APIFY_TOKEN").unwrap();
        let url = format!("https://api.apify.com/v2/key-value-stores/{}/records/{}?token={}", default_kv, key, token);
        let client = reqwest::blocking::Client::new();
        client.put(&url).body(json).header("Content-Type", "application/json").send().unwrap();
    } else {
        fs::write("apify_storage/key_value_stores/default/OUTPUT.JSON", json).unwrap();
    }
    
}

// TODO: We should reuse connection pool for perf - see https://docs.rs/reqwest/0.10.1/reqwest/index.html
pub fn request_text(url: &str, proxy: &Option<Proxy>) -> String {
    match proxy {
        Some(proxy) => {
            let client = reqwest::blocking::Client::builder()
                .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                .build().unwrap();
            client.get(url).send().unwrap().text().unwrap()
        },
        None => reqwest::blocking::get(url).unwrap().text().unwrap()
    }
}

pub async fn request_text_async(url: String, proxy: &Option<Proxy>) -> String {
    println!("Doing reqwest");
    match proxy {
        Some(proxy) => {
            let client = reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                .build().unwrap();
            client.get(&url).send().await.unwrap().text().await.unwrap()
        },
        None => reqwest::get(&url).await.unwrap().text().await.unwrap()
    }
}