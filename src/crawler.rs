use crate::requestlist::RequestList;
use crate::input::{Extract, ProxySettings};
use crate::extract_fn::extract_data_from_url;
use crate::proxy:: {get_apify_proxy};
use crate::storage:: {push_data};
use std::time::Duration;

use async_scoped::TokioScope;

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
    actor: crate::actor::Actor,
    extract: Vec<Extract>,
    force_cloud: bool,
    debug_log: bool,
    push_data_buffer: futures::lock::Mutex<Vec<serde_json::Value>>,
    client: reqwest::Client,
    proxy_client: reqwest::Client, // The reason for 2 clients is that proxy_client is used for websites and client for push_data
    max_concurrency: usize,
    max_request_retries: usize
}

impl Crawler {
    pub fn new(request_list: RequestList, options: CrawlerOptions) -> Crawler {
        println!("STATUS --- Initializing crawler");
        let client = reqwest::Client::builder().build().unwrap();

        let proxy = get_apify_proxy(&options.proxy_settings);
        let proxy_client = match proxy {
            Some(proxy) => {
                reqwest::Client::builder()
                    .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                    .build().unwrap()
            },
            None => {
                client.clone()
            }
        };
        Crawler {
            request_list,
            actor: crate::actor::Actor::new(),
            push_data_buffer: futures::lock::Mutex::new(Vec::with_capacity(options.push_data_size)),
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
        let mut scope = unsafe {
            TokioScope::create()
        };

        loop {
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
                        
            scope.spawn(async {
                let extract_data_result = extract_data_from_url(
                    &req,
                    &self.actor,
                    &self.extract,
                    &self.client,
                    &self.proxy_client,
                    &self.push_data_buffer,
                    self.force_cloud,
                    self.debug_log,
                ).await;

                match extract_data_result {
                    Ok(()) => {
                        // mark_request_handled inlined here
                        if self.debug_log {
                            println!("SUCCESS: Retry count: {}, URL: {}",
                            req.retry_count, req.url); 
                        }
                        let mut locked_state = self.request_list.state.lock().await;
                        locked_state.in_progress.remove(&req.unique_key);
                    },
                    Err(ref e) if req.retry_count < self.max_request_retries => {
                        // reclaim_request inlined here
                        println!("ERROR: Reclaiming request! Retry count: {}, URL: {}, error: {}",
                            req.retry_count, req.url, e);
                        let index = *self.request_list.unique_key_to_index.get(&req.unique_key).unwrap();
                        let mut locked_state = self.request_list.state.lock().await;
                        locked_state.requests[index].retry_count += 1;
                        locked_state.reclaimed.insert(req.unique_key);
                    },
                    Err(ref e) => {
                        // mark_request_failed
                        println!("ERROR: Max retries reached, marking failed! Retry count: {}, URL: {}, error: {}",
                            req.retry_count, req.url, e);
                        let mut locked_state = self.request_list.state.lock().await;
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
                // extract_data_result
            });
        }

        // Here we await all remaining requests 
        let _ = scope.collect();
        
        // After we are done looping, we need to flush the push_data_buffer one last time
        let locked_vec = self.push_data_buffer.lock().await;

        // TODO: We should not need to clone here
        println!("Remaing Push data buffer length before last push:{}", locked_vec.len());
        push_data(locked_vec.clone(), &self.client, self.force_cloud).await.unwrap(); 
    } 
}