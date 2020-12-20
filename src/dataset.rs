// use crate::actor::Actor;
use apify_client::client::{ApifyClient, IdOrName};
use rand::Rng;
use std::sync::Arc;
// Handle for both local and cloud datasets. 
// There are some fields that are useless but this is simpler now.
#[derive(Clone)]
pub struct DatasetHandle {
    pub id: String,
    pub name: String,
    pub is_on_cloud: bool,
    pub client: Arc<ApifyClient>, // A reference to the actor's client
}

impl DatasetHandle {
    pub async fn push_data<T: serde::Serialize> (&self, data: &[T]) 
        -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_on_cloud {
            self.client.put_items(&IdOrName::Id(self.id.clone()), data).send().await;
        } else {
            for val in data.iter() {
                let json = serde_json::to_string(&val)?;
                let mut rng = rand::thread_rng();
                // TODO: Implement increment instead of random
                let path = format!("apify_storage/datasets/{}/{}.json", self.name, rng.gen::<i32>());
                std::fs::write(path, json)?;
            } 
        }
        Ok(())
    }
}