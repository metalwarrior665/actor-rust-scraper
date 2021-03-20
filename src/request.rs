use crate::input::RequestOptions;

// Immutable part of Request
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Request {
    // id: String,
    pub url: String,
    pub unique_key: String,
    // method: String,
    // payload: String,
    // retry: bool,
    pub retry_count: usize,
    // error_messages: Vec<String>,
    // headers: HashMap<String, String>,
    // user_data: HashMap<String, String>,
    // handled_at: String
}

impl Request {
    pub fn new(req: RequestOptions) -> Request {
        let unique_key = match req.unique_key {
            Some(key) => key,
            None => req.url.clone()
        };
        Request {
            url: req.url,
            unique_key,
            retry_count: 0
        }
    }
}
