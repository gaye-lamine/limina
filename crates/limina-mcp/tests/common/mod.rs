//! A mock Limina API and a client of the `limina-mcp` binary, for the tests.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

/// A request the mock received.
#[derive(Debug, Clone)]
pub struct Recorded {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

/// A canned answer for one method and path.
#[derive(Debug, Clone)]
pub struct Route {
    pub method: &'static str,
    pub path: &'static str,
    pub status: u16,
    pub body: Value,
}

pub fn route(method: &'static str, path: &'static str, status: u16, body: Value) -> Route {
    Route {
        method,
        path,
        status,
        body,
    }
}

/// A running mock server.
pub struct Mock {
    pub url: String,
    pub seen: Arc<Mutex<Vec<Recorded>>>,
}

impl Mock {
    pub fn requests(&self) -> Vec<Recorded> {
        self.seen.lock().expect("lock").clone()
    }
}

/// Starts a mock that answers `routes` and `404` for anything else.
pub fn start(routes: Vec<Route>) -> Mock {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("addr"));
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&seen);
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let routes = routes.clone();
            let log = Arc::clone(&log);
            thread::spawn(move || serve(stream, &routes, &log));
        }
    });
    Mock { url, seen }
}

fn serve(stream: TcpStream, routes: &[Route], log: &Mutex<Vec<Recorded>>) {
    let Some(request) = read_request(&stream) else {
        return;
    };
    let answer = routes
        .iter()
        .find(|r| r.method == request.method && r.path == request.path)
        .cloned()
        .unwrap_or_else(|| {
            route(
                "",
                "",
                404,
                json!({"error": {"code": "NotFound", "message": "no such route in the mock"}}),
            )
        });
    log.lock().expect("lock").push(request);
    let body = answer.body.to_string();
    let head = format!(
        "HTTP/1.1 {} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        answer.status,
        body.len()
    );
    let mut out = &stream;
    let _ = out.write_all(head.as_bytes());
    let _ = out.write_all(body.as_bytes());
    let _ = out.flush();
}

fn read_request(stream: &TcpStream) -> Option<Recorded> {
    let mut reader = BufReader::new(stream);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let mut parts = first.split_whitespace();
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    let mut headers = Vec::new();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let line = line.trim_end().to_string();
        if line.is_empty() {
            break;
        }
        let (name, value) = line.split_once(':')?;
        if name.eq_ignore_ascii_case("content-length") {
            length = value.trim().parse().ok()?;
        }
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    Some(Recorded {
        method,
        path,
        headers,
        body: String::from_utf8_lossy(&body).to_string(),
    })
}

/// A running `limina-mcp` process, spoken to in JSON-RPC lines.
pub struct Session {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
}

impl Session {
    /// Starts the binary against `api_url`, with `extra_env` in its environment,
    /// and completes the MCP handshake.
    pub fn start(api_url: &str, extra_env: &[(&str, &str)]) -> Session {
        let mut command = Command::new(env!("CARGO_BIN_EXE_limina-mcp"));
        command
            .env("LIMINA_API_URL", api_url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (k, v) in extra_env {
            command.env(k, v);
        }
        let mut child = command.spawn().expect("spawn limina-mcp");
        let stdin = child.stdin.take().expect("stdin");
        let stdout = child.stdout.take().expect("stdout");
        let (tx, lines) = channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut session = Session {
            child,
            stdin,
            lines,
            next_id: 1,
        };
        let init = session.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": {"name": "limina-mcp-test", "version": "0"}
            }),
        );
        assert!(init.get("result").is_some(), "initialize failed: {init}");
        session.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        session
    }

    fn send(&mut self, message: &Value) {
        writeln!(self.stdin, "{message}").expect("write");
        self.stdin.flush().expect("flush");
    }

    /// Sends a request and returns the response with the same id.
    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(60))
                .expect("no answer from limina-mcp");
            let value: Value = serde_json::from_str(&line).expect("a JSON line");
            if value.get("id") == Some(&json!(id)) {
                return value;
            }
        }
    }

    /// Calls a tool and returns `(is_error, text)`.
    pub fn call(&mut self, name: &str, arguments: Value) -> (bool, String) {
        let response = self.request("tools/call", json!({"name": name, "arguments": arguments}));
        let result = response
            .get("result")
            .unwrap_or_else(|| panic!("no result: {response}"));
        let is_error = result
            .get("isError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        (is_error, text)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
