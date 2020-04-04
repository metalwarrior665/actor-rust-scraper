use std::env;
use std::fs;
use serde_json::{from_str, Value};
use crate::input::{Input};
use rand::Rng;

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

// I'm not using reference because trying to make borrow checker happy
pub async fn push_data_async (data: Vec<Value>, client: &reqwest::Client, force_cloud: bool) {
    let is_on_apify = get_is_on_apify();
    if is_on_apify {
        let json = serde_json::to_string(&data).unwrap();
        let default_dataset = env::var("APIFY_DEFAULT_DATASET_ID").unwrap();
        let token = env::var("APIFY_TOKEN").unwrap();
        let url = format!("https://api.apify.com/v2/datasets/{}/items?token={}", default_dataset, token);
        client.post(&url).body(json).header("Content-Type", "application/json").send().await.unwrap();
    } else if force_cloud {
        let json = serde_json::to_string(&data).unwrap();
        let cloud_test_dataset = "w7xbAHYhyoz3v8K8r";
        let token = env::var("APIFY_TOKEN").unwrap();
        let url = format!("https://api.apify.com/v2/datasets/{}/items?token={}", cloud_test_dataset, token);
        client.post(&url).body(json).header("Content-Type", "application/json").send().await.unwrap();
    } else {
        data.iter().enumerate().for_each(|(i, val)| {
            let json = serde_json::to_string(&val).unwrap();
            let mut rng = rand::thread_rng();
            let path = format!("apify_storage/datasets/default/{}.json", rng.gen::<i32>());
            fs::write(path, json).unwrap();
        });    
    }
}

pub fn get_value (key: &str) -> Input {
    let is_on_apify = get_is_on_apify();
    println!("Is on Apify? -> {}", is_on_apify);
    let json = if is_on_apify {
        let default_kv = env::var("APIFY_DEFAULT_KEY_VALUE_STORE_ID").unwrap();
        // println!("Default KV -> {}", default_kv);
        let url = format!("https://api.apify.com/v2/key-value-stores/{}/records/{}", default_kv, key);
        let client = reqwest::blocking::Client::builder().build().unwrap();
        let val = request_text(&url, &client);
        // println!("Loaded value from KV -> {}", val);
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
pub fn request_text(url: &str, client: &reqwest::blocking::Client) -> String {
    client.get(url).send().unwrap().text().unwrap()
}

pub async fn request_text_async(url: String, client: &reqwest::Client) -> Result<String, reqwest::Error>{
    Ok(client.get(&url).send().await?.text().await?) 
}