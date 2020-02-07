use crate::requestlist::RequestList;
use crate::request::Request;
use crate::input::{Extract, ProxySettings};
use crate::{extract_data_from_url_async};

use async_std::task;
// use async_std::prelude::Future;

use rayon::prelude::*;

pub struct Crawler {
    request_list: RequestList,
    extract: Vec<Extract>,
    proxy_settings: Option<ProxySettings>
}

impl Crawler {
    pub fn new(request_list: RequestList, extract: Vec<Extract>, proxy_settings: Option<ProxySettings>) -> Crawler {
        Crawler {
            request_list,
            extract,
            proxy_settings
        }
    }
    
    pub fn run(&self, f: fn(&Request, &Vec<Extract>, &Option<ProxySettings>)) {
        self.request_list.sources.par_iter().for_each(|req| f(req, &self.extract, &self.proxy_settings));
    }   

    pub async fn run_async(self) { 
        let futures = self.request_list.sources.iter().map(|req| extract_data_from_url_async(req.clone(), self.extract.clone(), self.proxy_settings.clone()));
        // std_async style
        // let tasks = futures.map(|fut| task::spawn(fut));

        // tokio style
        let tasks = futures.map(|fut| tokio::spawn(fut));

        for task in tasks.into_iter() {
            task.await;
        }
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