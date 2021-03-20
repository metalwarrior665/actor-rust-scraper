use crate::basic_crawler::{BasicCrawler, BasicCrawlerOptions, HandleRequestOutput};
use crate::requestlist::RequestList;
use crate::input::{Extract, ProxySettings, ExtractType};
use crate::request::Request;
use crate::storage::{ push_data, request_text};
use std::future::Future;
use std::time::Instant;

use scraper::{Selector, Html};
use serde_json::{Value};


pub struct HtppCrawlerOptions {
    extract: Vec<Extract>,
    proxy_settings: Option<ProxySettings>,
    force_cloud: bool,
    debug_log: bool,
    push_data_size: usize,
    max_concurrency: usize,
    max_request_retries: usize
}

impl Default for HtppCrawlerOptions {
    fn default() -> Self {
        HtppCrawlerOptions {
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
impl HtppCrawlerOptions {
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

pub struct HttpCrawler {
    pub basic_crawler: BasicCrawler,
}

pub async fn http_handle_request_function(
    req: &Request,
    basic_crawler: &BasicCrawler,
    handle_page_function: impl Fn(&Html, &Vec<Extract>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>)
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let url = &req.url;
        if basic_crawler.debug_log {
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
    
        let html = request_text(&url, &basic_crawler.proxy_client).await?;
        
        let dom = Html::parse_document(&html);

        let value = handle_page_function(&dom, &basic_crawler.extract)?;

        {
            // We could theoretically use non-async mutex here
            // but it would require to copy the data (so we don't hold across push_data)
            // Note sure what is better
            let mut locked_vec = basic_crawler.push_data_buffer.lock().await;
            locked_vec.push(value);
            let vec_len = locked_vec.len();
            if basic_crawler.debug_log {
                println!("Push data buffer length:{}", vec_len);
            }
            if vec_len >= locked_vec.capacity() { // Capacity should never grow over original push_data_size
                println!("Flushing data buffer --- length: {}", locked_vec.len());
                if basic_crawler.force_cloud {
                    basic_crawler.actor.client.put_items(&apify_client::client::IdOrName::Id("qdFyJscHebXJqilLu".to_string()), &*locked_vec)
                        .send().await?;
                } else {
                    push_data(locked_vec.clone(), &basic_crawler.client, basic_crawler.force_cloud).await?; 
                }
                // TODO: Fix actor implementation
                // actor.push_data(&locked_vec).await?;
                locked_vec.truncate(0);
                println!("Flushed data buffer --- length: {}", locked_vec.len());
            }
        }
        
        Ok(())
}

impl HttpCrawler {
    pub fn new(request_list: RequestList, options: BasicCrawlerOptions) -> Self {
        HttpCrawler {
            basic_crawler: BasicCrawler::new(request_list, options),
        }
    }

    pub async fn run<F>(
        self,
        handle_page_function: impl Fn(&Html, &Vec<Extract>) -> Result<Value, Box<dyn std::error::Error + Send + Sync>>
    )
    where
    F: Future<Output=HandleRequestOutput> + Send + Sync { 
        self.basic_crawler.run(
            
            |req, crawler| {
                let mut scope = unsafe {
                    async_scoped::TokioScope::create()
                };
                scope.spawn(async {
                    http_handle_request_function(req, crawler, handle_page_function).await
                });
                let collected = scope.collect();
                // FIX: We need to pass the Result back, not this dummy return
                async {
                    Ok(())
                }
            }
            
        ).await;   
    }
}