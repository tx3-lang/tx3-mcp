//! `tx3_invoke` — resolve-only.
//!
//! Reads `trix.toml` from `project_dir`, builds the TII in-process from the
//! project's main `.tx3` file, picks the TRP endpoint from the named profile,
//! and posts to it via `tx3-sdk`. Returns the unsigned tx CBOR + hash.
//!
//! Does NOT sign or submit in v1. The agent / user is expected to take the
//! `tx_hex` and sign+submit elsewhere (e.g. `trix invoke`).

use std::collections::HashMap;
use std::path::PathBuf;

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tx3_lang::Workspace;
use tx3_sdk::{
    facade::{Party, Tx3Client},
    tii::Protocol,
    trp::{Client as TrpClient, ClientOptions},
};

use crate::diagnostics::{from_miette, Diagnostic};
use crate::tii_emit::{build_tii_value, ProtocolMeta};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InvokeRequest {
    /// Absolute path to the trix project directory (must contain `trix.toml`).
    pub project_dir: String,
    /// Name of the `tx <name>` block to invoke. SDK requires this explicitly —
    /// no inference from prompts like cshell does.
    pub tx_name: String,
    /// JSON object of arguments. Same shape `trix invoke --args-json` accepts.
    pub args: Value,
    /// Optional profile name. Defaults to the first profile defined in
    /// `trix.toml`, then to `local` if none are defined.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Optional party_name → bech32-address bindings. Use this when a
    /// transaction reads party addresses at resolve-time. Keys are lowercased.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parties: Option<HashMap<String, String>>,
}

#[derive(Debug, Serialize)]
pub struct InvokeResponse {
    pub ok: bool,
    /// Hex-encoded CBOR of the resolved (unsigned) transaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hex: Option<String>,
    /// 32-byte tx hash, hex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Parse / analyze diagnostics from `main.tx3`. Empty when the source is clean.
    pub diagnostics: Vec<Diagnostic>,
    /// Configuration / SDK / TRP errors (anything that isn't a tx3-source diagnostic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn run(req: InvokeRequest) -> InvokeResponse {
    if !req.args.is_object() {
        return err_only("`args` must be a JSON object");
    }

    let project_dir = PathBuf::from(&req.project_dir);
    let trix_toml_path = project_dir.join("trix.toml");

    let trix_toml: toml::Value = match std::fs::read_to_string(&trix_toml_path)
        .map_err(|e| format!("failed to read {}: {e}", trix_toml_path.display()))
        .and_then(|s| toml::from_str(&s).map_err(|e| format!("invalid trix.toml: {e}")))
    {
        Ok(v) => v,
        Err(msg) => return err_only(msg),
    };

    let profile_name = match resolve_profile_name(&trix_toml, req.profile.as_deref()) {
        Ok(name) => name,
        Err(msg) => return err_only(msg),
    };

    let trp_url = match extract_trp_url(&trix_toml, &profile_name) {
        Ok(url) => url,
        Err(msg) => return err_only(msg),
    };

    let main_relative = trix_toml
        .get("protocol")
        .and_then(|p| p.get("main"))
        .and_then(|m| m.as_str())
        .unwrap_or("main.tx3");
    let main_path = project_dir.join(main_relative);
    let meta = protocol_meta_from(&trix_toml);

    // All workspace work happens in a sync helper so `Workspace` (which holds
    // Rc<Scope>, not Send) never crosses the resolve().await below.
    let tii_json = match build_tii_for(&main_path, &meta) {
        Ok(v) => v,
        Err(BuildErr::Diagnostic(d)) => {
            return InvokeResponse {
                ok: false,
                tx_hex: None,
                hash: None,
                diagnostics: vec![*d],
                error: None,
            };
        }
        Err(BuildErr::Other(msg)) => return err_only(msg),
    };

    let protocol = match Protocol::from_json(tii_json) {
        Ok(p) => p,
        Err(e) => return err_only(format!("Protocol::from_json failed: {e}")),
    };

    let trp = TrpClient::new(ClientOptions {
        endpoint: trp_url,
        headers: None,
    });

    let mut client = Tx3Client::new(protocol, trp).with_profile(&profile_name);
    if let Some(parties) = &req.parties {
        for (name, addr) in parties {
            client = client.with_party(name, Party::address(addr));
        }
    }

    let args_map = arg_map_from(&req.args);
    match client.tx(&req.tx_name).args(args_map).resolve().await {
        Ok(resolved) => InvokeResponse {
            ok: true,
            tx_hex: Some(resolved.tx_hex),
            hash: Some(resolved.hash),
            diagnostics: Vec::new(),
            error: None,
        },
        Err(e) => err_only(format!("resolve failed: {e}")),
    }
}

enum BuildErr {
    Diagnostic(Box<Diagnostic>),
    Other(String),
}

fn build_tii_for(main_path: &std::path::Path, meta: &ProtocolMeta) -> Result<Value, BuildErr> {
    let source = std::fs::read_to_string(main_path)
        .map_err(|e| BuildErr::Other(format!("failed to read {}: {e}", main_path.display())))?;

    let mut ws = Workspace::from_string(source.clone());
    let path_str = main_path.display().to_string();

    if let Err(e) = ws.parse() {
        return Err(BuildErr::Diagnostic(Box::new(from_miette(
            &e,
            Some(&source),
            Some(&path_str),
        ))));
    }
    if let Err(e) = ws.analyze() {
        return Err(BuildErr::Diagnostic(Box::new(from_miette(
            &e,
            Some(&source),
            Some(&path_str),
        ))));
    }
    if let Err(e) = ws.lower() {
        return Err(BuildErr::Diagnostic(Box::new(from_miette(
            &e,
            Some(&source),
            Some(&path_str),
        ))));
    }

    build_tii_value(&ws, meta).map_err(|e| BuildErr::Other(format!("TII emission failed: {e}")))
}

fn resolve_profile_name(
    trix_toml: &toml::Value,
    requested: Option<&str>,
) -> Result<String, String> {
    if let Some(name) = requested {
        return Ok(name.to_string());
    }

    let profiles = trix_toml.get("profile").and_then(|p| p.as_table());
    if let Some(table) = profiles {
        if let Some((name, _)) = table.iter().next() {
            return Ok(name.clone());
        }
    }
    Ok("local".to_string())
}

fn extract_trp_url(trix_toml: &toml::Value, profile_name: &str) -> Result<String, String> {
    let url = trix_toml
        .get("profile")
        .and_then(|p| p.get(profile_name))
        .and_then(|p| p.get("trp"))
        .and_then(|t| t.get("url"))
        .and_then(|u| u.as_str());
    match url {
        Some(s) => Ok(s.to_string()),
        None => Err(format!(
            "trix.toml is missing [profile.{profile_name}.trp].url; the SDK needs a TRP endpoint to resolve transactions"
        )),
    }
}

fn protocol_meta_from(trix_toml: &toml::Value) -> ProtocolMeta {
    let p = trix_toml.get("protocol");
    let s = |key: &str| {
        p.and_then(|t| t.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    ProtocolMeta {
        scope: s("scope").unwrap_or_else(|| "unknown".to_string()),
        name: s("name").unwrap_or_else(|| "unknown".to_string()),
        version: s("version").unwrap_or_else(|| "0.0.1".to_string()),
        description: s("description"),
    }
}

fn arg_map_from(args: &Value) -> tx3_sdk::core::ArgMap {
    let mut out = tx3_sdk::core::ArgMap::new();
    if let Some(obj) = args.as_object() {
        for (k, v) in obj {
            out.insert(k.clone(), v.clone());
        }
    }
    out
}

fn err_only(msg: impl Into<String>) -> InvokeResponse {
    InvokeResponse {
        ok: false,
        tx_hex: None,
        hash: None,
        diagnostics: Vec::new(),
        error: Some(msg.into()),
    }
}
