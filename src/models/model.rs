use chrono::{DateTime, Utc};
use conservator::{Domain, Creatable};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Model configuration linked to a provider
#[derive(Debug, Clone, Serialize, Deserialize, Domain)]
#[domain(table = "models")]
pub struct Model {
    #[domain(primary_key)]
    pub id: Uuid,
    pub provider_config_id: Uuid,
    pub model_id: String,
    pub display_name: Option<String>,
    pub input_cost_per_1k: Option<f64>,
    pub output_cost_per_1k: Option<f64>,
    pub max_context: Option<i32>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for creating models
#[derive(Debug, Clone, Creatable)]
pub struct CreateModel {
    pub provider_config_id: Uuid,
    pub model_id: String,
    pub display_name: Option<String>,
    pub input_cost_per_1k: Option<f64>,
    pub output_cost_per_1k: Option<f64>,
    pub max_context: Option<i32>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub is_active: bool,
}

/// Request to create a model
#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct CreateModelRequest {
    pub model_id: String,
    pub display_name: Option<String>,
    pub input_cost_per_1k: Option<f64>,
    pub output_cost_per_1k: Option<f64>,
    pub max_context: Option<i32>,
    pub supports_tools: Option<bool>,
    pub supports_vision: Option<bool>,
}

/// Request to update a model
#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct UpdateModelRequest {
    pub display_name: Option<String>,
    pub input_cost_per_1k: Option<f64>,
    pub output_cost_per_1k: Option<f64>,
    pub max_context: Option<i32>,
    pub supports_tools: Option<bool>,
    pub supports_vision: Option<bool>,
    pub is_active: Option<bool>,
}

/// Model info for API response
#[derive(Debug, Clone, Serialize, Schematic)]
pub struct ModelInfo {
    pub id: Uuid,
    pub provider_config_id: Uuid,
    pub model_id: String,
    pub display_name: Option<String>,
    pub input_cost_per_1k: Option<f64>,
    pub output_cost_per_1k: Option<f64>,
    pub max_context: Option<i32>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Model> for ModelInfo {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            provider_config_id: model.provider_config_id,
            model_id: model.model_id,
            display_name: model.display_name,
            input_cost_per_1k: model.input_cost_per_1k,
            output_cost_per_1k: model.output_cost_per_1k,
            max_context: model.max_context,
            supports_tools: model.supports_tools,
            supports_vision: model.supports_vision,
            is_active: model.is_active,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
