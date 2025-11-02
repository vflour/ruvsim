pub mod sim_parser_step_defs;
use cucumber::World;
use sim_parser_step_defs::SimParserWorld;

pub fn main() {
    futures::executor::block_on(SimParserWorld::run("test/features/parser.feature"));
}
