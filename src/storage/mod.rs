//! Storage layer boundary for local cache and remote API persistence.

pub mod indexed_db;

use serde::{Deserialize, Serialize};

use crate::domain::models::AppState;
use gloo_net::http::Request;

const DEFAULT_API_BASE_URL: &str = "http://localhost:8787";
const STORE_ID_KEY: &str = "coffee_erp:store_id";
const UI_PREFERENCES_KEY: &str = "coffee_erp:ui_preferences";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageBackendError {
    pub message: String,
}

impl StorageBackendError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    Backend(String),
    Serialize(String),
    Deserialize(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UiPreferences {
    pub preferred_page: Option<String>,
    pub batch_filter: Option<String>,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPreferences {
    pub store_id: Option<String>,
    pub ui: UiPreferences,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteHttpResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionConflict {
    pub current_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadRemoteStateError {
    Transport(String),
    InvalidResponse(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveRemoteStateError {
    Transport(String),
    InvalidResponse(String),
    RevisionConflict(RevisionConflict),
}

pub trait StateCacheStore {
    fn load_state_document(&self) -> Result<Option<String>, StorageBackendError>;
    fn save_state_document(&self, document: &str) -> Result<(), StorageBackendError>;
}

pub trait LocalPreferencesStore {
    fn get_item(&self, key: &str) -> Result<Option<String>, StorageBackendError>;
    fn set_item(&self, key: &str, value: &str) -> Result<(), StorageBackendError>;
}

pub trait RemoteStateTransport {
    fn get_state(&self, url: &str) -> Result<RemoteHttpResponse, String>;
    fn put_state(&self, url: &str, body: &str) -> Result<RemoteHttpResponse, String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteStateClient {
    base_url: String,
}

impl RemoteStateClient {
    pub fn from_public_api_base_url() -> Self {
        let base_url = option_env!("PUBLIC_API_BASE_URL")
            .unwrap_or(DEFAULT_API_BASE_URL)
            .to_string();
        Self::new(base_url)
    }

    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: normalize_base_url(base_url.into()),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn state_endpoint(&self, store_id: &str) -> String {
        format!("{}/api/state?store_id={store_id}", self.base_url)
    }

    pub fn load_remote_state<T: RemoteStateTransport>(
        &self,
        transport: &T,
        store_id: &str,
    ) -> Result<AppState, LoadRemoteStateError> {
        let endpoint = self.state_endpoint(store_id);
        let response = transport
            .get_state(&endpoint)
            .map_err(LoadRemoteStateError::Transport)?;
        decode_state_response(response.status, &response.body).map_err(|error| match error {
            DecodeStateError::Deserialize(message) => {
                LoadRemoteStateError::InvalidResponse(message)
            }
            DecodeStateError::UnexpectedStatus(status) => {
                LoadRemoteStateError::InvalidResponse(format!("unexpected GET status: {status}"))
            }
        })
    }

    pub fn save_remote_state<T: RemoteStateTransport>(
        &self,
        transport: &T,
        store_id: &str,
        state: &AppState,
    ) -> Result<AppState, SaveRemoteStateError> {
        let endpoint = self.state_endpoint(store_id);
        let body = serde_json::to_string(state)
            .map_err(|error| SaveRemoteStateError::InvalidResponse(error.to_string()))?;
        let response = transport
            .put_state(&endpoint, &body)
            .map_err(SaveRemoteStateError::Transport)?;

        match response.status {
            200 => {
                decode_state_response(response.status, &response.body).map_err(
                    |error| match error {
                        DecodeStateError::Deserialize(message) => {
                            SaveRemoteStateError::InvalidResponse(message)
                        }
                        DecodeStateError::UnexpectedStatus(status) => {
                            SaveRemoteStateError::InvalidResponse(format!(
                                "unexpected PUT status: {status}"
                            ))
                        }
                    },
                )
            }
            409 => decode_conflict(&response.body),
            status => Err(SaveRemoteStateError::InvalidResponse(format!(
                "unexpected PUT status: {status}"
            ))),
        }
    }
}

pub fn load_cached_state<T: StateCacheStore>(store: &T) -> Result<Option<AppState>, StorageError> {
    let Some(document) = store
        .load_state_document()
        .map_err(|error| StorageError::Backend(error.message))?
    else {
        return Ok(None);
    };
    let state = serde_json::from_str(&document)
        .map_err(|error| StorageError::Deserialize(error.to_string()))?;
    Ok(Some(state))
}

pub fn save_cached_state<T: StateCacheStore>(
    store: &T,
    state: &AppState,
) -> Result<(), StorageError> {
    let document =
        serde_json::to_string(state).map_err(|error| StorageError::Serialize(error.to_string()))?;
    store
        .save_state_document(&document)
        .map_err(|error| StorageError::Backend(error.message))
}

pub fn load_preferences<T: LocalPreferencesStore>(
    store: &T,
) -> Result<Option<StoredPreferences>, StorageError> {
    let store_id = store
        .get_item(STORE_ID_KEY)
        .map_err(|error| StorageError::Backend(error.message))?
        .and_then(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        });
    let ui_json = store
        .get_item(UI_PREFERENCES_KEY)
        .map_err(|error| StorageError::Backend(error.message))?;

    if store_id.is_none() && ui_json.is_none() {
        return Ok(None);
    }

    let ui = match ui_json {
        Some(value) => serde_json::from_str(&value)
            .map_err(|error| StorageError::Deserialize(error.to_string()))?,
        None => UiPreferences::default(),
    };

    Ok(Some(StoredPreferences { store_id, ui }))
}

pub fn save_preferences<T: LocalPreferencesStore>(
    store: &T,
    preferences: &StoredPreferences,
) -> Result<(), StorageError> {
    if let Some(store_id) = &preferences.store_id {
        store
            .set_item(STORE_ID_KEY, store_id)
            .map_err(|error| StorageError::Backend(error.message))?;
    } else {
        store
            .set_item(STORE_ID_KEY, "")
            .map_err(|error| StorageError::Backend(error.message))?;
    }

    let ui_json = serde_json::to_string(&preferences.ui)
        .map_err(|error| StorageError::Serialize(error.to_string()))?;
    store
        .set_item(UI_PREFERENCES_KEY, &ui_json)
        .map_err(|error| StorageError::Backend(error.message))
}

#[derive(Debug)]
pub(crate) enum DecodeStateError {
    Deserialize(String),
    UnexpectedStatus(u16),
}

#[derive(Debug, Deserialize)]
struct StateEnvelope {
    state: AppState,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    code: String,
    current_revision: Option<u64>,
}

pub(crate) fn decode_state_response(status: u16, body: &str) -> Result<AppState, DecodeStateError> {
    if status != 200 {
        return Err(DecodeStateError::UnexpectedStatus(status));
    }
    let envelope: StateEnvelope = serde_json::from_str(body)
        .map_err(|error| DecodeStateError::Deserialize(error.to_string()))?;
    Ok(envelope.state)
}

pub(crate) fn decode_conflict(body: &str) -> Result<AppState, SaveRemoteStateError> {
    let envelope: ErrorEnvelope = serde_json::from_str(body)
        .map_err(|error| SaveRemoteStateError::InvalidResponse(error.to_string()))?;
    if envelope.error.code != "revision_conflict" {
        return Err(SaveRemoteStateError::InvalidResponse(format!(
            "unexpected conflict code: {}",
            envelope.error.code
        )));
    }
    Err(SaveRemoteStateError::RevisionConflict(RevisionConflict {
        current_revision: envelope.error.current_revision.unwrap_or(0),
    }))
}

pub async fn load_remote_state_wasm(
    base_url: &str,
    store_id: &str,
) -> Result<AppState, LoadRemoteStateError> {
    let client = RemoteStateClient::new(base_url);
    let url = client.state_endpoint(store_id);
    let response = Request::get(&url)
        .send()
        .await
        .map_err(|error| LoadRemoteStateError::Transport(error.to_string()))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| LoadRemoteStateError::Transport(error.to_string()))?;
    decode_state_response(status, &body).map_err(|error| match error {
        DecodeStateError::Deserialize(message) => LoadRemoteStateError::InvalidResponse(message),
        DecodeStateError::UnexpectedStatus(status) => {
            LoadRemoteStateError::InvalidResponse(format!("unexpected GET status: {status}"))
        }
    })
}

pub async fn save_remote_state_wasm(
    base_url: &str,
    store_id: &str,
    state: &AppState,
) -> Result<AppState, SaveRemoteStateError> {
    let client = RemoteStateClient::new(base_url);
    let url = client.state_endpoint(store_id);
    let body = serde_json::to_string(state)
        .map_err(|error| SaveRemoteStateError::InvalidResponse(error.to_string()))?;
    let request = Request::put(&url)
        .header("Content-Type", "application/json")
        .body(body)
        .map_err(|error| SaveRemoteStateError::Transport(error.to_string()))?;
    let response = request
        .send()
        .await
        .map_err(|error| SaveRemoteStateError::Transport(error.to_string()))?;
    let status = response.status();
    let response_body = response
        .text()
        .await
        .map_err(|error| SaveRemoteStateError::Transport(error.to_string()))?;
    match status {
        200 => decode_state_response(status, &response_body).map_err(|error| match error {
            DecodeStateError::Deserialize(message) => {
                SaveRemoteStateError::InvalidResponse(message)
            }
            DecodeStateError::UnexpectedStatus(status) => {
                SaveRemoteStateError::InvalidResponse(format!("unexpected PUT status: {status}"))
            }
        }),
        409 => decode_conflict(&response_body),
        status => Err(SaveRemoteStateError::InvalidResponse(format!(
            "unexpected PUT status: {status}"
        ))),
    }
}

fn normalize_base_url(base_url: String) -> String {
    let trimmed = base_url.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        DEFAULT_API_BASE_URL.to_string()
    } else {
        trimmed
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use crate::domain::seed::seed_app_state;

    use super::{
        LoadRemoteStateError, LocalPreferencesStore, RemoteHttpResponse, RemoteStateClient,
        RemoteStateTransport, SaveRemoteStateError, StateCacheStore, StorageBackendError,
        StorageError, StoredPreferences, UI_PREFERENCES_KEY, UiPreferences, load_cached_state,
        load_preferences, save_cached_state, save_preferences,
    };

    #[test]
    fn save_cached_state_and_load_cached_state_roundtrip() {
        let backend = MemoryStateCacheStore::default();
        let state = seed_app_state();

        save_cached_state(&backend, &state).expect("state save should succeed");
        let loaded = load_cached_state(&backend)
            .expect("state load should succeed")
            .expect("cached state should exist");

        assert_eq!(loaded, state);
    }

    #[test]
    fn load_cached_state_returns_none_when_cache_empty() {
        let backend = MemoryStateCacheStore::default();

        let loaded = load_cached_state(&backend).expect("state load should succeed");

        assert_eq!(loaded, None);
    }

    #[test]
    fn load_cached_state_returns_deserialize_error_for_invalid_json() {
        let backend = MemoryStateCacheStore::with_json("invalid-json");

        let error = load_cached_state(&backend).expect_err("invalid json should fail");

        assert_eq!(storage_error_kind(&error), "deserialize");
    }

    #[test]
    fn save_preferences_and_load_preferences_roundtrip() {
        let backend = MemoryLocalPreferencesStore::default();
        let preferences = StoredPreferences {
            store_id: Some("store-abc".to_string()),
            ui: UiPreferences {
                preferred_page: Some("today".to_string()),
                batch_filter: Some("active".to_string()),
                last_synced_at: Some("2026-05-03T08:00:00Z".to_string()),
            },
        };

        save_preferences(&backend, &preferences).expect("preferences save should succeed");
        let loaded = load_preferences(&backend)
            .expect("preferences load should succeed")
            .expect("preferences should exist");

        assert_eq!(loaded, preferences);
    }

    #[test]
    fn load_preferences_returns_none_when_values_absent() {
        let backend = MemoryLocalPreferencesStore::default();

        let loaded = load_preferences(&backend).expect("preferences load should succeed");

        assert_eq!(loaded, None);
    }

    #[test]
    fn load_preferences_returns_deserialize_error_for_invalid_ui_json() {
        let backend = MemoryLocalPreferencesStore::default();
        backend
            .set_item(UI_PREFERENCES_KEY, "invalid-json")
            .expect("test preferences write should succeed");

        let error = load_preferences(&backend).expect_err("invalid preferences json should fail");

        assert_eq!(storage_error_kind(&error), "deserialize");
    }

    #[test]
    fn remote_state_client_uses_public_api_base_url_and_builds_endpoint() {
        let client = RemoteStateClient::new("https://api.coffee.example.com/");

        assert_eq!(client.base_url(), "https://api.coffee.example.com");
        assert_eq!(
            client.state_endpoint("store-abc"),
            "https://api.coffee.example.com/api/state?store_id=store-abc"
        );
    }

    #[test]
    fn remote_state_client_load_remote_state_returns_state_on_200() {
        let state = seed_app_state();
        let body = serde_json::json!({
            "state": state
        })
        .to_string();
        let transport = MockRemoteStateTransport {
            get_response: RefCell::new(Some(RemoteHttpResponse { status: 200, body })),
            put_response: RefCell::new(None),
            get_error: RefCell::new(None),
            put_error: RefCell::new(None),
        };
        let client = RemoteStateClient::new("https://api.coffee.example.com");

        let loaded = client
            .load_remote_state(&transport, "store-abc")
            .expect("remote load should succeed");

        assert_eq!(loaded, state);
    }

    #[test]
    fn remote_state_client_save_remote_state_returns_conflict_error() {
        let state = seed_app_state();
        let transport = MockRemoteStateTransport {
            get_response: RefCell::new(None),
            put_response: RefCell::new(Some(RemoteHttpResponse {
                status: 409,
                body: serde_json::json!({
                    "error": {
                        "code": "revision_conflict",
                        "message": "state revision is stale, refresh before retry",
                        "current_revision": 4
                    }
                })
                .to_string(),
            })),
            get_error: RefCell::new(None),
            put_error: RefCell::new(None),
        };
        let client = RemoteStateClient::new("https://api.coffee.example.com");

        let error = client
            .save_remote_state(&transport, "store-abc", &state)
            .expect_err("remote save should fail with revision conflict");

        assert_eq!(
            error,
            SaveRemoteStateError::RevisionConflict(super::RevisionConflict {
                current_revision: 4
            })
        );
    }

    #[test]
    fn remote_state_client_load_remote_state_returns_invalid_response_for_non_200() {
        let transport = MockRemoteStateTransport {
            get_response: RefCell::new(Some(RemoteHttpResponse {
                status: 500,
                body: String::new(),
            })),
            put_response: RefCell::new(None),
            get_error: RefCell::new(None),
            put_error: RefCell::new(None),
        };
        let client = RemoteStateClient::new("https://api.coffee.example.com");

        let error = client
            .load_remote_state(&transport, "store-abc")
            .expect_err("non-200 remote load should fail");

        assert_eq!(
            error,
            LoadRemoteStateError::InvalidResponse("unexpected GET status: 500".to_string())
        );
    }

    #[test]
    fn remote_state_client_save_remote_state_returns_invalid_response_for_unknown_conflict_code() {
        let state = seed_app_state();
        let transport = MockRemoteStateTransport {
            get_response: RefCell::new(None),
            put_response: RefCell::new(Some(RemoteHttpResponse {
                status: 409,
                body: serde_json::json!({
                    "error": {
                        "code": "other_conflict",
                        "message": "other conflict"
                    }
                })
                .to_string(),
            })),
            get_error: RefCell::new(None),
            put_error: RefCell::new(None),
        };
        let client = RemoteStateClient::new("https://api.coffee.example.com");

        let error = client
            .save_remote_state(&transport, "store-abc", &state)
            .expect_err("unknown conflict code should fail");

        assert_eq!(
            error,
            SaveRemoteStateError::InvalidResponse(
                "unexpected conflict code: other_conflict".to_string()
            )
        );
    }

    #[test]
    fn remote_state_client_save_remote_state_returns_state_on_200() {
        let state = seed_app_state();
        let mut saved_state = seed_app_state();
        saved_state.revision = 1;
        let body = serde_json::json!({
            "state": saved_state
        })
        .to_string();
        let transport = MockRemoteStateTransport {
            get_response: RefCell::new(None),
            put_response: RefCell::new(Some(RemoteHttpResponse { status: 200, body })),
            get_error: RefCell::new(None),
            put_error: RefCell::new(None),
        };
        let client = RemoteStateClient::new("https://api.coffee.example.com");

        let response = client
            .save_remote_state(&transport, "store-abc", &state)
            .expect("remote save should succeed");

        assert_eq!(response, saved_state);
    }

    #[test]
    fn remote_state_client_load_remote_state_returns_transport_error() {
        let transport = MockRemoteStateTransport {
            get_response: RefCell::new(None),
            put_response: RefCell::new(None),
            get_error: RefCell::new(Some("network unavailable".to_string())),
            put_error: RefCell::new(None),
        };
        let client = RemoteStateClient::new("https://api.coffee.example.com");

        let error = client
            .load_remote_state(&transport, "store-abc")
            .expect_err("remote load should fail");

        assert_eq!(
            error,
            LoadRemoteStateError::Transport("network unavailable".to_string())
        );
    }

    fn storage_error_kind(error: &StorageError) -> &'static str {
        match error {
            StorageError::Backend(_) => "backend",
            StorageError::Serialize(_) => "serialize",
            StorageError::Deserialize(_) => "deserialize",
        }
    }

    #[derive(Default)]
    struct MemoryStateCacheStore {
        state_json: RefCell<Option<String>>,
    }

    impl MemoryStateCacheStore {
        fn with_json(value: &str) -> Self {
            Self {
                state_json: RefCell::new(Some(value.to_string())),
            }
        }
    }

    impl StateCacheStore for MemoryStateCacheStore {
        fn load_state_document(&self) -> Result<Option<String>, StorageBackendError> {
            Ok(self.state_json.borrow_mut().take())
        }

        fn save_state_document(&self, document: &str) -> Result<(), StorageBackendError> {
            *self.state_json.borrow_mut() = Some(document.to_string());
            Ok(())
        }
    }

    #[derive(Default)]
    struct MemoryLocalPreferencesStore {
        values: RefCell<HashMap<String, String>>,
    }

    impl LocalPreferencesStore for MemoryLocalPreferencesStore {
        fn get_item(&self, key: &str) -> Result<Option<String>, StorageBackendError> {
            Ok(self.values.borrow().get(key).cloned())
        }

        fn set_item(&self, key: &str, value: &str) -> Result<(), StorageBackendError> {
            self.values
                .borrow_mut()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }
    }

    struct MockRemoteStateTransport {
        get_response: RefCell<Option<RemoteHttpResponse>>,
        put_response: RefCell<Option<RemoteHttpResponse>>,
        get_error: RefCell<Option<String>>,
        put_error: RefCell<Option<String>>,
    }

    impl RemoteStateTransport for MockRemoteStateTransport {
        fn get_state(&self, _url: &str) -> Result<RemoteHttpResponse, String> {
            if let Some(error) = self.get_error.borrow_mut().take() {
                return Err(error);
            }
            self.get_response
                .borrow_mut()
                .take()
                .ok_or_else(|| "missing GET response".to_string())
        }

        fn put_state(&self, _url: &str, _body: &str) -> Result<RemoteHttpResponse, String> {
            if let Some(error) = self.put_error.borrow_mut().take() {
                return Err(error);
            }
            self.put_response
                .borrow_mut()
                .take()
                .ok_or_else(|| "missing PUT response".to_string())
        }
    }
}
