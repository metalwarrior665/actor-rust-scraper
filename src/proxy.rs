use crate::input::ProxySettings;
use std::env;

#[derive(Debug, Clone)]
pub struct Proxy {
    pub base_url: String,
    pub username: String,
    pub password: String
}

impl Proxy {
    fn new(username: String, password: String) -> Proxy {
        let mut base_url = "http://@proxy.apify.com:8000".to_owned();
        if env::var("APIFY_PROXY_HOSTNAME").is_ok() && env::var("APIFY_PROXY_PORT").is_ok() {
            let hostname = env::var("APIFY_PROXY_HOSTNAME").unwrap();
            let port = env::var("APIFY_PROXY_PORT").unwrap();
            base_url = format!("http://@{}:{}", hostname, port);
        }

        Proxy {
            base_url,
            username,
            password
        }
    }
}

pub fn get_apify_proxy (settings: &Option<ProxySettings>) -> Option<Proxy> {
    // println!("proxy settings {:?}", settings);
    let use_apify_proxy = match settings {
        None => false,
        Some(settings) => settings.useApifyProxy
    };
    match use_apify_proxy {
        false => None,
        true => Some(construct_proxy(settings.clone().unwrap().apifyProxyGroups))
    }
}

fn construct_proxy (groups: Option<Vec<String>>) -> Proxy {
    let password = match env::var("APIFY_PROXY_PASSWORD") {
        Ok(pass) => pass,
        Err(_) => panic!("Missing APIFY_PROXY_PASSWORD environment variable. This is required to use Apify proxy!")
    };
    let username = match groups {
        None => "auto".to_owned(),
        Some(groups) => {
            let joined = groups.join("+");
            format!("groups-{}", joined)
        }
    };
    Proxy::new(username, password)
}