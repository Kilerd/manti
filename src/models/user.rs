use chrono::{DateTime, Utc};
use conservator::{Domain, Creatable, Selectable};
use gotcha::Schematic;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User model for authentication and authorization
#[derive(Debug, Clone, Serialize, Deserialize, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
    pub user_groups: Vec<String>,
}

/// DTO for creating new users
#[derive(Debug, Clone, Creatable)]
pub struct CreateUser {
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub is_active: bool,
    pub is_admin: bool,
    pub user_groups: Vec<String>,
}

impl CreateUser {
    /// Create a new user DTO with hashed password
    pub fn new(email: String, username: String, password: &str, is_admin: bool) -> crate::Result<Self> {
        let password_hash = hash_password(password)?;

        Ok(Self {
            email,
            username,
            password_hash,
            is_active: true,
            is_admin,
            user_groups: vec!["default".to_string()],
        })
    }
}

impl User {
    /// Create a new user with hashed password (backward compatibility)
    pub fn new(email: String, username: String, password: &str) -> crate::Result<Self> {
        let password_hash = hash_password(password)?;

        Ok(Self {
            id: Uuid::new_v4(),
            email,
            username,
            password_hash,
            is_active: true,
            is_admin: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
            user_groups: vec!["default".to_string()],
        })
    }

    /// Verify password against stored hash
    pub fn verify_password(&self, password: &str) -> bool {
        verify_password(password, &self.password_hash).unwrap_or(false)
    }

    /// Update last login timestamp
    pub fn update_last_login(&mut self) {
        self.last_login = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Check if user belongs to a specific group
    pub fn has_group(&self, group: &str) -> bool {
        self.user_groups.contains(&group.to_string())
    }

    /// Check if user has access based on allowed groups
    /// Returns true if allowed_groups is empty (public) or has intersection with user's groups
    pub fn has_access(&self, allowed_groups: &[String]) -> bool {
        allowed_groups.is_empty() || allowed_groups.iter().any(|g| self.user_groups.contains(g))
    }
}

/// Hash a password using Argon2
pub fn hash_password(password: &str) -> crate::Result<String> {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| crate::MantiError::Auth(format!("Failed to hash password: {}", e)))
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> crate::Result<bool> {
    use argon2::{
        password_hash::{PasswordHash, PasswordVerifier},
        Argon2,
    };

    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| crate::MantiError::Auth(format!("Invalid password hash: {}", e)))?;

    let argon2 = Argon2::default();
    Ok(argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok())
}

/// User creation request
#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct CreateUserRequest {
    pub email: String,
    pub username: String,
    pub password: String,
}

/// User login request
#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// User login response
#[derive(Debug, Clone, Serialize, Schematic)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// Public user information (without sensitive data)
#[derive(Debug, Clone, Serialize, Deserialize, Selectable, Schematic)]
pub struct UserInfo {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub user_groups: Vec<String>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            is_admin: user.is_admin,
            created_at: user.created_at,
            user_groups: user.user_groups,
        }
    }
}

/// Request to update user groups
#[derive(Debug, Clone, Deserialize, Schematic)]
pub struct UpdateUserGroupsRequest {
    pub user_groups: Vec<String>,
}