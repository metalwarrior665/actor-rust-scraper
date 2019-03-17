use crate::requestlist::RequestList;
use crate::request::Request;
use crate::input::{Extract, ProxySettings};

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
    /*
    pub fn run(&self, f: fn(&Request, &Vec<Extract>) -> Value) -> Vec<Value> {
        self.request_list.sources.par_iter().map(|req| f(req, &self.extract)).collect()
    } 
    */
    pub fn run(&self, f: fn(&Request, &Vec<Extract>, &Option<ProxySettings>)) {
        self.request_list.sources.par_iter().for_each(|req| f(req, &self.extract, &self.proxy_settings));
    }   
}