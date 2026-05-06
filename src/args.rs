use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use serde_json::Value;
use tx3_tir::reduce::ArgValue;

/// Convert a JSON object into the BTreeMap<String, ArgValue> expected by
/// `tx3_lang::Workspace::apply_args`.
///
/// Conventions:
///  - JSON numbers → `ArgValue::Int` (rejecting non-integer floats)
///  - JSON strings starting with `0x` → `ArgValue::Bytes` (hex-decoded)
///  - JSON strings → `ArgValue::String`
///  - JSON booleans → `ArgValue::Bool`
///  - JSON nulls / objects / arrays are rejected (callers must pre-shape)
pub fn args_from_json(args: &Value) -> Result<BTreeMap<String, ArgValue>> {
    let obj = args
        .as_object()
        .ok_or_else(|| anyhow!("expected `args` to be a JSON object"))?;

    let mut out = BTreeMap::new();
    for (k, v) in obj {
        out.insert(k.clone(), value_to_arg(v)?);
    }
    Ok(out)
}

fn value_to_arg(v: &Value) -> Result<ArgValue> {
    match v {
        Value::Bool(b) => Ok(ArgValue::Bool(*b)),
        Value::Number(n) => {
            let i = n
                .as_i64()
                .ok_or_else(|| anyhow!("number {n} is not a 64-bit integer"))?;
            Ok(ArgValue::Int(i as i128))
        }
        Value::String(s) => {
            if let Some(hex) = s.strip_prefix("0x") {
                let bytes = hex::decode(hex).map_err(|e| anyhow!("invalid hex bytes: {e}"))?;
                Ok(ArgValue::Bytes(bytes))
            } else {
                Ok(ArgValue::String(s.clone()))
            }
        }
        Value::Null => Err(anyhow!("null arg values are not supported")),
        Value::Array(_) | Value::Object(_) => {
            Err(anyhow!("nested arrays/objects in args are not supported in v1"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_simple_args() {
        let v = json!({ "n": 42, "flag": true, "name": "hello", "key": "0xdeadbeef" });
        let args = args_from_json(&v).unwrap();
        assert!(matches!(args["n"], ArgValue::Int(42)));
        assert!(matches!(args["flag"], ArgValue::Bool(true)));
        assert!(matches!(&args["name"], ArgValue::String(s) if s == "hello"));
        assert!(matches!(&args["key"], ArgValue::Bytes(b) if b == &vec![0xde, 0xad, 0xbe, 0xef]));
    }

    #[test]
    fn rejects_null_and_nested() {
        assert!(args_from_json(&json!({ "x": null })).is_err());
        assert!(args_from_json(&json!({ "x": [1, 2] })).is_err());
        assert!(args_from_json(&json!({ "x": { "y": 1 } })).is_err());
    }
}
