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
    pub fn set_debug_log(&mut self, debug_log: bool) -> &mut Self {
        self.debug_log = debug_log;
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
    // Remove non proxy client after everything is implemented in apify_client
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
        // Scope is needed for non 'static futures so we can pass normal references around
        // instead of putting everything behind Arc

        // SAFETY: With async scopes, it is possible to use mem::forget to leak the future
        // We have control over that we don't do that here so that is fine
        let mut scope = unsafe {
            TokioScope::create()
        };

        loop {
            // Check concurrency
            let (in_progress_count, reclaimed_count) = {
                let locked_state = self.request_list.state.lock();
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
                if self.debug_log {
                    // println!("Max concurrency {} reached, waiting", self.max_concurrency);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
                continue;
            }

            // This req is immutable, only mutable via requests vec
            let req = match self.request_list.fetch_next_request() {
                None => {
                    // We still can have in_progress
                    let in_progress_count;
                    {
                        let locked_state = self.request_list.state.lock();
                        in_progress_count = locked_state.in_progress.len();
                    }
                    if in_progress_count > 0 {
                        // println!("We still have some in-progress, waiting");
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    } else {
                        break;
                    }
                },
                Some(req) => req
            };

            // println!("Took new request: {:?}", req);
                        
            scope.spawn(async {
                if self.debug_log {
                    println!("Spawning extraction for {}", req.url);
                }
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

                if self.debug_log {
                    println!("Extraction finished for {}", req.url);
                }

                // Log concurrency
                if self.debug_log {
                    let locked_state = self.request_list.state.lock();
                    println!("In progress count:{}", locked_state.in_progress.len());
                }

                match extract_data_result {
                    Ok(()) => {
                        if self.debug_log {
                            println!("SUCCESS: Retry count: {}, URL: {}",
                            req.retry_count, req.url); 
                        }
                        self.request_list.mark_request_handled(req);
                    },
                    Err(ref e) if req.retry_count < self.max_request_retries => {
                        println!("ERROR: Reclaiming request! Retry count: {}, URL: {}, error: {}",
                            req.retry_count, req.url, e);
                            self.request_list.reclaim_request(req);
                    },
                    Err(ref e) => {
                        // mark_request_failed
                        println!("ERROR: Max retries reached, marking failed! Retry count: {}, URL: {}, error: {}",
                            req.retry_count, req.url, e);
                        self.request_list.mark_request_failed(req);
                    }
                }          
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