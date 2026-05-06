use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tx3_lang::Workspace;

use crate::{args::args_from_json, diagnostics::Diagnostic};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ApplyArgsRequest {
    /// Tx3 source code.
    pub source: String,
    /// Name of the `tx <name>` block.
    pub tx_name: String,
    /// JSON object mapping arg names to values. Strings starting with `0x` are
    /// treated as hex bytes; other strings are passed through. Numbers must fit
    /// in i64. Arrays/objects/nulls are rejected.
    pub args: Value,
}

#[derive(Debug, Serialize)]
pub struct ApplyArgsResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tir: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn run(req: ApplyArgsRequest) -> ApplyArgsResponse {
    let args = match args_from_json(&req.args) {
        Ok(a) => a,
        Err(e) => {
            return ApplyArgsResponse {
                ok: false,
                tir: None,
                diagnostics: Vec::new(),
                error: Some(format!("invalid args: {e}")),
            }
        }
    };

    let mut workspace = Workspace::from_string(req.source.clone());

    if let Err(e) = workspace.parse() {
        return ApplyArgsResponse {
            ok: false,
            tir: None,
            diagnostics: vec![crate::diagnostics::from_miette(&e, Some(&req.source), None)],
            error: None,
        };
    }

    if let Err(e) = workspace.analyze() {
        return ApplyArgsResponse {
            ok: false,
            tir: None,
            diagnostics: vec![crate::diagnostics::from_miette(&e, Some(&req.source), None)],
            error: None,
        };
    }

    if let Err(e) = workspace.lower() {
        return ApplyArgsResponse {
            ok: false,
            tir: None,
            diagnostics: vec![crate::diagnostics::from_miette(&e, Some(&req.source), None)],
            error: None,
        };
    }

    if let Err(e) = workspace.apply_args(&args) {
        return ApplyArgsResponse {
            ok: false,
            tir: None,
            diagnostics: vec![crate::diagnostics::from_miette(&e, Some(&req.source), None)],
            error: None,
        };
    }

    let tir = workspace.tir(&req.tx_name);
    match tir {
        Some(t) => ApplyArgsResponse {
            ok: true,
            tir: serde_json::to_value(t).ok(),
            diagnostics: Vec::new(),
            error: None,
        },
        None => ApplyArgsResponse {
            ok: false,
            tir: None,
            diagnostics: Vec::new(),
            error: Some(format!("transaction `{}` not found in source", req.tx_name)),
        },
    }
}
