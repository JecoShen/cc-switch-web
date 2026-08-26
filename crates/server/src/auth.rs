use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::sync::RwLock;
use std::time::{Duration, Instant};

/// Session expiry duration: 7 days (604800 seconds)
const SESSION_EXPIRY_SECS: u64 = 604800;

/// Default username when only password_hash is supplied (backward compatible)
pub const DEFAULT_USERNAME: &str = "admin";

/// Configuration for web authentication.
///
/// Supports two config shapes:
/// - New: `{ "username": "admin", "password_hash": "..." }`
/// - Legacy: `{ "password_hash": "..." }`  → username defaults to "admin"
#[derive(Debug, Clone, Deserialize)]
pub struct AuthConfig {
    /// Optional username. When absent, defaults to `"admin"`.
    #[serde(default)]
    pub username: Option<String>,
    pub password_hash: String,
}

impl AuthConfig {
    /// Returns the effective username (always non-empty)
    pub fn effective_username(&self) -> &str {
        self.username
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_USERNAME)
    }
}

/// Represents an active user session
#[derive(Debug, Clone)]
pub struct Session {
    pub token: String,
    pub created_at: Instant,
    pub expires_at: Instant,
}

/// In-memory session store with thread-safe access
pub struct SessionStore {
    sessions: RwLock<HashMap<String, Session>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a new session and returns the token
    pub fn create_session(&self) -> String {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        let token = hex::encode(bytes);

        let now = Instant::now();
        let session = Session {
            token: token.clone(),
            created_at: now,
            expires_at: now + Duration::from_secs(SESSION_EXPIRY_SECS),
        };

        let mut sessions = self.sessions.write().unwrap();
        sessions.insert(token.clone(), session);
        token
    }

    /// Validates a session token
    pub fn validate_session(&self, token: &str) -> bool {
        let sessions = self.sessions.read().unwrap();
        if let Some(session) = sessions.get(token) {
            Instant::now() < session.expires_at
        } else {
            false
        }
    }

    /// Removes expired sessions
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        let mut sessions = self.sessions.write().unwrap();
        sessions.retain(|_, session| now < session.expires_at);
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Loads authentication configuration from ~/.cc-switch/web-auth.json
/// Returns None if file is missing or invalid (auth disabled)
pub fn load_auth_config() -> Option<AuthConfig> {
    let home = dirs::home_dir()?;
    let config_path = home.join(".cc-switch").join("web-auth.json");

    let content = fs::read_to_string(&config_path).ok()?;
    let config: AuthConfig = serde_json::from_str(&content).ok()?;

    // Validate that password_hash is not empty
    if config.password_hash.is_empty() {
        return None;
    }

    Some(config)
}

/// Verifies a password against a bcrypt hash (legacy helper, used when only
/// a password is supplied).
pub fn verify_password(password: &str, hash: &str) -> bool {
    bcrypt::verify(password, hash).unwrap_or(false)
}

/// Verifies username + password against an AuthConfig.
///
/// Returns true only when the supplied username matches the configured
/// username (default `"admin"`) AND the password matches the bcrypt hash.
pub fn verify_credentials(
    username: &str,
    password: &str,
    config: &AuthConfig,
) -> bool {
    // Case-sensitive username match
    if username != config.effective_username() {
        return false;
    }
    verify_password(password, &config.password_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_store_create_and_validate() {
        let store = SessionStore::new();
        let token = store.create_session();

        assert_eq!(token.len(), 64); // 32 bytes = 64 hex chars
        assert!(store.validate_session(&token));
        assert!(!store.validate_session("invalid_token"));
    }

    #[test]
    fn test_session_token_is_unique() {
        let store = SessionStore::new();
        let token1 = store.create_session();
        let token2 = store.create_session();
        assert_ne!(token1, token2);
    }

    #[test]
    fn test_verify_password_with_valid_hash() {
        // Pre-generated bcrypt hash for "test123" with cost 4 (for fast tests)
        let hash = "$2b$04$MJuc/Azj7j9Js28.20f31uIhhVpf8f1GqCdPbh3D5StxPf8/FxYSi";
        assert!(verify_password("test123", hash));
        assert!(!verify_password("wrong", hash));
    }

    #[test]
    fn test_verify_password_with_invalid_hash() {
        assert!(!verify_password("test", "invalid_hash"));
        assert!(!verify_password("test", ""));
    }

    #[test]
    fn test_load_auth_config_missing_file() {
        // This test assumes no config file exists at the path
        let _ = load_auth_config();
    }

    #[test]
    fn test_verify_credentials_with_default_admin() {
        let config = AuthConfig {
            username: None,
            password_hash: "$2b$04$MJuc/Azj7j9Js28.20f31uIhhVpf8f1GqCdPbh3D5StxPf8/FxYSi"
                .to_string(),
        };
        assert_eq!(config.effective_username(), "admin");
        assert!(verify_credentials("admin", "test123", &config));
        assert!(!verify_credentials("admin", "wrong", &config));
        assert!(!verify_credentials("root", "test123", &config));
    }

    #[test]
    fn test_verify_credentials_with_custom_username() {
        let config = AuthConfig {
            username: Some("alice".to_string()),
            password_hash: "$2b$04$MJuc/Azj7j9Js28.20f31uIhhVpf8f1GqCdPbh3D5StxPf8/FxYSi"
                .to_string(),
        };
        assert_eq!(config.effective_username(), "alice");
        assert!(verify_credentials("alice", "test123", &config));
        assert!(!verify_credentials("admin", "test123", &config));
    }

    #[test]
    fn test_verify_credentials_rejects_empty_username() {
        let config = AuthConfig {
            username: Some(String::new()),
            password_hash: "$2b$04$MJuc/Azj7j9Js28.20f31uIhhVpf8f1GqCdPbh3D5StxPf8/FxYSi"
                .to_string(),
        };
        // Empty username falls back to default "admin"
        assert_eq!(config.effective_username(), "admin");
        assert!(verify_credentials("admin", "test123", &config));
    }
}
