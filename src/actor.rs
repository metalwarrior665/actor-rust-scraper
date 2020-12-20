use std::sync::Arc;

use apify_client::client::ApifyClient;

use crate::utils::is_on_apify;
use crate::dataset::DatasetHandle;

#[derive(Clone)]
pub struct Actor {
    // Rc might bite us later, let's see
    pub client: Arc<ApifyClient>,
    // TODO: Probably wrap in mutex
    pub dataset_cache: std::collections::HashMap<String, crate::dataset::DatasetHandle>
}

impl Actor {
    /// Creates new Actor handler and initiazes client
    pub fn new () -> Actor {
        let maybe_token = std::env::var("APIFY_TOKEN");
        Actor {
            client: Arc::new(ApifyClient::new(maybe_token.ok())),
            dataset_cache: std::collections::HashMap::new(),
        }
    }

    pub async fn open_dataset(&mut self, dataset_name_or_id: Option<&str>, force_cloud: bool) -> DatasetHandle {
        if force_cloud && !self.client.optional_token.is_some() {
            panic!("Cannot open cloud dataset without a token! Add APIFY_TOKEN env var!")
        }

        // TODO: Fix this remove/insert to clone
        if let Some(dataset) = self.dataset_cache.remove(dataset_name_or_id.unwrap_or("default")) {
            self.dataset_cache.insert(dataset.id.clone(), dataset.clone());
            return dataset;
        }

        let is_default = dataset_name_or_id.is_none();

        println!("is_default {}", is_default);

        let dataset;
        if is_on_apify() || force_cloud {
            if is_default {
                dataset = DatasetHandle {
                    id: std::env::var("APIFY_DEFAULT_DATASET_ID").unwrap(),
                    name: "default".to_string(),
                    is_on_cloud: true,
                    client: self.client.clone(),
                }
            } else {
                let cloud_dataset = self.client.create_dataset(dataset_name_or_id.unwrap()).send().await.unwrap();
                dataset = DatasetHandle {
                    id: cloud_dataset.id,
                    name: cloud_dataset.name.unwrap(),
                    is_on_cloud: true,
                    client: self.client.clone(),
                }
            }
        } else {
            let name = dataset_name_or_id.unwrap_or("default");
            // Will return error if the dir already exists
            // TODO: Handle properly
            std::fs::create_dir(format!("apify_storage/datasets/{}", name));
            dataset = DatasetHandle {
                id: name.to_string(),
                name: name.to_string(),
                is_on_cloud: false,
                client: Arc::clone(&self.client),
            }
        }
        self.dataset_cache.insert(dataset.id.clone(), dataset.clone());
        dataset
    }

    /// Pushes data to default dataset (initializes default DatasetHandle)
    pub async fn push_data<T: serde::Serialize> (&mut self, data: &[T]) 
    -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let  dataset_handle = self.open_dataset(None, false).await;
        dataset_handle.push_data(data).await?;
        Ok(())
    }
}