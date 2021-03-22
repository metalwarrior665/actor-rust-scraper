use crate::requestlist::RequestList;
use crate::request::Request;
use crate::input::{Extract, ExtractType, ProxySettings};
use crate::proxy:: {get_apify_proxy};
use scraper::{Selector, Html};
use serde_json::{Value};
use crate::storage::{ push_data, request_text};
use std::collections::HashMap;
use std::time::{Instant};

use std::time::Duration;
use std::future::Future;

use async_scoped::TokioScope;

pub type HandleRequestOutput = Result<(), Box<dyn std::error::Error + Send + Sync>>;

pub struct BasicCrawlerOptions {
    extract: Vec<Extract>,
    proxy_settings: Option<ProxySettings>,
    force_cloud: bool,
    debug_log: bool,
    push_data_size: usize,
    max_concurrency: usize,
    max_request_retries: usize,
}

impl Default for BasicCrawlerOptions {
    fn default() -> Self {
        BasicCrawlerOptions {
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
impl BasicCrawlerOptions {
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

pub struct CrawlingContext<'a> {
    // req: &'b Request,
    actor: &'a crate::actor::Actor,
    extract: &'a Vec<Extract>,
    force_cloud:&'a bool,
    debug_log: &'a bool,
    push_data_buffer: &'a futures::lock::Mutex<Vec<serde_json::Value>>,
    // Remove non proxy client after everything is implemented in apify_client
    client: &'a reqwest::Client,
    proxy_client: &'a reqwest::Client, // The reason for 2 clients is that proxy_client is used for websites and client for push_data
    max_concurrency: &'a usize,
    max_request_retries: &'a usize,
}

pub trait Wrapped<'a> {
    type Fut: 'a + Future<Output = HandleRequestOutput> + Send + Sync;
    fn call(
        &self,
        req: &'a Request,
        context: CrawlingContext,
    ) -> Self::Fut;
}

impl<'a, Fun, LoadFut> Wrapped<'a> for Fun
where
    Fun: Send + Sync + Fn(&'a Request,  CrawlingContext) -> LoadFut,
    LoadFut: Future<Output = HandleRequestOutput> + 'a + Send + Sync,
{
    type Fut = LoadFut;
    fn call(
        &self,
        req: &'a Request,
        context: CrawlingContext,
    ) -> Self::Fut {
        (self)(req, context)//context)
    }
}

pub struct BasicCrawler<Fun>
where
for<'r> Fun:  Wrapped<'r> + Send + Sync,
/*
    Fun: Fn(& Request, CrawlingContext) -> Fut,
    Fut: Future<Output=HandleRequestOutput> + Send + Sync,
*/
{
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
    max_request_retries: usize,
    handle_request_function: Fun,
}

impl <Fun> BasicCrawler <Fun> 
where 
for<'r> Fun:  Wrapped<'r> + Send + Sync,
    {
    pub fn new(request_list: RequestList, options: BasicCrawlerOptions, handle_request_function: Fun)
    -> BasicCrawler <Fun>
    where
    
     {
    
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
        BasicCrawler {
            request_list,
            actor: crate::actor::Actor::new(),
            push_data_buffer: futures::lock::Mutex::new(Vec::with_capacity(options.push_data_size)),
            client,
            proxy_client,
            extract: options.extract,
            force_cloud: options.force_cloud,
            debug_log: options.debug_log,
            max_concurrency: options.max_concurrency,
            max_request_retries: options.max_request_retries,
            handle_request_function,
        }
    }

    pub async fn log(&self) {
        println!("logging {}", self.max_concurrency);
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

            let cloned_req = req.clone();

            let crawlingContext = CrawlingContext {
                actor: &self.actor,
                extract: &self.extract,
                force_cloud: &self.force_cloud,
                debug_log: &self.debug_log,
                push_data_buffer: &self.push_data_buffer,
                client: &self.client,
                proxy_client: &self.proxy_client,
                max_concurrency: &self.max_concurrency,
                max_request_retries: &self.max_request_retries,
            };

            // println!("Took new request: {:?}", req);
                        
            scope.spawn(async {
                if self.debug_log {
                    println!("Spawning extraction for {}", req.url);
                }

                let extract_data_result = self.handle_request_function.call(&req, crawlingContext).await;

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
        push_data(locked_vec.clone(), &self.client, /*self.force_cloud*/).await.unwrap(); 
    } 

    /*
    pub async fn handle_request_function_default <'a>(req: &Request, basic_crawler: CrawlingContext<'a>)
        -> HandleRequestOutput {
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
    
        let now = Instant::now();
        let html = request_text(&url, &basic_crawler.proxy_client).await?;
        let request_time = now.elapsed().as_millis();
    
        // println!("Reqwest retuned");
        let mut map: HashMap<String, Value> = HashMap::new();
        let parse_time;
        let extract_time;
        {
            let now = Instant::now();
            let dom = Html::parse_document(&html);
            parse_time = now.elapsed().as_millis();
        
            let now = Instant::now();
            for extr in basic_crawler.extract.iter() {
                let selector_bind = &extr.selector.clone();
                // TODO: Implement std::Error
                let selector = Selector::parse(selector_bind).unwrap();
                let element = dom.select(&selector).next();
                let val = match element {
                    Some(element) => {
                        // println!("matched element");
                        let extracted_value = match &extr.extract_type {
                            ExtractType::Text => element.text().fold(String::from(""), |acc, s| acc + s).trim().to_owned(),
                            // TODO: Implement std::Error
                            ExtractType::Attribute(at) => element.value().attr(&at).unwrap().to_owned()
                        };
                        Some(extracted_value)
                    },
                    None => None
                };
                let insert_value = match val {
                    Some(string) => Value::String(string),
                    None => Value::Null,
                };
                map.insert(extr.field_name.clone(), insert_value);
            }
            extract_time = now.elapsed().as_millis();
        }
        
        let map_size = map.len();
    
        let value = serde_json::to_value(map)?;
    
        let now = Instant::now();
    
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
        
        let push_time = now.elapsed().as_millis();
        
        if basic_crawler.debug_log {
            println!(
                "SUCCESS({}/{}) - {} - timings (in ms) - request: {}, parse: {}, extract: {}, push: {}",
                map_size,
                basic_crawler.extract.len(),
                url,
                request_time,
                parse_time,
                extract_time,
                push_time
            );
        }
        Ok(())
    }
    */
}
