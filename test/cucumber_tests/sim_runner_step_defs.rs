use cucumber::{given, then, when, World};
use ruvsim::sim_runner::Runner;
use std::fmt;

#[derive(Default, World)]
#[world(init = Self::new)]
pub struct SimRunnerWorld {
    pub runner: Option<Runner>,
}

impl fmt::Debug for SimRunnerWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SimRunnerWorld")
            .field("runner", &self.runner.as_ref().map(|_| "<Runner>"))
            .finish()
    }
}

impl SimRunnerWorld {
    pub fn new() -> Self {
        Self { runner: None }
    }
}

#[given("a vsim runner")]
async fn given_a_vsim_runner(world: &mut SimRunnerWorld) {
    // Use the fake vsim in test/bin, run in repo root
    let runner = Runner::new("test/bin/vsim", &vec![], ".");
    world.runner = Some(runner);
}

#[when("I wait for vsim prompt")]
async fn when_i_wait_for_vsim_prompt(world: &mut SimRunnerWorld) {
    let runner = world.runner.as_mut().expect("Runner not initialized");
    runner
        .wait_until_prompt_startup()
        .expect("Failed to detect VSIM prompt");
}

#[when(expr = "I send command {string}")]
async fn when_i_send_command(world: &mut SimRunnerWorld, cmd: String) {
    let runner = world.runner.as_mut().expect("Runner not initialized");
    runner
        .send_command(&cmd)
        .expect("Failed to send command to vsim");
}

#[then(expr = "a log line contains {string}")]
async fn then_a_log_line_contains(world: &mut SimRunnerWorld, needle: String) {
    let runner = world.runner.as_mut().expect("Runner not initialized");
    let matches = runner
        .get_log_matches(|line| line.contains(&needle))
        .expect("Failed to scan logs");
    assert!(
        !matches.is_empty(),
        "Expected to find a log containing '{}', but none matched",
        needle
    );
}

#[then("I stop the runner")]
async fn then_i_stop_the_runner(world: &mut SimRunnerWorld) {
    let runner = world.runner.as_mut().expect("Runner not initialized");
    runner.finish().expect("Failed to stop runner");
}
