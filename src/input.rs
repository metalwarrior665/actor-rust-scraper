// #[macro_use] extern crate serde_derive;
use crate::basic_crawler::BasicCrawlerOptions;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestOptions {
    // id: String,
    pub url: String,
    pub unique_key: Option<String>,
    // method: String,
    // payload: String,
    // retry: bool,
    // error_messages: Vec<String>,
    // headers: HashMap<String, String>,
    // user_data: HashMap<String, String>,
    // handled_at: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Input {
    pub urls: Vec<RequestOptions>,
    pub extract: Option<Vec<Extract>>,
    pub proxy_settings: Option<ProxySettings>,
    pub force_cloud: Option<bool>,
    pub debug_log: Option<bool>,
    pub push_data_size: Option<usize>,
    pub max_concurrency: Option<usize>,
    pub max_request_retries: Option<usize>,
    pub use_http3: Option<bool>,
}

impl Input {
    pub fn to_options (self) -> BasicCrawlerOptions {
        let mut options: BasicCrawlerOptions = Default::default();
        if let Some(extract) = self.extract {
            options.set_extract(extract);
        }
        if let Some(proxy_settings) = self.proxy_settings {
            options.set_proxy_settings(proxy_settings);
        }
        if let Some(push_data_size) = self.push_data_size {
            options.set_push_data_size(push_data_size);
        }
        if let Some(max_concurrency) = self.max_concurrency {
            options.set_max_concurrency(max_concurrency);
        }
        if let Some(max_request_retries) = self.max_request_retries {
            options.set_max_request_retries(max_request_retries);
        }
        if let Some(debug_log) = self.debug_log {
            options.set_debug_log(debug_log);
        }
        options
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Extract {
    pub field_name: String,
    pub selector: String,
    pub extract_type: ExtractType
}

impl Default for Extract {
    fn default() -> Self {
        Extract {
            field_name: "description".to_owned(),
            selector: "meta[name=\"description\"]".to_owned(),
            extract_type: ExtractType::Attribute("content".to_owned())
        }
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "content")]
pub enum ExtractType {
    Text,
    Attribute(String)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(non_snake_case)]
pub struct ProxySettings {
    pub useApifyProxy: bool,
    pub apifyProxyGroups: Option<Vec<String>>
}

impl Default for ProxySettings {
    fn default() -> Self {
        ProxySettings {
            useApifyProxy: true,
            apifyProxyGroups: None
        }
    }
}