use super::{Provider, ProviderConfig, ProviderFactory};
use super::model_instance::ModelInstance;
use crate::models::response::ModelInfo;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry that manages models and their provider associations
pub struct ModelRegistry {
    providers: Arc<RwLock<HashMap<String, Arc<dyn Provider>>>>,
    models: Arc<RwLock<HashMap<String, Arc<ModelInstance>>>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            models: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a provider and its models
    pub fn register_provider(&self, name: String, config: ProviderConfig) -> crate::Result<()> {
        let provider = ProviderFactory::create(config)?;

        // Get all models supported by this provider
        let provider_models = provider.get_models();

        // Create model instances for each model
        {
            let mut models = self.models.write().unwrap();
            for model_config in provider_models {
                let model_instance = Arc::new(ModelInstance::new(
                    model_config.id.clone(),
                    model_config.id.clone(), // For now, use same name for both
                    provider.clone(),
                    model_config.clone(),
                ));

                // Register by both the model ID and any aliases
                models.insert(model_config.id.clone(), model_instance.clone());

                // Also register common aliases
                if model_config.id.starts_with("gpt-") {
                    // Register without provider prefix for OpenAI models
                    models.insert(model_config.id.clone(), model_instance.clone());
                } else if model_config.id.starts_with("claude-") {
                    // Register without provider prefix for Anthropic models
                    models.insert(model_config.id.clone(), model_instance.clone());
                }
            }
        }

        // Store the provider
        {
            let mut providers = self.providers.write().unwrap();
            providers.insert(name, provider);
        }

        Ok(())
    }

    /// Get a model instance by name
    pub fn get_model(&self, name: &str) -> Option<Arc<ModelInstance>> {
        let models = self.models.read().unwrap();
        models.get(name).cloned()
    }

    /// Get all available models
    pub fn get_all_models(&self) -> Vec<ModelInfo> {
        let models = self.models.read().unwrap();
        models
            .values()
            .map(|instance| ModelInfo {
                id: instance.name.clone(),
                object: "model".to_string(),
                owned_by: instance.provider.name().to_string(),
                created: 1686935002, // Placeholder timestamp
            })
            .collect()
    }

    /// Check if a model exists
    pub fn has_model(&self, name: &str) -> bool {
        let models = self.models.read().unwrap();
        models.contains_key(name)
    }

    /// Get all model names
    pub fn list_model_names(&self) -> Vec<String> {
        let models = self.models.read().unwrap();
        models.keys().cloned().collect()
    }
}