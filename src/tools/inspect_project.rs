use std::path::{Path, PathBuf};

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tx3_lang::Workspace;

use crate::diagnostics::{from_miette, Diagnostic};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InspectProjectRequest {
    /// Absolute path to a directory containing `trix.toml`.
    pub project_dir: String,
}

#[derive(Debug, Serialize)]
pub struct InspectProjectResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trix_toml: Option<toml::Value>,
    pub txs: Vec<TxSummary>,
    pub parties: Vec<String>,
    pub assets: Vec<String>,
    pub policies: Vec<String>,
    pub functions: Vec<FnSummary>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FnSummary {
    pub name: String,
    pub params: Vec<TxParam>,
    pub return_type: String,
    /// True for compiler-provided built-ins (e.g. `min_utxo`), false for
    /// user-defined functions declared in source.
    pub builtin: bool,
}

#[derive(Debug, Serialize)]
pub struct TxSummary {
    pub name: String,
    pub params: Vec<TxParam>,
    pub input_count: usize,
    pub output_count: usize,
    pub mint_count: usize,
    pub burn_count: usize,
    pub references_count: usize,
    pub has_validity: bool,
    pub has_signers: bool,
    pub has_metadata: bool,
    pub cardano_blocks: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct TxParam {
    pub name: String,
    pub r#type: String,
}

pub fn run(req: InspectProjectRequest) -> InspectProjectResponse {
    let dir = PathBuf::from(&req.project_dir);
    let trix_toml_path = dir.join("trix.toml");

    let toml_str = match std::fs::read_to_string(&trix_toml_path) {
        Ok(s) => s,
        Err(e) => {
            return InspectProjectResponse {
                ok: false,
                trix_toml: None,
                txs: Vec::new(),
                parties: Vec::new(),
                assets: Vec::new(),
                policies: Vec::new(),
            functions: Vec::new(),
                diagnostics: Vec::new(),
                error: Some(format!("failed to read {}: {e}", trix_toml_path.display())),
            }
        }
    };

    let trix_toml: toml::Value = match toml::from_str(&toml_str) {
        Ok(v) => v,
        Err(e) => {
            return InspectProjectResponse {
                ok: false,
                trix_toml: None,
                txs: Vec::new(),
                parties: Vec::new(),
                assets: Vec::new(),
                policies: Vec::new(),
            functions: Vec::new(),
                diagnostics: Vec::new(),
                error: Some(format!("invalid trix.toml: {e}")),
            }
        }
    };

    let main_relative = trix_toml
        .get("protocol")
        .and_then(|p| p.get("main"))
        .and_then(|m| m.as_str())
        .unwrap_or("main.tx3");
    let main_path = dir.join(main_relative);

    let source = match std::fs::read_to_string(&main_path) {
        Ok(s) => s,
        Err(e) => {
            return InspectProjectResponse {
                ok: false,
                trix_toml: Some(trix_toml),
                txs: Vec::new(),
                parties: Vec::new(),
                assets: Vec::new(),
                policies: Vec::new(),
            functions: Vec::new(),
                diagnostics: Vec::new(),
                error: Some(format!("failed to read {}: {e}", main_path.display())),
            }
        }
    };

    summarise(&source, &main_path, trix_toml)
}

fn summarise(source: &str, main_path: &Path, trix_toml: toml::Value) -> InspectProjectResponse {
    let mut workspace = Workspace::from_string(source.to_string());

    if let Err(e) = workspace.parse() {
        return InspectProjectResponse {
            ok: false,
            trix_toml: Some(trix_toml),
            txs: Vec::new(),
            parties: Vec::new(),
            assets: Vec::new(),
            policies: Vec::new(),
            functions: Vec::new(),
            diagnostics: vec![from_miette(
                &e,
                Some(source),
                Some(&main_path.display().to_string()),
            )],
            error: None,
        };
    }
    let _ = workspace.analyze();

    let analyze_diags = workspace
        .analisis()
        .map(|r| {
            r.errors
                .iter()
                .map(|e| from_miette(e, Some(source), Some(&main_path.display().to_string())))
                .collect()
        })
        .unwrap_or_default();

    let ast = match workspace.ast() {
        Some(a) => a,
        None => {
            return InspectProjectResponse {
                ok: false,
                trix_toml: Some(trix_toml),
                txs: Vec::new(),
                parties: Vec::new(),
                assets: Vec::new(),
                policies: Vec::new(),
            functions: Vec::new(),
                diagnostics: analyze_diags,
                error: Some("workspace has no AST".to_string()),
            }
        }
    };

    let parties = ast.parties.iter().map(|p| p.name.value.clone()).collect();
    let assets = ast.assets.iter().map(|a| a.name.value.clone()).collect();
    let policies = ast.policies.iter().map(|p| p.name.value.clone()).collect();

    let functions = ast
        .functions
        .iter()
        .map(|f| FnSummary {
            name: f.name.value.clone(),
            params: f
                .parameters
                .parameters
                .iter()
                .map(|p| TxParam {
                    name: p.name.value.clone(),
                    r#type: format!("{:?}", p.r#type),
                })
                .collect(),
            return_type: format!("{:?}", f.return_type),
            builtin: f.builtin.is_some(),
        })
        .collect();

    let txs = ast
        .txs
        .iter()
        .map(|tx| TxSummary {
            name: tx.name.value.clone(),
            params: tx
                .parameters
                .parameters
                .iter()
                .map(|p| TxParam {
                    name: p.name.value.clone(),
                    r#type: format!("{:?}", p.r#type),
                })
                .collect(),
            input_count: tx.inputs.len(),
            output_count: tx.outputs.len(),
            mint_count: tx.mints.len(),
            burn_count: tx.burns.len(),
            references_count: tx.references.len(),
            has_validity: tx.validity.is_some(),
            has_signers: tx.signers.is_some(),
            has_metadata: tx.metadata.is_some(),
            cardano_blocks: tx
                .adhoc
                .iter()
                .map(|b| {
                    let dbg = format!("{b:?}");
                    dbg.split('(').next().unwrap_or("Unknown").to_string()
                })
                .collect(),
        })
        .collect();

    InspectProjectResponse {
        ok: true,
        trix_toml: Some(trix_toml),
        txs,
        parties,
        assets,
        policies,
        functions,
        diagnostics: analyze_diags,
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_user_defined_functions() {
        let source = r#"
party Sender;

fn double(x: Int) -> Int {
    let twice = x + x;
    twice
}

tx pay(quantity: Int) {
    output {
        to: Sender,
        amount: double(quantity),
    }
}
"#;
        let resp = summarise(
            source,
            Path::new("main.tx3"),
            toml::Value::Table(Default::default()),
        );

        assert!(resp.ok, "expected a clean summary, got {:?}", resp.error);
        assert_eq!(resp.functions.len(), 1);

        let func = &resp.functions[0];
        assert_eq!(func.name, "double");
        assert!(!func.builtin);
        assert_eq!(func.params.len(), 1);
        assert_eq!(func.params[0].name, "x");
    }
}
