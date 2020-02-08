use crate::requestlist::RequestList;
use crate::request::Request;
use crate::input::{Extract, ProxySettings};
use crate::{extract_data_from_url_async,extract_data_from_url};
use crate::proxy:: {get_apify_proxy};
use crate::storage:: {push_data_async};

use std::sync::{Arc, Mutex};

use async_std::task;
// use async_std::prelude::Future;

use rayon::prelude::*;

pub struct Crawler {
    request_list: RequestList,
    extract: Vec<Extract>,
    push_data_size: usize,
    push_data_buffer: Arc<Mutex<Vec<serde_json::Value>>>,
    client: reqwest::Client,
    proxy_client: reqwest::Client,
    blocking_client: reqwest::blocking::Client,
    proxy_blocking_client: reqwest::blocking::Client
}

impl Crawler {
    pub fn new(request_list: RequestList, extract: Vec<Extract>, proxy_settings: Option<ProxySettings>, push_data_size: usize) -> Crawler {
        let client = reqwest::Client::builder().build().unwrap();
        let blocking_client = reqwest::blocking::Client::builder().build().unwrap();

        let proxy = get_apify_proxy(&proxy_settings);
        let (proxy_client, proxy_blocking_client) = match proxy {
            Some(proxy) => {
                let proxy_client = reqwest::Client::builder()
                    .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                    .build().unwrap();

                let proxy_blocking_client = reqwest::blocking::Client::builder()
                    .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                    .build().unwrap();
                (proxy_client, proxy_blocking_client)
            },
            None => {
                (client.clone(), blocking_client.clone())
            }
        };
        Crawler {
            request_list,
            extract,
            push_data_size,
            push_data_buffer: Arc::new(Mutex::new(Vec::with_capacity(push_data_size))),
            client,
            proxy_client,
            blocking_client,
            proxy_blocking_client
        }
    }
    
    pub fn run(&self) {
        self.request_list.sources.par_iter().for_each(|req| extract_data_from_url(req, &self.extract, &self.blocking_client, &self.proxy_blocking_client));
    }   

    pub async fn run_async(self) { 
        let futures = self.request_list.sources.iter().map(|req| extract_data_from_url_async(req.clone(), &self.extract, &self.client, &self.proxy_client, self.push_data_size, self.push_data_buffer.clone()));
        
        let fut = futures::future::join_all(futures.into_iter()).await;

        // After we are done looping, we need to flush the push_data_buffer one last time
        let locked_vec = self.push_data_buffer.lock().unwrap();

        // TODO: We should not need to clone here
        push_data_async(locked_vec.clone(), &self.client).await; 
    } 
}