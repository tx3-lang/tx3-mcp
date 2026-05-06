use std::process::Stdio;
use std::time::Duration;

use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InvokeRequest {
    /// Absolute path to the trix project directory (must contain `trix.toml`).
    pub project_dir: String,
    /// JSON object of arguments passed verbatim to `trix invoke --args-json`.
    /// Shape matches what trix expects (same schema as `--args-json`).
    pub args: serde_json::Value,
    /// Optional profile name (`-p / --profile`). Default: trix's first profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Skip submission, only assemble + sign (`--skip-submit`).
    #[serde(default)]
    pub skip_submit: bool,
    /// Timeout in seconds (default 60). stdin is closed before exec, so any
    /// interactive cshell prompt fails fast rather than hangs; this guard
    /// catches longer-running stalls (e.g. devnet not yet ready).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct InvokeResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub async fn run(req: InvokeRequest) -> InvokeResponse {
    let args_json = match serde_json::to_string(&req.args) {
        Ok(s) => s,
        Err(e) => {
            return InvokeResponse {
                ok: false,
                exit_code: None,
                stdout: String::new(),
                stderr: String::new(),
                error: Some(format!("args serialization failed: {e}")),
            }
        }
    };

    if !req.args.is_object() {
        return InvokeResponse {
            ok: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some("`args` must be a JSON object".to_string()),
        };
    }

    let mut cmd = Command::new("trix");
    cmd.current_dir(&req.project_dir);
    if let Some(profile) = &req.profile {
        cmd.arg("-p").arg(profile);
    }
    cmd.arg("invoke");
    cmd.arg("--args-json").arg(&args_json);
    if req.skip_submit {
        cmd.arg("--skip-submit");
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let secs = req.timeout_secs.unwrap_or(60);
    let fut = cmd.output();

    match timeout(Duration::from_secs(secs), fut).await {
        Ok(Ok(out)) => InvokeResponse {
            ok: out.status.success(),
            exit_code: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            error: None,
        },
        Ok(Err(e)) => InvokeResponse {
            ok: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(format!(
                "failed to spawn `trix`: {e}. Is trix on PATH? Run `tx3up` to install."
            )),
        },
        Err(_) => InvokeResponse {
            ok: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(format!(
                "trix invoke timed out after {secs}s. The most common cause is cshell waiting for an interactive prompt (e.g. tx selection or signer choice) that the args didn't disambiguate. Ensure `args` fully specifies the transaction, or pre-launch the devnet with `trix devnet -b`."
            )),
        },
    }
}
