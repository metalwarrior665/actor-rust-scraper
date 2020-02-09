// #[macro_use] extern crate serde_derive;
use crate::request::Request;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Input {
    pub urls: Vec<Request>,
    pub extract: Vec<Extract>,
    pub proxy_settings: Option<ProxySettings>,
    pub run_async: bool,
    pub force_cloud: bool,
    pub debug_log: bool,
    pub push_data_size: usize
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Extract {
    pub field_name: String,
    pub selector: String,
    pub extract_type: ExtractType
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "content")]
pub enum ExtractType {
    Text,
    Attribute(String)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ProxySettings {
    pub useApifyProxy: bool,
    pub apifyProxyGroups: Option<Vec<String>>
}