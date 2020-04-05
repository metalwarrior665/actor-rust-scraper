use crate::requestlist::RequestList;
use crate::input::{Extract, ProxySettings};
use crate::extract_fn::extract_data_from_url;
use crate::proxy:: {get_apify_proxy};
use crate::storage:: {push_data};
use crate::tokio;
use std::time::Duration;

use std::sync::{Arc};

pub struct CrawlerOptions {
    extract: Vec<Extract>,
    proxy_settings: Option<ProxySettings>,
    force_cloud: bool,
    debug_log: bool,
    push_data_size: usize,
    max_concurrency: usize,
    max_request_retries: usize
}

impl Default for CrawlerOptions {
    fn default() -> Self {
        CrawlerOptions {
            debug_log: false,
            force_cloud: false,
            push_data_size: 500,
            max_concurrency: 200,
            max_request_retries: 3,
            proxy_settings: Some(Default::default()),
            extract: vec![Default::default()]
        }
    }
}

// Builders
impl CrawlerOptions {
    pub fn set_extract(&mut self, extract: Vec<Extract>) -> &mut Self {
        self.extract = extract;
        self
    }
    pub fn set_push_data_size(&mut self, push_data_size: usize) -> &mut Self {
        self.push_data_size = push_data_size;
        self
    }
    pub fn set_max_concurrency(&mut self, max_concurrency: usize) -> &mut Self {
        self.max_concurrency = max_concurrency;
        self    
    }
    pub fn set_max_request_retries(&mut self, max_request_retries: usize) -> &mut Self {
        self.max_request_retries = max_request_retries;
        self
    }
    pub fn set_proxy_settings(&mut self, proxy_settings: ProxySettings) -> &mut Self {
        self.proxy_settings = Some(proxy_settings);
        self
    }
}

pub struct Crawler {
    request_list: RequestList,
    extract: Vec<Extract>,
    force_cloud: bool,
    debug_log: bool,
    push_data_buffer: Arc<futures::lock::Mutex<Vec<serde_json::Value>>>,
    client: reqwest::Client,
    proxy_client: reqwest::Client, // The reason for 2 clients is that proxy_client is used for websites and client for push_data
    max_concurrency: usize,
    max_request_retries: usize
}

impl Crawler {
    pub fn new(
        request_list: RequestList,
        options: CrawlerOptions
    ) -> Crawler {
        println!("STATUS --- Initializing crawler");
        let client = reqwest::Client::builder().build().unwrap();

        let proxy = get_apify_proxy(&options.proxy_settings);
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
            push_data_buffer: Arc::new(futures::lock::Mutex::new(Vec::with_capacity(options.push_data_size))),
            client,
            proxy_client,
            extract: options.extract,
            force_cloud: options.force_cloud,
            debug_log: options.debug_log,
            max_concurrency: options.max_concurrency,
            max_request_retries: options.max_request_retries
        }
    }

    pub async fn run(self) { 
        let mut task_handles = vec![];

        // This is only to keep copies for later
        let push_data_buffer_2 = self.push_data_buffer.clone();
        let client_2 = self.client.clone();
        let force_cloud = self.force_cloud;

        loop {
            let debug_log = self.debug_log;
            if debug_log {
                // println!("Loop iteration start");
            }
            // Check concurrency
            let (in_progress_count, reclaimed_count) = {
                let locked_state = self.request_list.state.lock().await;
                (locked_state.in_progress.len(), locked_state.reclaimed.len())
            };

            // ****
            // We have to careful here that (in_progress_count, reclaimed_count) can get out of sync here
            // But since the threads can only lower it, it will not go above max_concurrency
            // Thus being correct 
            // But need to observe if new features come
            // ****

            // If there is any reclaimed one, we always pick,
            // reclaimed is subset of in_progress so it should no go above max_concurrency
            if reclaimed_count == 0 && in_progress_count >= self.max_concurrency {
                // println!("Max concurrency {} reached, waiting", self.max_concurrency);
                tokio::time::delay_for(Duration::from_millis(10)).await;
                continue;
            }

            // This req is immutable, only mutable via requests vec
            let req = match self.request_list.fetch_next_request().await {
                None => {
                    // We still can have in_progress
                    let in_progress_count;
                    {
                        let locked_state = self.request_list.state.lock().await;
                        in_progress_count = locked_state.in_progress.len();
                    }
                    if in_progress_count > 0 {
                        // println!("We still have some in-progress, waiting");
                        tokio::time::delay_for(Duration::from_millis(10)).await;
                        continue;
                    } else {
                        break;
                    }
                },
                Some(req) => req
            };

            // println!("Took new request: {:?}", req);

            let extract = self.extract.clone();
            let client = self.client.clone();
            let proxy_client = self.proxy_client.clone();
            let push_data_buffer = self.push_data_buffer.clone();
            let force_cloud = self.force_cloud;
            
            let max_request_retries = self.max_request_retries;

            // Sending the state via Arc to a thread
            let state = Arc::clone(&self.request_list.state);
            let unique_key_to_index = Arc::clone(&self.request_list.unique_key_to_index);
            
            let handle = tokio::task::spawn(async move {
                let extract_data_result = extract_data_from_url(
                    req.clone(),
                    extract,
                    client,
                    proxy_client,
                    push_data_buffer,
                    force_cloud,
                    debug_log,
                ).await;

                match extract_data_result {
                    Ok(()) => {
                        // mark_request_handled inlined here
                        if (debug_log) {
                            println!("SUCCESS: Retry count: {}, URL: {}",
                            req.retry_count, req.url); 
                        }
                        let mut locked_state = state.lock().await;
                        locked_state.in_progress.remove(&req.unique_key);
                    },
                    Err(ref e) if req.retry_count < max_request_retries => {
                        // reclaim_request inlined here
                        println!("ERROR: Reclaiming request! Retry count: {}, URL: {}, error: {}",
                            req.retry_count, req.url, e);
                        let index = *unique_key_to_index.get(&req.unique_key).unwrap();
                        let mut locked_state = state.lock().await;
                        locked_state.requests[index].retry_count += 1;
                        locked_state.reclaimed.insert(req.unique_key);
                    },
                    Err(ref e) => {
                        // mark_request_failed
                        println!("ERROR: Max retries reached, marking failed! Retry count: {}, URL: {}, error: {}",
                            req.retry_count, req.url, e);
                        let mut locked_state = state.lock().await;
                        locked_state.reclaimed.remove(&req.unique_key);
                        locked_state.in_progress.remove(&req.unique_key);
                    }
                } 
                // Log concurrency
                /*
                {
                    let locked_state = state.lock().await;
                    println!("In progress count:{}", locked_state.in_progress.len());
                }
                */
                extract_data_result
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
        push_data(locked_vec.clone(), &client_2, force_cloud).await.unwrap(); 
    } 
}