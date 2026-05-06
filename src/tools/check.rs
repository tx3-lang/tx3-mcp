use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tx3_lang::{analyzing, parsing};

use crate::diagnostics::{from_miette, Diagnostic};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckRequest {
    /// Tx3 source code. Provide either `source` or `path`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Filesystem path to a .tx3 file. Used for source_path on diagnostics.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckResponse {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn run(req: CheckRequest) -> anyhow::Result<CheckResponse> {
    let source = match (&req.source, &req.path) {
        (Some(s), _) => s.clone(),
        (None, Some(p)) => std::fs::read_to_string(p)?,
        (None, None) => anyhow::bail!("must provide either `source` or `path`"),
    };

    let diagnostics = check_source(&source, req.path.as_deref());
    let ok = diagnostics
        .iter()
        .all(|d| !matches!(d.severity, crate::diagnostics::Severity::Error));

    Ok(CheckResponse { ok, diagnostics })
}

fn check_source(source: &str, path: Option<&str>) -> Vec<Diagnostic> {
    let mut program = match parsing::parse_string(source) {
        Ok(p) => p,
        Err(e) => return vec![from_miette(&e, Some(source), path)],
    };

    let report = analyzing::analyze(&mut program);
    report
        .errors
        .iter()
        .map(|err| from_miette(err, Some(source), path))
        .collect()
}
