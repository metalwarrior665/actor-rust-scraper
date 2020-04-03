use crate::requestlist::RequestList;
use crate::input::{Extract, ProxySettings};
use crate::{extract_data_from_url_async};
use crate::proxy:: {get_apify_proxy};
use crate::storage:: {push_data_async};
use crate::tokio;
use std::time::Duration;

use std::sync::{Arc};

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
    max_concurrency: usize
}

impl Crawler {
    pub fn new(
        request_list: RequestList,
        extract: Vec<Extract>,
        proxy_settings: Option<ProxySettings>,
        push_data_size: usize,
        force_cloud: bool,
        debug_log: bool,
        max_concurrency: usize
    ) -> Crawler {
        println!("STATUS --- Initializing crawler");
        let client = reqwest::Client::builder().build().unwrap();

        let proxy = get_apify_proxy(&proxy_settings);
        let proxy_client = match proxy {
            Some(proxy) => {
                let proxy_client = reqwest::Client::builder()
                    .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                    .build().unwrap();
                proxy_client
            },
            None => {
                client.clone()
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
            max_concurrency
        }
    }

    pub async fn run_async(self) { 
        let mut task_handles = vec![];

        // This is noly to keep copies for later
        let push_data_buffer_2 = self.push_data_buffer.clone();
        let client_2 = self.client.clone();
        let force_cloud = self.force_cloud;

        loop {
            // Check concurrency
            let in_progress = {
                let locked_state = self.request_list.state.lock().await;
                locked_state.in_progress.len()
            };

            if in_progress >= self.max_concurrency {
                // println!("Max concurrency {} reached, waiting", self.max_concurrency);
                tokio::time::delay_for(Duration::from_millis(10)).await;
                continue;
            }

            let req = match self.request_list.fetch_next_request().await {
                None => break,
                Some(req) => req
            };

            let extract = self.extract.clone();
            let client = self.client.clone();
            let proxy_client = self.proxy_client.clone();
            let push_data_buffer = self.push_data_buffer.clone();
            let force_cloud = self.force_cloud;
            let push_data_size = self.push_data_size;
            let debug_log = self.debug_log;

            // Sending the state via Arc to a thread
            let state = self.request_list.state.clone();
            
            let handle = tokio::task::spawn(async move {
                let task_handle = extract_data_from_url_async(
                    req.clone(),
                    extract,
                    client,
                    proxy_client,
                    push_data_size,
                    push_data_buffer,
                    force_cloud,
                    debug_log,
                ).await;

                // lock start
                let mut locked_state = state.lock().await;
                locked_state.in_progress.remove(&req.url);
                println!("In progress count:{}", locked_state.in_progress.len());
                task_handle
                // lock end
            });
            task_handles.push(handle);
        }
        
        for handle in task_handles {
            handle.await;
        }

        // After we are done looping, we need to flush the push_data_buffer one last time
        let locked_vec = push_data_buffer_2.lock().await;

        // TODO: We should not need to clone here
        println!("Remaing Push data buffer length before last push:{}", locked_vec.len());
        push_data_async(locked_vec.clone(), &client_2, force_cloud).await; 
    } 
}

// First working solution for async
/*
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
*/