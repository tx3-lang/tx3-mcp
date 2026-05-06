//! End-to-end smoke test: launch the `tx3-mcp` binary, drive it over stdio
//! using the MCP JSON-RPC framing, and assert that tools are listed and
//! callable.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn drive(messages: &[&str]) -> Vec<serde_json::Value> {
    let exe = env!("CARGO_BIN_EXE_tx3-mcp");
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn tx3-mcp");

    let stdin = child.stdin.as_mut().expect("stdin");
    for m in messages {
        writeln!(stdin, "{m}").unwrap();
    }
    drop(child.stdin.take());

    let stdout = child.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);

    let mut responses = Vec::new();
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(&line).expect("parse json");
        responses.push(v);
    }

    let _ = child.wait();
    responses
}

const INIT: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"smoke","version":"0"}}}"#;
const INITED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const LIST: &str = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

#[test]
fn lists_eight_tools() {
    let responses = drive(&[INIT, INITED, LIST]);
    let list = responses
        .iter()
        .find(|r| r.get("id").map(|i| i.as_i64() == Some(2)).unwrap_or(false))
        .expect("tools/list response");

    let tools = list
        .pointer("/result/tools")
        .and_then(|t| t.as_array())
        .expect("tools array");

    let names: Vec<_> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();

    let expected = [
        "tx3_parse",
        "tx3_check",
        "tx3_lower",
        "tx3_apply_args",
        "tx3_inspect_project",
        "tx3_invoke",
        "tx3_examples_list",
        "tx3_example_get",
    ];
    for tool in expected {
        assert!(names.contains(&tool), "missing tool: {tool}; got {names:?}");
    }
}

#[test]
fn parse_returns_ast_for_valid_source() {
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "tx3_parse",
            "arguments": { "source": "tx swap() {}" }
        }
    });
    let responses = drive(&[INIT, INITED, &call.to_string()]);
    let resp = responses
        .iter()
        .find(|r| r.get("id").map(|i| i.as_i64() == Some(3)).unwrap_or(false))
        .expect("call response");

    let text = resp
        .pointer("/result/content/0/text")
        .and_then(|t| t.as_str())
        .expect("text content");
    let body: serde_json::Value = serde_json::from_str(text).expect("nested json");
    assert_eq!(body["ok"], serde_json::Value::Bool(true));
    assert!(body["ast"].is_object(), "expected ast in response");
}

#[test]
fn check_surfaces_diagnostics_for_broken_source() {
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "tools/call",
        "params": {
            "name": "tx3_check",
            "arguments": { "source": "tx oops( {" }
        }
    });
    let responses = drive(&[INIT, INITED, &call.to_string()]);
    let resp = responses
        .iter()
        .find(|r| r.get("id").map(|i| i.as_i64() == Some(4)).unwrap_or(false))
        .expect("call response");

    let text = resp
        .pointer("/result/content/0/text")
        .and_then(|t| t.as_str())
        .expect("text content");
    let body: serde_json::Value = serde_json::from_str(text).expect("nested json");
    assert_eq!(body["ok"], serde_json::Value::Bool(false));
    let diags = body["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty(), "expected at least one diagnostic");
}

#[test]
fn examples_list_returns_ten_entries() {
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "tools/call",
        "params": { "name": "tx3_examples_list", "arguments": {} }
    });
    let responses = drive(&[INIT, INITED, &call.to_string()]);
    let resp = responses
        .iter()
        .find(|r| r.get("id").map(|i| i.as_i64() == Some(5)).unwrap_or(false))
        .expect("call response");

    let text = resp
        .pointer("/result/content/0/text")
        .and_then(|t| t.as_str())
        .expect("text content");
    let body: serde_json::Value = serde_json::from_str(text).expect("nested json");
    let examples = body["examples"].as_array().expect("examples array");
    assert_eq!(examples.len(), 10);
}
