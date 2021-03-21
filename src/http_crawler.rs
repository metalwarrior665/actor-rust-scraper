/*
use crate::basic_crawler::{BasicCrawler, BasicCrawlerOptions};
use crate::requestlist::RequestList;
use crate::input::{Extract, ProxySettings};
use crate::request::Request;
use crate::basic_crawler::HandleRequestOutput;

use std::future::Future;

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

pub struct HttpCrawler <F, Fut> where
    Fut: Future<Output=HandleRequestOutput> + Send + Sync,
    F: Fn(&Request, &BasicCrawler<F, Fut>) -> Fut + Send + Sync
{
    pub basic_crawler: BasicCrawler <F, Fut>,
}

impl <F, Fut> HttpCrawler <F, Fut> {
    pub fn new(request_list: RequestList, options: BasicCrawlerOptions) -> Self {
        HttpCrawler {
            basic_crawler: BasicCrawler::new(request_list, options),
        }
    }

    pub async fn run(self) {
        self.basic_crawler.run().await;
    }
}
*/