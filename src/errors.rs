// Creating my dummy error type before I solve how to send Error over threads
use std::fmt;
use std::error::Error;

#[derive(Debug)]
pub struct CrawlerError {
    message: String
}

impl CrawlerError {
    pub fn new(err: String) -> CrawlerError {
        CrawlerError {
            message: err
        }
    }
}

impl fmt::Display for CrawlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for CrawlerError {}