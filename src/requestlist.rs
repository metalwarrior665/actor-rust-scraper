use crate::request::Request;
use futures::lock::{Mutex};
use std::sync::{ Arc };
use std::collections::HashSet;

pub struct RequestListState {
    pub next_index: usize,
    pub next_uniqueKey: String,
    pub in_progress: HashSet<String>
}

pub struct RequestList {
    pub sources: Vec<Request>,
    pub state: Arc<Mutex<RequestListState>>
}

impl RequestList {
    pub fn new(sources: Vec<Request>) -> RequestList {
        let fresh_state = RequestListState {
            next_index: 0,
            next_uniqueKey: sources[0].url.clone(),
            in_progress: HashSet::new()
        };
        RequestList {
            sources,
            state: Arc::new(Mutex::new(fresh_state))
        }
    }

    pub async fn fetch_next_request(&self) -> Option<Request> {
        let mut locked_state = self.state.lock().await;
        if locked_state.next_index >= self.sources.len() {
            return None;
        }
        let next_req = self.sources[locked_state.next_index].clone();
        locked_state.in_progress.insert(next_req.url.clone());
        locked_state.next_index += 1;
        return Some(next_req);
    }

    // I use this fn inlined for now
    pub async fn mark_request_handled(&self, req: Request) {
        let mut locked_state = self.state.lock().await;
        locked_state.in_progress.remove(&req.url);
    }
}