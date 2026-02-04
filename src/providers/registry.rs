use super::{Provider, ProviderConfig, ProviderFactory, ModelConfig};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct ProviderRegistry {
    providers: Arc<RwLock<HashMap<String, Arc<dyn Provider>>>>,
    model_map: Arc<RwLock<HashMap<String, String>>>, // model_id -> provider_name
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            model_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a provider
    pub fn register(&self, name: String, config: ProviderConfig) -> crate::Result<()> {
        let provider = ProviderFactory::create(config)?;

        // Update model map
        {
            let mut model_map = self.model_map.write().unwrap();
            for model in provider.get_models() {
                model_map.insert(model.id.clone(), name.clone());
            }
        }

        // Add provider
        {
            let mut providers = self.providers.write().unwrap();
            providers.insert(name, provider);
        }

        Ok(())
    }

    /// Get provider by name
    pub fn get_provider(&self, name: &str) -> Option<Arc<dyn Provider>> {
        let providers = self.providers.read().unwrap();
        providers.get(name).cloned()
    }

    /// Get provider for a specific model
    pub fn get_provider_for_model(&self, model: &str) -> Option<Arc<dyn Provider>> {
        let model_map = self.model_map.read().unwrap();
        if let Some(provider_name) = model_map.get(model) {
            self.get_provider(provider_name)
        } else {
            // Try to find by checking each provider
            let providers = self.providers.read().unwrap();
            for (_name, provider) in providers.iter() {
                if provider.supports_model(model) {
                    return Some(provider.clone());
                }
            }
            None
        }
    }

    /// List all registered providers
    pub fn list_providers(&self) -> Vec<String> {
        let providers = self.providers.read().unwrap();
        providers.keys().cloned().collect()
    }

    /// Get all available models
    pub fn get_all_models(&self) -> Vec<ModelConfig> {
        let providers = self.providers.read().unwrap();
        let mut models = Vec::new();
        for provider in providers.values() {
            models.extend(provider.get_models());
        }
        models
    }

    /// Remove a provider
    pub fn unregister(&self, name: &str) {
        let mut providers = self.providers.write().unwrap();
        if providers.remove(name).is_some() {
            // Clean up model map
            let mut model_map = self.model_map.write().unwrap();
            let models_to_remove: Vec<String> = model_map
                .iter()
                .filter(|(_, provider_name)| *provider_name == name)
                .map(|(model, _)| model.clone())
                .collect();

            for model in models_to_remove {
                model_map.remove(&model);
            }
        }
    }
}