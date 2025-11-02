use ruvsim::{sim_parser, sim_types};
use sim_parser::Parser;
use sim_types::LineType;
use std::io::Cursor;

#[test]
fn parse_chunk_trim_and_filter() {
    let chunk = "  line1\n\nline2  \n   \n#comment\n";
    let lines = Parser::parse_chunk_lines(chunk);
    assert_eq!(
        lines,
        vec![
            "line1".to_string(),
            "line2".to_string(),
            "#comment".to_string()
        ]
    );
}

#[test]
fn new_with_reader_reads_lines_and_classifies() {
    let data = "Hello world\n# comment\nVSIM 5>\n";
    let cursor = Cursor::new(data.as_bytes().to_vec());
    let mut parser = Parser::new_with_reader(cursor);

    // Try to collect up to 10 lines (parser.get_next_line returns false on timeout)
    let mut got = 0;
    for _ in 0..10 {
        if parser.get_next_line().unwrap() {
            got += 1;
        } else {
            break;
        }
    }

    assert!(got >= 3, "expected at least 3 lines parsed, got {}", got);

    let parsed = parser.parsed_buffer();
    assert_eq!(parsed.len(), got as usize);

    assert_eq!(parsed[0].content, "Hello world");
    assert!(matches!(parsed[1].line_type, LineType::Log));
    assert!(matches!(parsed[2].line_type, LineType::Prompt));
}
