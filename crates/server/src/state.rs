use std::sync::Arc;

use cc_switch_core::CoreContext;

use crate::auth::{AuthConfig, SessionStore};
use crate::events::EventSender;

pub struct ServerState {
    pub auth_token: Option<String>,
    pub event_bus: EventSender,
    pub core: CoreContext,
    pub session_store: Arc<SessionStore>,
    pub auth_config: Option<AuthConfig>,
}

impl ServerState {
    pub fn new(
        auth_token: Option<String>,
        event_bus: EventSender,
        session_store: Arc<SessionStore>,
        auth_config: Option<AuthConfig>,
    ) -> Arc<Self> {
        // 初始化核心上下文（数据库、SkillService 等）
        let core = CoreContext::new().unwrap_or_else(|e| {
            panic!("failed to initialize cc-switch core context: {e}");
        });
        let auto_sync_events = event_bus.clone();
        cc_switch_core::set_s3_auto_sync_status_callback(Arc::new(move |status, error| {
            let mut payload = serde_json::json!({
                "source": "auto",
                "status": status,
            });
            if let Some(message) = error {
                payload["error"] = serde_json::Value::String(message.to_string());
            }
            let _ = auto_sync_events.send(crate::events::ServerEvent {
                name: "s3-sync-status-updated".to_string(),
                payload,
            });
        }));
        Arc::new(Self {
            auth_token,
            event_bus,
            core,
            session_store,
            auth_config,
        })
    }
}
