use super::{Provider, ProviderConfig, ProviderFactory, ModelConfig};
use super::model_instance::ModelInstance;
use crate::models::response::ModelInfo;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Extended model info that includes provider access control
#[derive(Clone)]
pub struct RegisteredModel {
    pub instance: Arc<ModelInstance>,
    pub provider_config_id: Uuid,
    pub allowed_groups: Vec<String>,
}

/// Registry that manages models and their provider associations
pub struct ModelRegistry {
    providers: Arc<RwLock<HashMap<String, (Arc<dyn Provider>, Uuid, Vec<String>)>>>,
    models: Arc<RwLock<HashMap<String, RegisteredModel>>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            models: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a provider (without auto-registering its models)
    pub fn register_provider(
        &self,
        name: String,
        config: ProviderConfig,
        provider_config_id: Uuid,
        allowed_groups: Vec<String>,
    ) -> crate::Result<()> {
        let provider = ProviderFactory::create(config)?;

        // Store the provider with its config ID and allowed groups
        let mut providers = self.providers.write().unwrap();
        providers.insert(name, (provider, provider_config_id, allowed_groups));

        Ok(())
    }

    /// Register a model with explicit configuration
    pub fn register_model(
        &self,
        provider_name: &str,
        model_config: ModelConfig,
        provider_config_id: Uuid,
        allowed_groups: Vec<String>,
    ) -> crate::Result<()> {
        // Get the provider
        let providers = self.providers.read().unwrap();
        let (provider, _, _) = providers.get(provider_name).ok_or_else(|| {
            crate::MantiError::Provider(format!("Provider '{}' not found", provider_name))
        })?;
        let provider = provider.clone();
        drop(providers);

        // Create the model instance
        let model_instance = Arc::new(ModelInstance::new(
            model_config.id.clone(),
            model_config.id.clone(),
            provider,
            model_config.clone(),
        ));

        // Register the model
        let mut models = self.models.write().unwrap();
        models.insert(
            model_config.id.clone(),
            RegisteredModel {
                instance: model_instance,
                provider_config_id,
                allowed_groups,
            },
        );

        Ok(())
    }

    /// Get a model instance by name
    pub fn get_model(&self, name: &str) -> Option<Arc<ModelInstance>> {
        let models = self.models.read().unwrap();
        models.get(name).map(|m| m.instance.clone())
    }

    /// Get a model with its access control info
    pub fn get_model_with_access(&self, name: &str) -> Option<RegisteredModel> {
        let models = self.models.read().unwrap();
        models.get(name).cloned()
    }

    /// Check if user has access to a model based on their groups
    pub fn user_has_access(&self, model_name: &str, user_groups: &[String]) -> bool {
        let models = self.models.read().unwrap();
        if let Some(model) = models.get(model_name) {
            // Empty allowed_groups means public access
            model.allowed_groups.is_empty()
                || model.allowed_groups.iter().any(|g| user_groups.contains(g))
        } else {
            false
        }
    }

    /// Get all models accessible by user groups
    pub fn get_accessible_models(&self, user_groups: &[String]) -> Vec<ModelInfo> {
        let models = self.models.read().unwrap();
        models
            .iter()
            .filter(|(_, model)| {
                model.allowed_groups.is_empty()
                    || model.allowed_groups.iter().any(|g| user_groups.contains(g))
            })
            .map(|(_, model)| ModelInfo {
                id: model.instance.name.clone(),
                object: "model".to_string(),
                owned_by: model.instance.provider.name().to_string(),
                created: 1686935002, // Placeholder timestamp
            })
            .collect()
    }

    /// Get all available models (for admin or public listing)
    pub fn get_all_models(&self) -> Vec<ModelInfo> {
        let models = self.models.read().unwrap();
        models
            .values()
            .map(|model| ModelInfo {
                id: model.instance.name.clone(),
                object: "model".to_string(),
                owned_by: model.instance.provider.name().to_string(),
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

    /// Clear all providers and models
    pub fn clear(&self) {
        {
            let mut providers = self.providers.write().unwrap();
            providers.clear();
        }
        {
            let mut models = self.models.write().unwrap();
            models.clear();
        }
    }
}
