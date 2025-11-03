pub mod sim_parser_step_defs;
pub mod sim_runner_step_defs;
use cucumber::World;
use sim_parser_step_defs::SimParserWorld;
use sim_runner_step_defs::SimRunnerWorld;

pub fn main() {
    futures::executor::block_on(async {
        SimParserWorld::run("test/features/parser.feature").await;
        SimRunnerWorld::run("test/features/runner.feature").await;
    });
}
