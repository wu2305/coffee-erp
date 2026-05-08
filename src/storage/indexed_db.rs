//! IndexedDB cache implementation for WASM browser environment.
//! Uses web_sys low-level bindings wrapped in wasm_bindgen_futures::JsFuture.

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    DomException, IdbDatabase, IdbObjectStore, IdbOpenDbRequest, IdbRequest, IdbTransactionMode,
    Window,
};

use crate::domain::models::AppState;
use crate::storage::{StorageBackendError, StorageError};

const DB_NAME: &str = "coffee_erp";
const DB_VERSION: u32 = 1;
const STORE_NAME: &str = "state_cache";

fn window() -> Result<Window, StorageBackendError> {
    web_sys::window().ok_or_else(|| StorageBackendError::new("window unavailable"))
}

fn js_error(e: JsValue) -> StorageBackendError {
    if let Some(dom) = e.dyn_ref::<DomException>() {
        StorageBackendError::new(format!("DOMException: {} ({})", dom.message(), dom.name()))
    } else {
        StorageBackendError::new(format!("{:?}", e))
    }
}

async fn idb_request_to_future(request: IdbRequest) -> Result<JsValue, JsValue> {
    let request = Rc::new(RefCell::new(Some(request)));

    let promise =
        js_sys::Promise::new(&mut |resolve: js_sys::Function, reject: js_sys::Function| {
            let req_success = request.clone();
            let success = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                if let Some(req) = req_success.borrow_mut().take() {
                    let result = req.result().unwrap_or(JsValue::NULL);
                    let _ = resolve.call1(&JsValue::NULL, &result);
                }
            }) as Box<dyn FnMut(web_sys::Event)>);
            if let Some(req) = request.borrow().as_ref() {
                req.set_onsuccess(Some(success.as_ref().unchecked_ref()));
            }
            success.forget();

            let req_error = request.clone();
            let error = Closure::wrap(Box::new(move |_event: web_sys::Event| {
                if let Some(req) = req_error.borrow_mut().take() {
                    let err = req
                        .error()
                        .map(|e| JsValue::from(e))
                        .unwrap_or(JsValue::NULL);
                    let _ = reject.call1(&JsValue::NULL, &err);
                }
            }) as Box<dyn FnMut(web_sys::Event)>);
            if let Some(req) = request.borrow().as_ref() {
                req.set_onerror(Some(error.as_ref().unchecked_ref()));
            }
            error.forget();
        });

    JsFuture::from(promise).await
}

async fn open_db() -> Result<IdbDatabase, StorageBackendError> {
    let window = window()?;
    let factory = window
        .indexed_db()
        .map_err(|e| StorageBackendError::new(format!("indexed_db unavailable: {:?}", e)))?
        .ok_or_else(|| StorageBackendError::new("indexed_db not supported"))?;

    let request: IdbOpenDbRequest = factory
        .open_with_f64(DB_NAME, DB_VERSION as f64)
        .map_err(|e| StorageBackendError::new(format!("open failed: {:?}", e)))?;

    let request_clone = request.clone();
    let upgrade = Closure::once_into_js(move |event: web_sys::IdbVersionChangeEvent| {
        if let Ok(result) = request_clone.result() {
            if let Ok(db) = result.dyn_into::<IdbDatabase>() {
                let names = db.object_store_names();
                let mut found = false;
                for i in 0..names.length() {
                    if names.item(i).as_deref() == Some(STORE_NAME) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    let _ = db.create_object_store(STORE_NAME);
                }
            }
        }
        let _ = event;
    });
    request.set_onupgradeneeded(Some(upgrade.unchecked_ref()));

    let result = idb_request_to_future(request.into())
        .await
        .map_err(|e| StorageBackendError::new(format!("open db promise rejected: {:?}", e)))?;

    result
        .dyn_into::<IdbDatabase>()
        .map_err(|e| StorageBackendError::new(format!("open db cast failed: {:?}", e)))
}

async fn with_store(mode: IdbTransactionMode) -> Result<IdbObjectStore, StorageBackendError> {
    let db = open_db().await?;
    let tx = db
        .transaction_with_str_and_mode(STORE_NAME, mode)
        .map_err(js_error)?;
    let store = tx.object_store(STORE_NAME).map_err(js_error)?;
    Ok(store)
}

pub async fn load_cached_state_web(store_id: &str) -> Result<Option<AppState>, StorageError> {
    let store = with_store(IdbTransactionMode::Readonly)
        .await
        .map_err(|e| StorageError::Backend(e.message))?;

    let key = state_key(store_id);
    let request = store
        .get(&JsValue::from_str(&key))
        .map_err(|e| StorageError::Backend(js_error(e).message))?;

    let result = idb_request_to_future(request)
        .await
        .map_err(|e| StorageError::Backend(js_error(e).message))?;

    if result.is_null() || result.is_undefined() {
        return Ok(None);
    }

    let json = result
        .as_string()
        .ok_or_else(|| StorageError::Deserialize("cached value is not a string".to_string()))?;
    let state = serde_json::from_str::<AppState>(&json)
        .map_err(|e| StorageError::Deserialize(e.to_string()))?;
    Ok(Some(state))
}

pub async fn save_cached_state_web(store_id: &str, state: &AppState) -> Result<(), StorageError> {
    let store = with_store(IdbTransactionMode::Readwrite)
        .await
        .map_err(|e| StorageError::Backend(e.message))?;

    let key = state_key(store_id);
    let json = serde_json::to_string(state).map_err(|e| StorageError::Serialize(e.to_string()))?;

    let request = store
        .put_with_key(&JsValue::from_str(&json), &JsValue::from_str(&key))
        .map_err(|e| StorageError::Backend(js_error(e).message))?;

    idb_request_to_future(request)
        .await
        .map_err(|e| StorageError::Backend(js_error(e).message))?;

    Ok(())
}

fn state_key(store_id: &str) -> String {
    format!("coffee_erp:store:{}:state", store_id)
}
