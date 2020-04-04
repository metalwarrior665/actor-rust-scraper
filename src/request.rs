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
    pub fn new(url: String) -> Request {
        Request {
            url: url.clone(),
            unique_key: url,
            retry_count: 0
        }
    }
}
