use crate::request::Request;
use futures::lock::{Mutex};
use std::sync::{ Arc };
use std::collections::{HashSet, HashMap};

pub struct RequestListState {
    pub next_index: usize,
    // pub next_uniqueKey: String, --- In Apify implementation, not needed now
    pub in_progress: HashSet<String>,
    pub reclaimed: HashSet<String>,
    pub requests: Vec<Request>,
}

pub struct RequestList {
    pub state: Arc<Mutex<RequestListState>>,
    pub unique_key_to_index: Arc<HashMap<String, usize>>
}

// The implementation is very simplified verison of - https://github.com/apifytech/apify-js/blob/master/src/request_list.js 
impl RequestList {
    pub fn new(sources: Vec<Request>) -> RequestList {
        let mut unique_key_to_index = HashMap::with_capacity(sources.len());
        for (i, req) in sources.iter().enumerate() {
            unique_key_to_index.insert(req.unique_key.clone(), i);
        }
        let fresh_state = RequestListState {
            next_index: 0,
            // next_uniqueKey: sources[0].url.clone(),
            in_progress: HashSet::new(),
            reclaimed: HashSet::new(),
            requests: sources,
        };
        RequestList {
            state: Arc::new(Mutex::new(fresh_state)),
            unique_key_to_index: Arc::new(unique_key_to_index)
        }
    }

    pub async fn fetch_next_request(&self) -> Option<Request> {
        
        let mut locked_state = self.state.lock().await;  
        // println!("Fetch start, reclaimed length: {}", locked_state.reclaimed.len());
        // First check reclaimed if empty then fetch next

        // We have to do it ugly like this because there is no .take(1) on HashSetS
        let mut maybe_reclaimed_key = None;
        for unique_key in locked_state.reclaimed.iter().take(1) {
            maybe_reclaimed_key = Some(unique_key.clone());
        }

        // If we found some reclaimed, we return that
        if let Some(unique_key) = maybe_reclaimed_key {
            // println!("FETCHING: picking reclaimed request: {}", unique_key);
            // Need to remove it from the reclaimed set first
            locked_state.reclaimed.remove(&unique_key);
            // This should not fail
            let index = self.unique_key_to_index.get(&unique_key).unwrap();
            let req = locked_state.requests[*index].clone();
            return Some(req);
        }
  
        if locked_state.next_index >= locked_state.requests.len() {
            return None;
        }
        let next_req = locked_state.requests[locked_state.next_index].clone();
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