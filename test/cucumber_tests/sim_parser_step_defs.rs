use cucumber::{World, given, then, when};
#[cfg(test)]
use std::io::Cursor;

use ruvsim::{sim_parser, sim_types};
use sim_parser::Parser;
use sim_types::LineType;

#[derive(Debug, Default, World)]
#[world(init = Self::new)]
pub struct SimParserWorld {
    pub parser: Option<Parser>,
}

impl SimParserWorld {
    pub fn new() -> Self {
        SimParserWorld { parser: None }
    }
}

#[given(expr = "a parser with input {string}")]
async fn given_a_parser_with_input(world: &mut SimParserWorld, inp: String) {
    // We need to replace the escaped newlines with actual newlines
    let mut input = inp.replace("\\\\n", "[DONT_ERASE_THIS]n");
    input = input.replace("\\n", "\n");
    input = input.replace("[DONT_ERASE_THIS]n", "\\n");
    let cursor = Cursor::new(input.as_bytes().to_vec());

    let parser = Parser::new_with_reader(cursor);
    world.parser = Some(parser);
}

#[when("I parse lines")]
async fn when_i_parse_lines(world: &mut SimParserWorld) {
    let parser = world.parser.as_mut().expect("Parser not initialized");
    // Read lines until get_next_line returns false or we exceed a reasonable limit
    for _ in 0..50 {
        if !parser.get_next_line().unwrap() {
            break;
        }
    }
}

#[then(expr = "buffer has {int} lines")]
async fn then_buffer_has_lines(world: &mut SimParserWorld, lines: i32) {
    let n: usize = lines as usize;
    let parser = world.parser.as_ref().expect("Parser not initialized");
    let buf = parser.parsed_buffer();
    assert_eq!(buf.len(), n, "buffer size mismatch");
}

#[then(expr = "last line content is {string}")]
async fn then_last_line_content_is(world: &mut SimParserWorld, content: String) {
    let parser = world.parser.as_ref().expect("Parser not initialized");
    let buf = parser.parsed_buffer();
    let last = buf.last().expect("no last line");
    assert_eq!(&last.content, &content, "last content mismatch");
}

#[then(expr = "last line type is {word}")]
async fn then_last_line_type_is(world: &mut SimParserWorld, _line_type: LineType) {
    let parser = world.parser.as_ref().expect("Parser not initialized");
    let buf = parser.parsed_buffer();
    let last = buf.last().expect("no last line");

    assert!(
        matches!(&last.line_type, _line_type),
        "last line type mismatch"
    );
}
