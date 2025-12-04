use super::api_routes::{
    __path_create_session, __path_delete_session, __path_examine_signal, __path_get_logs,
    __path_health, __path_list_sessions, __path_list_signals, __path_run_session, __path_send_cmd,
};

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use ruvsim::{
    sim_runner::Runner,
    sim_types::{SimDriver, SimRadix, SimSignal, SimSignalDirection},
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use uuid::Uuid;

#[derive(Clone)]
pub struct Config {
    pub work_dir: String,
    pub modelsim_path: String,
    pub vsim_bin: String,
}

#[derive(Clone)]
pub struct AppState {
    pub sessions: Arc<Mutex<HashMap<Uuid, Session>>>,
    pub config: Config,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }
}

pub struct Session {
    pub runner: Runner,
    pub last_access: Instant,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorMessage {
    message: String,
}

pub trait ApiParameter<T> {
    fn parse_param(&self) -> Result<T, ApiError>;
}

impl ApiParameter<SimSignalDirection> for NetsQuery {
    fn parse_param(&self) -> Result<SimSignalDirection, ApiError> {
        match self.direction.as_deref() {
            Some("Input") => Ok(SimSignalDirection::Input),
            Some("Output") => Ok(SimSignalDirection::Output),
            Some("Inout") => Ok(SimSignalDirection::Inout),
            Some("All") | None => Ok(SimSignalDirection::Unknown),
            Some(other) => Err(ApiError::BadRequest(format!(
                "Invalid direction parameter: {}",
                other
            ))),
        }
    }
}

fn parse_radix(radix_str: Option<&str>) -> SimRadix {
    match radix_str {
        Some("binary") | Some("bin") => SimRadix::Binary,
        Some("octal") | Some("oct") => SimRadix::Octal,
        Some("decimal") | Some("dec") => SimRadix::Decimal,
        Some("hexadecimal") | Some("hex") => SimRadix::Hexadecimal,
        Some("unsigned") | Some("uns") => SimRadix::Unsigned,
        _ => SimRadix::Unknown,
    }
}

impl ApiParameter<SimRadix> for NetsQuery {
    fn parse_param(&self) -> Result<SimRadix, ApiError> {
        Ok(parse_radix(self.radix.as_deref()))
    }
}

impl ApiParameter<SimRadix> for ExamineRequest {
    fn parse_param(&self) -> Result<SimRadix, ApiError> {
        Ok(parse_radix(self.radix.as_deref()))
    }
}

pub trait ApiResponseFormat<T> {
    fn format_field(&self) -> T;
}

impl ApiResponseFormat<String> for SimRadix {
    fn format_field(&self) -> String {
        match self {
            SimRadix::Binary => "binary",
            SimRadix::Octal => "octal",
            SimRadix::Decimal => "decimal",
            SimRadix::Hexadecimal => "hexadecimal",
            SimRadix::Unsigned => "unsigned",
            SimRadix::Unknown => "unknown",
        }
        .to_string()
    }
}

impl ApiResponseFormat<String> for SimSignalDirection {
    fn format_field(&self) -> String {
        match self {
            SimSignalDirection::Input => "Input",
            SimSignalDirection::Output => "Output",
            SimSignalDirection::Inout => "Inout",
            SimSignalDirection::Unknown => "All",
            SimSignalDirection::None => "",
        }
        .to_string()
    }
}

impl ApiResponseFormat<DriverDto> for SimDriver {
    fn format_field(&self) -> DriverDto {
        DriverDto {
            driver_type: format!("{:?}", self.driver_type),
            source: self.source.clone(),
        }
    }
}

impl ApiResponseFormat<Vec<DriverDto>> for Vec<SimDriver> {
    fn format_field(&self) -> Vec<DriverDto> {
        self.iter().map(|d| d.format_field()).collect()
    }
}

impl ApiResponseFormat<(String, String)> for Option<(SimRadix, String)> {
    fn format_field(&self) -> (String, String) {
        match self {
            Some((radix, value)) => (radix.format_field(), value.clone()),
            None => ("unknown".to_string(), "".to_string()),
        }
    }
}

impl ApiResponseFormat<NetDto> for SimSignal {
    fn format_field(&self) -> NetDto {
        let value = self.value.clone().format_field();
        NetDto {
            name: self.name.clone(),
            direction: self.direction.format_field(),
            left_bound: self.bounds.left,
            right_bound: self.bounds.right,
            drivers: self.drivers.format_field(),
            value: Some(value.1),
            radix: Some(value.0),
        }
    }
}

impl ApiResponseFormat<ExamineResponse> for SimSignal {
    fn format_field(&self) -> ExamineResponse {
        let value = self.value.clone().format_field();

        ExamineResponse {
            path: self.name.clone(),
            value: Some(value.1),
            radix: Some(value.0),
        }
    }
}

pub enum ApiError {
    NotFound,
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST, m),
            ApiError::Internal(m) => (StatusCode::INTERNAL_SERVER_ERROR, m),
        };
        (status, Json(ErrorMessage { message: msg })).into_response()
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health,
        list_sessions,
        create_session,
        list_signals,
        run_session,
        get_logs,
        examine_signal,
        send_cmd,
        delete_session,
    ),
    components(
        schemas(
            ErrorMessage,
            CreateSessionRequest,
            CreateSessionResponse,
            SessionsListResponse,
            DriverDto,
            NetDto,
            RunRequest,
            RunResponse,
            LogsQuery,
            LogLine,
            ExamineRequest,
            ExamineResponse,
            CmdRequest,
            CmdResponse,
        )
    ),
    tags(
        (name = "ruvsim", description = "RuvSim REST API")
    )
)]
pub struct ApiDoc;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSessionRequest {
    pub work_lib: String,
    pub top: String,
    /// Dependencies to compile. Must be relative to RUVSIM_WORK_DIR. Absolute paths are rejected.
    pub deps: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateSessionResponse {
    pub id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NetsQuery {
    pub path: Option<String>,
    pub direction: Option<String>, // Input|Output|Inout|All
    #[serde(default)]
    pub examine: bool,
    pub radix: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DriverDto {
    driver_type: String,
    source: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NetDto {
    pub name: String,
    pub direction: String,
    pub left_bound: i32,
    pub right_bound: i32,
    pub drivers: Vec<DriverDto>,
    pub value: Option<String>,
    pub radix: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum RunRequest {
    All,
    Next,
    For { time: u64, unit: String },
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RunResponse {
    pub ran_in_ms: u128,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LogsQuery {
    pub contains: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LogLine {
    pub line: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExamineRequest {
    pub path: String,
    #[serde(default)]
    pub radix: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExamineResponse {
    pub path: String,
    pub value: Option<String>,
    pub radix: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CmdRequest {
    pub command: String,
    pub expect_contains: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CmdResponse {
    pub matched: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SessionsListResponse {
    pub ids: Vec<Uuid>,
}
