mod api_routes;
mod api_session;
mod api_types;
mod api_util;

use std::{
    env,
    time::{Duration, Instant},
};

use axum::{
    Router,
    routing::{delete, get, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{EnvFilter, fmt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use uuid::Uuid;

use api_routes::{
    create_session, delete_session, examine_signal, get_logs, health, list_signals, list_sessions,
    run_session, send_cmd,
};
use api_types::{AppState, Config};

use crate::api_types::ApiDoc;

#[tokio::main]
async fn main() {
    env_logger::init();
    // init tracing
    let _ = fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .try_init();

    // Load config from env
    let config = Config {
        work_dir: env::var("RUVSIM_WORK_DIR").expect("RUVSIM_WORK_DIR not set"),
        modelsim_path: env::var("RUVSIM_MODELSIM_PATH").unwrap_or_default(),
        vsim_bin: env::var("RUVSIM_VSIM_BIN").unwrap_or_else(|_| "vsim".to_string()),
    };
    let state = AppState::new(config);

    let app = build_app(state.clone());

    // Background eviction task
    let evict_state = state.clone();
    tokio::spawn(async move {
        let ttl = std::env::var("RUVSIM_SESSION_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(600);
        let ttl = Duration::from_secs(ttl);
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            let mut guard = evict_state.sessions.lock().unwrap();
            let now = Instant::now();
            let ids_to_remove: Vec<Uuid> = guard
                .iter()
                .filter_map(|(id, s)| {
                    if now.duration_since(s.last_access) > ttl {
                        Some(*id)
                    } else {
                        None
                    }
                })
                .collect();
            for id in ids_to_remove {
                if let Some(mut s) = guard.remove(&id) {
                    let _ = s.runner.finish();
                }
            }
        }
    });

    let bind_addr = env::var("RUVSIM_API_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listener = tokio::net::TcpListener::bind(&bind_addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .unwrap();
}

fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/sessions", get(list_sessions).post(create_session))
        .route("/sessions/:id/signals", get(list_signals))
        .route("/sessions/:id/run", post(run_session))
        .route("/sessions/:id/logs", get(get_logs))
        .route("/sessions/:id/examine", post(examine_signal))
        .route("/sessions/:id/cmd", post(send_cmd))
        .route("/sessions/:id", delete(delete_session))
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::fs;
    use tower::ServiceExt; // for oneshot

    fn new_test_app() -> Router {
        // create a temp work dir under target/tmp without using env::set_var
        let base = std::env::current_dir()
            .unwrap()
            .join("target/tmp/test_work_dir");
        let _ = fs::create_dir_all(&base);
        let config = Config {
            work_dir: base.to_string_lossy().to_string(),
            modelsim_path: String::new(),
            vsim_bin: "vsim".to_string(),
        };
        let state = AppState::new(config);
        build_app(state)
    }

    #[tokio::test]
    async fn reject_absolute_dep() {
        let app = new_test_app();
        let body = serde_json::json!({
            "work_lib": "work",
            "top": "top",
            "deps": ["/etc/passwd"]
        });
        let res = app
            .oneshot(
                Request::post("/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn reject_parent_dir_dep() {
        let app = new_test_app();
        let body = serde_json::json!({
            "work_lib": "work",
            "top": "top",
            "deps": ["../escape.sv"]
        });
        let res = app
            .oneshot(
                Request::post("/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn missing_dep_returns_400() {
        let app = new_test_app();
        let body = serde_json::json!({
            "work_lib": "work",
            "top": "top",
            "deps": ["hdl/does_not_exist.sv"]
        });
        let res = app
            .oneshot(
                Request::post("/sessions")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}
