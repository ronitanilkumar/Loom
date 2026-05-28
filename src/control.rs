use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type NodeMap = Arc<Mutex<HashMap<String, Vec<String>>>>;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub node: String,
    pub model: String,
}

#[derive(Serialize)]
pub struct LocateResponse {
    pub model: String,
    pub nodes: Vec<String>,
}

async fn register(
    State(map): State<NodeMap>,
    Json(req): Json<RegisterRequest>,
) -> Json<serde_json::Value> {
    let mut map = map.lock().unwrap();
    map.entry(req.model.clone())
        .or_insert_with(Vec::new)
        .push(req.node.clone());
    Json(serde_json::json!({ "status": "ok" }))
}

async fn locate(
    Path(model): Path<String>,
    State(map): State<NodeMap>,
) -> Json<LocateResponse> {
    let map = map.lock().unwrap();
    let nodes = map.get(&model).cloned().unwrap_or_default();
    Json(LocateResponse { model, nodes })
}

pub fn router() -> Router {
    let map: NodeMap = Arc::new(Mutex::new(HashMap::new()));
    Router::new()
        .route("/register", post(register))
        .route("/locate/:model", get(locate))
        .with_state(map)
}