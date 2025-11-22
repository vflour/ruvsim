use super::sim_types::{LineType, ParsedLine, ParsedPrompt, SimError};
use derive_getters::Getters;
use std::io::BufReader;
use std::io::Read;
use std::process::ChildStdout;
use std::sync::mpsc::{self, channel};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug)]
enum ParserState {
    Running,
    Stopped,
}

#[derive(Getters, Debug)]
pub struct Parser {
    // stdout: BufReader<ChildStdout>,
    #[getter(skip)]
    thread: Option<thread::JoinHandle<()>>,

    #[getter(skip)]
    rx: mpsc::Receiver<String>,

    #[getter(skip)]
    state: Arc<Mutex<ParserState>>,

    #[getter(rename = "parsed_buffer")]
    _parsed_buffer: Vec<ParsedLine>,

    #[getter(rename = "latest_buffer")]
    _latest_buffer: Vec<usize>,
}

const MAX_RETRIES: i32 = 5;
const READ_TIMEOUT_MS: Duration = Duration::from_millis(10);

impl Parser {
    fn parse_chunk(chunk: &str) -> Vec<&str> {
        chunk
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect()
    }

    /// Public helper for tests: parse a chunk into owned strings.
    #[allow(dead_code)]
    pub fn parse_chunk_lines(chunk: &str) -> Vec<String> {
        Parser::parse_chunk(chunk)
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Public helper constructor for tests: accept any reader that implements Read + Send + 'static.
    #[allow(dead_code)]
    pub fn new_with_reader<R: Read + Send + 'static>(reader: R) -> Self {
        let (tx, rx) = channel();
        let parser_state = Arc::new(Mutex::new(ParserState::Running));
        let parser_clone = parser_state.clone();

        let buf_reader = BufReader::new(reader);

        // Create a thread that reads from the reader using poll/select on the FD
        let thread: thread::JoinHandle<()> = thread::spawn(move || {
            let mut reader = buf_reader;
            let state = parser_clone;

            // Enter loop to read from stdout
            while matches!(*state.lock().unwrap(), ParserState::Running) {
                let mut buf = [0u8; 4096];
                // Read by chunks
                match reader.read(&mut buf) {
                    Err(_) => break,
                    Ok(0) => break,
                    Ok(n) => {
                        // And then parse lines from the chunk
                        let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                        let parsed_lines = Parser::parse_chunk(&chunk);
                        for line in parsed_lines {
                            if tx.send(line.to_string()).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Parser {
            thread: Some(thread),
            rx: rx,
            state: parser_state,
            _parsed_buffer: Vec::new(),
            _latest_buffer: Vec::new(),
        }
    }

    pub fn new(stdout: BufReader<ChildStdout>) -> Self {
        let (tx, rx) = channel();
        let parser_state = Arc::new(Mutex::new(ParserState::Running));
        let parser_clone = parser_state.clone();

        // Create a thread that reads from the reader using poll/select on the FD
        let thread: thread::JoinHandle<()> = thread::spawn(move || {
            let mut reader = stdout;
            let state = parser_clone;

            // Enter loop to read from stdout
            while matches!(*state.lock().unwrap(), ParserState::Running) {
                let mut buf = [0u8; 4096];
                // Read by chunks
                match reader.read(&mut buf) {
                    Err(_) => break,
                    Ok(0) => break,
                    Ok(n) => {
                        // And then parse lines from the chunk
                        let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                        let parsed_lines = Parser::parse_chunk(&chunk);
                        for line in parsed_lines {
                            if tx.send(line.to_string()).is_err() {
                                break;
                            }
                        }
                    }
                }
            }
        });

        Parser {
            thread: Some(thread),
            rx: rx,
            state: parser_state,
            _parsed_buffer: Vec::new(),
            _latest_buffer: Vec::new(),
        }
    }

    pub fn get_next_prompt(&mut self) -> Result<Option<ParsedPrompt>, SimError> {
        let timeout = Instant::now() + Duration::from_millis(100);
        self._latest_buffer.clear();
        while Instant::now() < timeout {
            let has_new_line = self.get_next_line()?;
            if has_new_line {
                if let Some(parsed_line) = self._parsed_buffer.last() {
                    if parsed_line.prompt_info.is_some() {
                        return Ok(parsed_line.prompt_info.clone());
                    }
                    if matches!(parsed_line.line_type, LineType::Exit) {
                        return Ok(None);
                    }
                }
            }
        }
        return Ok(None);
    }

    pub fn get_next_line(&mut self) -> Result<bool, SimError> {
        for _ in 0..MAX_RETRIES {
            let received = self.rx.recv_timeout(READ_TIMEOUT_MS);
            match received {
                Ok(line) => {
                    // Received a line from parser thread; pass to parsing logic
                    let parsed_line = self.parse(&line);
                    self._parsed_buffer.push(parsed_line);
                    let idx = self._parsed_buffer.len() - 1;
                    self._latest_buffer.push(idx);

                    return Ok(true);
                }
                _ => {
                    // No data yet, retry
                    continue;
                }
            }
        }
        Ok(false)
    }

    fn parse(&mut self, line: &str) -> ParsedLine {
        ParsedLine::new(&line)
    }
}

impl Drop for Parser {
    fn drop(&mut self) {
        *self.state.lock().unwrap() = ParserState::Stopped;
        // Join the thread to ensure it finishes before dropping the parser.
        let _ = self.thread.take().unwrap().join();
    }
}
