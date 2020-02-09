use crate::requestlist::RequestList;
use crate::request::Request;
use crate::input::{Extract, ProxySettings};
use crate::{extract_data_from_url_async,extract_data_from_url};
use crate::proxy:: {get_apify_proxy};
use crate::storage:: {push_data_async, push_data};

use std::sync::{Arc};

use async_std::task;
// use async_std::prelude::Future;

use rayon::prelude::*;

pub struct Crawler {
    request_list: RequestList,
    extract: Vec<Extract>,
    force_cloud: bool,
    debug_log: bool,
    push_data_size: usize,
    push_data_buffer: Arc<futures::lock::Mutex<Vec<serde_json::Value>>>,
    push_data_buffer_sync: Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
    client: reqwest::Client,
    proxy_client: reqwest::Client,
    blocking_client: reqwest::blocking::Client,
    proxy_blocking_client: reqwest::blocking::Client
}

impl Crawler {
    pub fn new(
        request_list: RequestList,
        extract: Vec<Extract>,
        proxy_settings: Option<ProxySettings>,
        push_data_size: usize,
        force_cloud: bool,
        debug_log: bool
    ) -> Crawler {
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
            force_cloud,
            debug_log,
            push_data_size,
            push_data_buffer: Arc::new(futures::lock::Mutex::new(Vec::with_capacity(push_data_size))),
            push_data_buffer_sync: Arc::new(std::sync::Mutex::new(Vec::with_capacity(push_data_size))),
            client,
            proxy_client,
            blocking_client,
            proxy_blocking_client
        }
    }
    
    pub fn run(&self) {
        self.request_list.sources.par_iter().for_each(|req| extract_data_from_url(
            req,
            &self.extract,
            &self.blocking_client,
            &self.proxy_blocking_client,
            self.push_data_size,
            self.push_data_buffer_sync.clone(),
            self.force_cloud,
            self.debug_log
        ));

        // After we are done looping, we need to flush the push_data_buffer one last time
        let locked_vec = self.push_data_buffer_sync.lock().unwrap();

        // TODO: We should not need to clone here
        println!("Remaing Push data buffer length before last push:{}", locked_vec.len());
        push_data(locked_vec.clone(), &self.blocking_client, self.force_cloud); 
    }   

    pub async fn run_async(self) { 
        let futures = self.request_list.sources.iter().map(|req| extract_data_from_url_async(
            req.clone(),
            &self.extract,
            &self.client,
            &self.proxy_client,
            self.push_data_size,
            self.push_data_buffer.clone(),
            self.force_cloud,
            self.debug_log
        ));
        
        let fut = futures::future::join_all(futures.into_iter()).await;

        // After we are done looping, we need to flush the push_data_buffer one last time
        let locked_vec = self.push_data_buffer.lock().await;

        // TODO: We should not need to clone here
        println!("Remaing Push data buffer length before last push:{}", locked_vec.len());
        push_data_async(locked_vec.clone(), &self.client, self.force_cloud).await; 
    } 
}