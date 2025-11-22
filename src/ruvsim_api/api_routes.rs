use crate::api_types::LogLine;

use super::api_session::{SpawnSessionOptions, spawn_session_task};
use super::api_types::Session;
use super::api_util::verify_dependencies;

use axum::{
    Json,
    extract::{Path as AxumPath, Query, State},
    http::StatusCode,
};
use ruvsim::sim_types::SimSignalDirection;
use ruvsim::sim_types::{SimRadix, SimSignal, SimTimeUnit};
use uuid::Uuid;

use std::str::FromStr;
use std::time::Instant;

use super::api_types::{
    ApiError, ApiParameter, ApiResponseFormat, AppState, CmdRequest, CmdResponse,
    CreateSessionRequest, CreateSessionResponse, ExamineRequest, ExamineResponse, LogsQuery,
    NetDto, NetsQuery, RunRequest, RunResponse, SessionsListResponse,
};

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "OK", content_type = "text/plain"))
)]
pub async fn health() -> &'static str {
    "ok"
}

/**
 * Path: /sessions
 * Method: POST
 * Description: Create a new simulation session
 */
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/sessions",
    tag = "ruvsim",
    request_body = CreateSessionRequest,
    responses(
        (status = 200, description = "Session created", body = CreateSessionResponse),
        (status = 400, description = "Bad request", body = ErrorMessage),
        (status = 500, description = "Internal error", body = ErrorMessage)
    )
)]
pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ApiError> {
    // Load required paths from state config
    let work_dir = state.config.work_dir;
    let modelsim_path = state.config.modelsim_path;
    let vsim_bin = state.config.vsim_bin;

    // Verify dependencies
    let mut resolved_deps: Vec<String> = Vec::new();
    verify_dependencies(
        req.deps.iter().map(|s| s.as_str()).collect(),
        &mut resolved_deps,
        &work_dir,
    )?;

    // Compile and start simulator in a blocking thread
    let session_options = SpawnSessionOptions {
        work_dir: work_dir,
        work_lib: req.work_lib,
        top: req.top,
        deps: resolved_deps,
        modelsim_path: modelsim_path,
        vsim_bin: vsim_bin,
    };
    let runner = spawn_session_task(session_options).await?;

    // Create session ID, store it in sessions map
    let id = Uuid::new_v4();
    let session = Session {
        runner,
        last_access: Instant::now(),
    };
    state.sessions.lock().unwrap().insert(id, session);

    Ok(Json(CreateSessionResponse { id }))
}

/**
 * Path: /sessions/{id}/nets
 * Method: GET
 * Description: List nets in the session
 */
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/sessions/{id}/signals",
    tag = "ruvsim",
    params(
        ("id" = Uuid, Path, description = "Session ID"),
        ("path" = Option<String>, Query, description = "Signal path pattern"),
        ("direction" = Option<String>, Query, description = "Input|Output|Inout|All"),
        ("examine" = Option<bool>, Query, description = "Examine signals to include value"),
        ("radix" = Option<String>, Query, description = "binary|octal|decimal|hexadecimal|unsigned")
    ),
    responses((status = 200, description = "Signals", body = [NetDto]))
)]
pub async fn list_signals(
    AxumPath(id): AxumPath<Uuid>,
    State(state): State<AppState>,
    Query(q): Query<NetsQuery>,
) -> Result<Json<Vec<NetDto>>, ApiError> {
    let path = q.path.as_deref().unwrap_or("/*");
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.get_mut(&id).ok_or(ApiError::NotFound)?;
    session.last_access = Instant::now();

    let parsed_signal_direction: SimSignalDirection = q.parse_param()?;
    let signal_direction = match parsed_signal_direction {
        SimSignalDirection::Unknown => None,
        _ => Some(parsed_signal_direction),
    };

    let signals = session.runner.get_signals(path, signal_direction)
    .map_err(|e| ApiError::Internal(format!("Failed to query signals: {}", e)))?;

    // Redeclare, get mutable copy
    let mut out = Vec::with_capacity(signals.len());
    let mut signals = signals; // mutable if examine requested

    if q.examine {
        let radix: SimRadix = q.parse_param()?;
        for signal in signals.iter_mut() {
            session
                .runner
                .examine_signal(signal, radix)
                .map_err(|e| ApiError::Internal(format!("examine failed: {}", e)))?;
        }
    }

    for signal in signals.into_iter() {
        out.push(signal.format_field());
    }

    Ok(Json(out))
}

/**
 * Path: /sessions/{id}/run
 * Method: POST
 * Description: Execute the run command in the Session
 */
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/sessions/{id}/run",
    tag = "ruvsim",
    params(("id" = Uuid, Path, description = "Session ID")),
    request_body = RunRequest,
    responses((status = 200, description = "Run done", body = RunResponse))
)]
pub async fn run_session(
    AxumPath(id): AxumPath<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<RunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.get_mut(&id).ok_or(ApiError::NotFound)?;
    session.last_access = Instant::now();
    let start = Instant::now();
    match req {
        RunRequest::All => session.runner.run_all(),
        RunRequest::Next => session.runner.run_next(),
        RunRequest::For {time, unit} => session.runner.run_for(time, SimTimeUnit::from_str(&unit).map_err(|e| ApiError::BadRequest(e))?),
    }.map_err(|e| ApiError::Internal(format!("run failed: {}", e)))?;
    Ok(Json(RunResponse {
        ran_in_ms: start.elapsed().as_millis(),
    }))
}

/**
 * Path: /sessions/{id}/logs
 * Method: GET
 * Description: Get logs from the session, optionally filtered by substring
 */
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/sessions/{id}/logs",
    tag = "ruvsim",
    params(
        ("id" = Uuid, Path, description = "Session ID"),
        ("contains" = Option<String>, Query, description = "Substring filter for logs")
    ),
    responses((status = 200, description = "Logs", body = [LogLine]))
)]
pub async fn get_logs(
    AxumPath(id): AxumPath<Uuid>,
    State(state): State<AppState>,
    Query(q): Query<LogsQuery>,
) -> Result<Json<Vec<LogLine>>, ApiError> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.get_mut(&id).ok_or(ApiError::NotFound)?;
    session.last_access = Instant::now();
    let contains = q.contains.unwrap_or_default();
    let matches = session
        .runner
        .get_log_matches(|line| contains.is_empty() || line.contains(&contains))
        .map_err(|e| ApiError::Internal(format!("log query failed: {}", e)))?;
    Ok(Json(
        matches
            .into_iter()
            .map(|p| LogLine {
                line: p.content.clone(),
            })
            .collect(),
    ))
}

/** 
 * Path: /sessions/{id}/examine
 * Method: POST
 * Description: Examine a net in the session
 */
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/sessions/{id}/examine",
    tag = "ruvsim",
    params(("id" = Uuid, Path, description = "Session ID")),
    request_body = ExamineRequest,
    responses((status = 200, description = "Examined net", body = ExamineResponse))
)]
pub async fn examine_signal(
    AxumPath(id): AxumPath<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<ExamineRequest>,
) -> Result<Json<ExamineResponse>, ApiError> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.get_mut(&id).ok_or(ApiError::NotFound)?;
    session.last_access = Instant::now();

    // Obtain the net via query so we can reuse existing API logic
    let mut signals = session
        .runner
        .get_signals(&req.path, None)
        .map_err(|e| ApiError::Internal(format!("signal query failed: {}", e)))?;

    if signals.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "No signals found at path {}",
            req.path
        )));
    }
    let radix: SimRadix = req.parse_param()?;

    let mut signal: SimSignal = signals.remove(0);
    session
        .runner
        .examine_signal(&mut signal, radix)
        .map_err(|e| ApiError::Internal(format!("examine failed: {}", e)))?;

    Ok(Json(signal.format_field()))
}

/** 
 *  Path: /sessions/{id}/cmd
 *  Method: POST
 *  Description: Send a command to the simulator in the session
 */
#[axum::debug_handler]
#[utoipa::path(
    post,
    path = "/sessions/{id}/cmd",
    tag = "ruvsim",
    params(("id" = Uuid, Path, description = "Session ID")),
    request_body = CmdRequest,
    responses((status = 200, description = "Command sent", body = CmdResponse))
)]
pub async fn send_cmd(
    AxumPath(id): AxumPath<Uuid>,
    State(state): State<AppState>,
    Json(req): Json<CmdRequest>,
) -> Result<Json<CmdResponse>, ApiError> {
    let mut sessions = state.sessions.lock().unwrap();
    let session = sessions.get_mut(&id).ok_or(ApiError::NotFound)?;
    session.last_access = Instant::now();

    let matched = if let Some(substr) = req.expect_contains.as_deref() {
        session
            .runner
            .send_and_expect_result(&req.command, |line| line.contains(substr))
            .map_err(|e| ApiError::Internal(format!("command failed: {}", e)))?
            .into_iter()
            .map(|p| p.content.clone())
            .collect()
    } else {
        session
            .runner
            .send_command(&req.command)
            .map_err(|e| ApiError::Internal(format!("command failed: {}", e)))?;
        Vec::new()
    };

    Ok(Json(CmdResponse { matched }))
}

/**
 * Path: /sessions/{id}
 * Method: DELETE
 * Description: Delete a simulation session
 */
#[axum::debug_handler]
#[utoipa::path(
    delete,
    path = "/sessions/{id}",
    tag = "ruvsim",
    params(("id" = Uuid, Path, description = "Session ID")),
    responses((status = 204, description = "Session deleted"))
)]
pub async fn delete_session(
    AxumPath(id): AxumPath<Uuid>,
    State(state): State<AppState>,
) -> Result<StatusCode, ApiError> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(mut session) = sessions.remove(&id) {
        let _ = session.runner.finish();
        return Ok(StatusCode::NO_CONTENT);
    }
    Err(ApiError::NotFound)
}

/** 
 *  Path: /sessions
 *  Method: GET
 *  Description: List all active session IDs
 */
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/sessions",
    tag = "ruvsim",
    responses((status = 200, description = "Sessions", body = SessionsListResponse))
)]
pub async fn list_sessions(State(state): State<AppState>) -> Json<SessionsListResponse> {
    let ids = state.sessions.lock().unwrap().keys().cloned().collect();
    Json(SessionsListResponse { ids })
}
