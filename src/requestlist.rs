use std::collections::{HashSet, HashMap};

use crate::request::Request;
use crate::input::RequestOptions;

pub struct RequestListState {
    pub next_index: usize,
    // pub next_uniqueKey: String, --- In Apify implementation, not needed now
    pub in_progress: HashSet<String>,
    pub reclaimed: HashSet<String>,
    pub requests: Vec<Request>,
}

pub struct RequestList {
    // WARNING: We must never hold this lock across await (it will freeze)
    pub state: parking_lot::Mutex<RequestListState>,
    pub unique_key_to_index: HashMap<String, usize>,
    debug_log: bool,
}

// The implementation is very simplified verison of - https://github.com/apifytech/apify-js/blob/master/src/request_list.js 
impl RequestList {
    pub fn new(sources: Vec<RequestOptions>, debug_log: bool) -> RequestList {
        let mut requests: Vec<Request> = vec![];
        let mut unique_key_to_index = HashMap::with_capacity(sources.len());
        for (i, source_req) in sources.into_iter().enumerate() {
            let new_req = Request::new(source_req);
            if !unique_key_to_index.contains_key(&new_req.unique_key) {
                unique_key_to_index.insert(new_req.unique_key.clone(), i);
                requests.push(new_req);
            }
            
        }
        let fresh_state = RequestListState {
            next_index: 0,
            // next_uniqueKey: sources[0].url.clone(),
            in_progress: HashSet::new(),
            reclaimed: HashSet::new(),
            requests,
        };
        RequestList {
            state: parking_lot::Mutex::new(fresh_state),
            unique_key_to_index,
            debug_log,
        }
    }

    pub fn fetch_next_request(&self) -> Option<Request> {
        let mut locked_state = self.state.lock();  
        // println!("Fetch start, reclaimed length: {}", locked_state.reclaimed.len());
        // First check reclaimed if empty then fetch next

        // We have to do it ugly like this because there is no .take(1) on HashSetS
        let mut maybe_reclaimed_key = None;
        for unique_key in locked_state.reclaimed.iter().take(1) {
            maybe_reclaimed_key = Some(unique_key.clone());
        }

        // let maybe_reclaimed_key = locked_state.reclaimed.iter().next();

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
        Some(next_req)
    }

    pub fn mark_request_handled(&self, req: Request) {
        let mut locked_state = self.state.lock();
        locked_state.in_progress.remove(&req.unique_key);
    }

    pub fn reclaim_request(&self, req: Request) {        
        let index = *self.unique_key_to_index.get(&req.unique_key).unwrap();
        let mut locked_state = self.state.lock();
        locked_state.requests[index].retry_count += 1;
        locked_state.reclaimed.insert(req.unique_key);
    }

    pub fn mark_request_failed(&self, req: Request) {    
        let mut locked_state = self.state.lock();
        locked_state.reclaimed.remove(&req.unique_key);
        locked_state.in_progress.remove(&req.unique_key);
    }
}