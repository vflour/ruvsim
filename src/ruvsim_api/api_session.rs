use super::api_types::ApiError;

use ruvsim::{
    sim_compiler::{Compiler, CompilerCommand},
    sim_runner::Runner,
};
use tokio::task;

pub struct SpawnSessionOptions {
    pub work_dir: String,
    pub work_lib: String,
    pub top: String,
    pub deps: Vec<String>,
    #[allow(dead_code)]
    pub modelsim_path: String,
    pub vsim_bin: String,
}

/**
 * Spawns a simulation session in a blocking task
 * If the task panics, it should return an ApiError:Internal
 */
pub async fn spawn_session_task(options: SpawnSessionOptions) -> Result<Runner, ApiError> {
    task::spawn_blocking(move || {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Compiler::new(
                &options.work_dir,
                &options.modelsim_path,
                CompilerCommand::Vlog,
            )
            .enable_system_verilog()
            .set_work(&options.work_lib)
            .add_dependencies(&options.deps)
            .run()
            .map_err(|e| ApiError::Internal(format!("Compilation failed: {}", e)))?;

            let args = vec!["-work", &options.work_lib, &options.top];
            let mut runner = Runner::new(&options.vsim_bin, &args, &options.work_dir);
            runner
                .wait_until_prompt_startup()
                .map_err(|e| ApiError::Internal(format!("Failed to start simulator: {}", e)))?;

            Ok(runner)
        }))
    })
    .await
    .map_err(|e| ApiError::Internal(format!("Task panicked: {:?}", e)))
    .and_then(|res| res.map_err(|_e| ApiError::Internal("Failed to spawn session".to_string())))?
}
