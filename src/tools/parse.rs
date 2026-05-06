use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tx3_lang::parsing;

use crate::diagnostics::{from_miette, Diagnostic};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ParseRequest {
    /// Tx3 source code to parse.
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct ParseResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ast: Option<Value>,
    pub errors: Vec<Diagnostic>,
}

pub fn run(req: ParseRequest) -> ParseResponse {
    match parsing::parse_string(&req.source) {
        Ok(program) => {
            let ast = serde_json::to_value(&program).ok();
            ParseResponse {
                ok: true,
                ast,
                errors: Vec::new(),
            }
        }
        Err(err) => {
            let diag = from_miette(&err, Some(&req.source), None);
            ParseResponse {
                ok: false,
                ast: None,
                errors: vec![diag],
            }
        }
    }
}
