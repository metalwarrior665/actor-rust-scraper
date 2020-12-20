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

// I'm not using reference because trying to make borrow checker happy
pub async fn push_data (data: Vec<Value>, client: &reqwest::Client, force_cloud: bool) 
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let is_on_apify = get_is_on_apify();
    if is_on_apify {
        let json = serde_json::to_string(&data)?;
        let default_dataset = env::var("APIFY_DEFAULT_DATASET_ID")?;
        let token = env::var("APIFY_TOKEN")?;
        let url = format!("https://api.apify.com/v2/datasets/{}/items?token={}", default_dataset, token);
        client.post(&url).body(json).header("Content-Type", "application/json").send().await?;
    } else if force_cloud {
        let json = serde_json::to_string(&data)?;
        let cloud_test_dataset = "w7xbAHYhyoz3v8K8r";
        let token = env::var("APIFY_TOKEN")?;
        let url = format!("https://api.apify.com/v2/datasets/{}/items?token={}", cloud_test_dataset, token);
        client.post(&url).body(json).header("Content-Type", "application/json").send().await?;
    } else {
        for val in data.iter() {
            let json = serde_json::to_string(&val)?;
            let mut rng = rand::thread_rng();
            let path = format!("apify_storage/datasets/default/{}.json", rng.gen::<i32>());
            fs::write(path, json)?;
        } 
    }
    Ok(())
}

pub async fn get_value (key: &str) -> Result<Input, Box<dyn std::error::Error + Send + Sync>> {
    let is_on_apify = get_is_on_apify();
    println!("Is on Apify? -> {}", is_on_apify);
    let json = if is_on_apify {
        let default_kv = env::var("APIFY_DEFAULT_KEY_VALUE_STORE_ID")?;
        // println!("Default KV -> {}", default_kv);
        let url = format!("https://api.apify.com/v2/key-value-stores/{}/records/{}", default_kv, key);
        let client = reqwest::Client::builder().build()?;
        let val = request_text(&url, &client).await?;
        // println!("Loaded value from KV -> {}", val);
        val
    } else {
        fs::read_to_string("apify_storage/key_value_stores/default/INPUT.JSON")?
    };

    // We have to tell compiler that we want to Box the error
    from_str(&json).or_else(|err| Err(Box::new(err) as Box<dyn std::error::Error + Send + Sync>))
}

#[allow(dead_code)]
pub async fn set_value (key: &str, value: &[Value]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let is_on_apify = get_is_on_apify();
    let json = serde_json::to_string(&value)?;
    if is_on_apify {
        let default_kv = env::var("APIFY_DEFAULT_KEY_VALUE_STORE_ID")?;
        let token = env::var("APIFY_TOKEN")?;
        let url = format!("https://api.apify.com/v2/key-value-stores/{}/records/{}?token={}", default_kv, key, token);
        let client = reqwest::Client::new();
        client.put(&url).body(json).header("Content-Type", "application/json").send().await?;
    } else {
        fs::write("apify_storage/key_value_stores/default/OUTPUT.JSON", json)?;
    }
    Ok(())
}

pub async fn request_text(url: &str, client: &reqwest::Client) -> Result<String, reqwest::Error> {
    Ok(client.get(url).send().await?.text().await?) 
}