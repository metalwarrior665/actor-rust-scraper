use crate::request::Request;

pub struct RequestList {
    pub sources: Vec<Request>
}

impl RequestList {
    pub fn new(sources: Vec<Request>) -> RequestList {
        RequestList {
            sources
        }
    }
}