use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tx3_lang::{analyzing, lowering, parsing};

#[allow(unused_imports)]
use crate::diagnostics::Severity;
use crate::diagnostics::{from_miette, Diagnostic};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LowerRequest {
    /// Tx3 source code.
    pub source: String,
    /// Name of the `tx <name>` block to lower.
    pub tx_name: String,
}

#[derive(Debug, Serialize)]
pub struct LowerResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tir: Option<Value>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn run(req: LowerRequest) -> LowerResponse {
    let mut program = match parsing::parse_string(&req.source) {
        Ok(p) => p,
        Err(e) => {
            return LowerResponse {
                ok: false,
                tir: None,
                diagnostics: vec![from_miette(&e, Some(&req.source), None)],
            }
        }
    };

    let report = analyzing::analyze(&mut program);
    if !report.errors.is_empty() {
        return LowerResponse {
            ok: false,
            tir: None,
            diagnostics: report
                .errors
                .iter()
                .map(|e| from_miette(e, Some(&req.source), None))
                .collect(),
        };
    }

    match lowering::lower(&program, &req.tx_name) {
        Ok(tir) => LowerResponse {
            ok: true,
            tir: serde_json::to_value(&tir).ok(),
            diagnostics: Vec::new(),
        },
        Err(e) => LowerResponse {
            ok: false,
            tir: None,
            diagnostics: vec![Diagnostic {
                severity: crate::diagnostics::Severity::Error,
                code: Some("tx3::lowering".to_string()),
                message: e.to_string(),
                help: None,
                url: None,
                source_path: None,
                spans: Vec::new(),
                related: Vec::new(),
            }],
        },
    }
}
