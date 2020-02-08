use crate::requestlist::RequestList;
use crate::request::Request;
use crate::input::{Extract, ProxySettings};
use crate::{extract_data_from_url_async,extract_data_from_url};
use crate::proxy:: {get_apify_proxy};

use async_std::task;
// use async_std::prelude::Future;

use rayon::prelude::*;

pub struct Crawler {
    request_list: RequestList,
    extract: Vec<Extract>,
    client: reqwest::Client,
    blocking_client: reqwest::blocking::Client
}

impl Crawler {
    pub fn new(request_list: RequestList, extract: Vec<Extract>, proxy_settings: Option<ProxySettings>) -> Crawler {
        let proxy = get_apify_proxy(&proxy_settings);
        let (client, blocking_client) = match proxy {
            Some(proxy) => {
                let client = reqwest::Client::builder()
                    .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                    .build().unwrap();

                let blocking_client = reqwest::blocking::Client::builder()
                    .proxy(reqwest::Proxy::all(&proxy.base_url).unwrap().basic_auth(&proxy.username, &proxy.password))
                    .build().unwrap();
                (client, blocking_client)
            },
            None => {
                let client = reqwest::Client::builder() .build().unwrap();
                let blocking_client = reqwest::blocking::Client::builder().build().unwrap();
                (client, blocking_client)
            }
        };
        Crawler {
            request_list,
            extract,
            client,
            blocking_client
        }
    }
    
    pub fn run(&self) {
        self.request_list.sources.par_iter().for_each(|req| extract_data_from_url(req, &self.extract, &self.blocking_client));
    }   

    pub async fn run_async(self) { 
        let futures = self.request_list.sources.iter().map(|req| extract_data_from_url_async(req.clone(), self.extract.clone(), self.client.clone()));
        // std_async style
        // let tasks = futures.map(|fut| task::spawn(fut));

        let fut = futures::future::join_all(futures.into_iter()).await;

        // tokio::spawn(fut).await;
        // tokio style
        // let tasks = futures.map(|fut| tokio::spawn(fut));

        // futures::future::join_all(tasks.into_iter()).await;
    } 

    // Was naively trying to pass async function in
    /*
    pub async fn run_async<F>(&self, f: impl Fn(&Request, &Vec<Extract>, &Option<ProxySettings>) -> F)
    where F: Future {
        let tasks = self.request_list.sources.iter().map(|req| task::spawn(async {
            println!("Spawning task");
            f(req, &self.extract, &self.proxy_settings).await;
        }));
    } 
    */
}