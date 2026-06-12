//! In-memory TII (Transaction Invocation Interface) builder.
//!
//! Adapted from `tx3/bin/tx3c/src/tii/{mod.rs,types.rs}` (TII v1beta0).
//! That code lives in the `tx3c` binary crate and isn't published as a
//! library, so we duplicate the assembly logic here. Drift risk: when
//! upstream `emit_tii` changes, sync this module.
//!
//! Differences from the upstream version:
//!  - returns a `serde_json::Value` instead of writing a file to disk
//!  - takes a small `ProtocolMeta` struct in lieu of the `tx3c` CLI `Args`
//!  - drops dotfile/profile parsing — emits a single empty `local` profile
//!    (the SDK reads env values from `args` passed at `.resolve()` time, not
//!    from TII profiles, so this is sufficient for v1)

use std::collections::HashMap;

use anyhow::anyhow;
use schemars::Schema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tx3_lang::{ast, Workspace};

const TII_VERSION: &str = "v1beta0";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TiiFile {
    tii: TiiInfo,
    protocol: Protocol,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<Schema>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    parties: HashMap<String, Party>,
    transactions: HashMap<String, Transaction>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    profiles: HashMap<String, Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TiiInfo {
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Protocol {
    scope: String,
    name: String,
    version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
enum BytesEncoding {
    #[allow(dead_code)]
    Base64,
    Hex,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct TirEnvelope {
    content: String,
    encoding: BytesEncoding,
    version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Transaction {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    tir: TirEnvelope,
    params: Schema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Party {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Profile {
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    environment: Value,
    parties: Value,
}

/// Protocol metadata for the emitted TII. Caller fills this from `trix.toml`'s
/// `[protocol]` table.
#[derive(Debug, Clone)]
pub struct ProtocolMeta {
    pub scope: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

impl Default for ProtocolMeta {
    fn default() -> Self {
        Self {
            scope: "unknown".to_string(),
            name: "unknown".to_string(),
            version: "0.0.1".to_string(),
            description: None,
        }
    }
}

fn map_ast_type_to_json_schema(r#type: &ast::Type) -> Value {
    match r#type {
        ast::Type::Int => json!({"type": "integer"}),
        ast::Type::Bool => json!({"type": "boolean"}),
        ast::Type::Bytes => json!({ "$ref": "https://tx3.land/specs/v1beta0/core#Bytes" }),
        ast::Type::Address => json!({ "$ref": "https://tx3.land/specs/v1beta0/core#Address" }),
        ast::Type::UtxoRef => json!({ "$ref": "https://tx3.land/specs/v1beta0/core#UtxoRef" }),
        ast::Type::Unit => json!({"type": "null"}),
        ast::Type::List(inner) => json!({
            "type": "array",
            "items": map_ast_type_to_json_schema(inner)
        }),
        ast::Type::Map(_, value) => json!({
            "type": "object",
            "additionalProperties": map_ast_type_to_json_schema(value)
        }),
        // A tuple is a fixed-length, positionally-typed array.
        ast::Type::Tuple(elements) => json!({
            "type": "array",
            "prefixItems": elements
                .iter()
                .map(map_ast_type_to_json_schema)
                .collect::<Vec<_>>(),
            "items": false,
            "minItems": elements.len(),
            "maxItems": elements.len()
        }),
        ast::Type::Custom(_) => json!({"type": "object"}),
        ast::Type::Undefined => json!({"type": "null"}),
        ast::Type::Utxo => json!({ "$ref": "https://tx3.land/specs/v1beta0/core#Utxo" }),
        ast::Type::AnyAsset => json!({ "$ref": "https://tx3.land/specs/v1beta0/core#AnyAsset" }),
    }
}

fn infer_env_schema(ast: &ast::Program) -> Schema {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    if let Some(env) = &ast.env {
        for field in env.fields.iter() {
            let field_schema = map_ast_type_to_json_schema(&field.r#type);
            properties.insert(field.name.clone(), field_schema);
            required.push(field.name.clone());
        }
    }

    let schema_json = json!({
        "type": "object",
        "properties": properties,
        "required": required
    });

    serde_json::from_value(schema_json)
        .unwrap_or_else(|_| serde_json::from_value(json!({"type": "object"})).unwrap())
}

fn infer_tx_params_schema(tx: &ast::TxDef) -> Schema {
    let mut properties = serde_json::Map::new();
    let mut required = Vec::new();

    for param in tx.parameters.parameters.iter() {
        let field_schema = map_ast_type_to_json_schema(&param.r#type);
        properties.insert(param.name.value.clone(), field_schema);
        required.push(param.name.value.clone());
    }

    let schema_json = json!({
        "type": "object",
        "properties": properties,
        "required": required
    });

    serde_json::from_value(schema_json)
        .unwrap_or_else(|_| serde_json::from_value(json!({"type": "object"})).unwrap())
}

/// Build a TII document in memory from a fully-lowered Workspace and
/// caller-supplied protocol metadata. Returns the JSON value the SDK's
/// `Protocol::from_json` expects.
pub fn build_tii_value(ws: &Workspace, meta: &ProtocolMeta) -> anyhow::Result<Value> {
    let ast = ws.ast().ok_or_else(|| anyhow!("workspace has no AST"))?;

    let mut tii = TiiFile {
        tii: TiiInfo {
            version: TII_VERSION.to_string(),
        },
        protocol: Protocol {
            scope: meta.scope.clone(),
            name: meta.name.clone(),
            version: meta.version.clone(),
            description: meta.description.clone(),
        },
        environment: Some(infer_env_schema(ast)),
        parties: HashMap::new(),
        transactions: HashMap::new(),
        profiles: HashMap::new(),
    };

    for party in ast.parties.iter() {
        tii.parties
            .insert(party.name.value.to_lowercase(), Party { description: None });
    }

    for tx in ast.txs.iter() {
        let tir = ws
            .tir(&tx.name.value)
            .ok_or_else(|| anyhow!("missing TIR for tx `{}`", tx.name.value))?;

        let (bytes, version) = tx3_tir::encoding::to_bytes(tir);
        let hex_string = hex::encode(&bytes);

        tii.transactions.insert(
            tx.name.value.clone(),
            Transaction {
                description: None,
                tir: TirEnvelope {
                    content: hex_string,
                    encoding: BytesEncoding::Hex,
                    version: version.to_string(),
                },
                params: infer_tx_params_schema(tx),
            },
        );
    }

    // The TII format expects at least a "local" profile; populate an empty one
    // so `Protocol::from_json` is happy. Real profile data flows in via the
    // SDK's `.with_profile()` + args at resolve-time.
    tii.profiles.insert(
        "local".to_string(),
        Profile {
            description: None,
            environment: json!({}),
            parties: json!({}),
        },
    );

    Ok(json!(tii))
}
