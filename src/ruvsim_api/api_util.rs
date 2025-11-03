use ruvsim::sim_types::SimRadix;

use super::api_types::ApiError;
use std::path::{Component, Path};

pub fn verify_dependencies(
    paths: Vec<&str>,
    resolved_deps: &mut Vec<String>,
    work_dir: &str,
) -> Result<(), ApiError> {
    for p in paths {
        let path: &Path = Path::new(p);
        // Check absolute dependency path
        if path.is_absolute() {
            log::error!("Absolute dependency path not allowed: {}", p);
            return Err(ApiError::BadRequest(p.to_string()));
        }

        // Check parent traversal
        if path.components().any(|c| c == Component::ParentDir) {
            log::error!(
                "Parent directory traversal not allowed in dependency path: {}",
                p
            );
            return Err(ApiError::BadRequest(p.to_string()));
        }

        let path = Path::new(work_dir).join(path);
        // Cheeck if exists
        if !path.exists() {
            log::error!("Dependency path does not exist: {}", p);
            return Err(ApiError::BadRequest(p.to_string()));
        }

        // Canonicalize, make sure it is still under work_dir
        let canonicalized = path.canonicalize().map_err(|_| {
            ApiError::Internal("Failed to canonicalize dependency path".to_string())
        })?;
        let work_dir_path = Path::new(work_dir)
            .canonicalize()
            .map_err(|_| ApiError::Internal("Failed to canonicalize work_dir".to_string()))?;
        if !canonicalized.starts_with(&work_dir_path) {
            log::error!(
                "Dependency path is outside of work_dir: {}",
                canonicalized.display()
            );
            return Err(ApiError::BadRequest(p.to_string()));
        }

        resolved_deps.push(canonicalized.to_string_lossy().to_string());
    }
    Ok(())
}

#[allow(dead_code)]
pub fn infer_radix(value: &str) -> SimRadix {
    if value.chars().all(|c| c == '0' || c == '1') {
        SimRadix::Binary
    } else if value.chars().all(|c| c.is_digit(10)) {
        SimRadix::Decimal
    } else if value.chars().all(|c| c.is_digit(16)) {
        SimRadix::Hexadecimal
    } else {
        SimRadix::Unknown
    }
}
