use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ProtocolVersion, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ErrorData as McpError, ServerHandler,
};

use crate::tools::{
    self, ApplyArgsRequest, CheckRequest, ExampleGetRequest, InspectProjectRequest, LowerRequest,
    ParseRequest,
};

#[derive(Clone)]
pub struct Tx3Server {
    #[allow(dead_code)] // populated and used by the #[tool_handler] macro expansion
    tool_router: ToolRouter<Tx3Server>,
}

#[tool_router]
impl Tx3Server {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "Parse a Tx3 source string and return its AST or structured parse errors with line/column spans."
    )]
    fn tx3_parse(
        &self,
        Parameters(req): Parameters<ParseRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = tools::run_parse(req);
        json_result(&resp)
    }

    #[tool(
        description = "Run the Tx3 parser and analyzer over a source string or file path. Returns structured diagnostics with severity, code, message, line/column spans, and help text."
    )]
    fn tx3_check(
        &self,
        Parameters(req): Parameters<CheckRequest>,
    ) -> Result<CallToolResult, McpError> {
        match tools::run_check(req) {
            Ok(resp) => json_result(&resp),
            Err(e) => Err(McpError::internal_error(format!("tx3_check: {e}"), None)),
        }
    }

    #[tool(
        description = "Lower a single named Tx3 transaction to its TIR (typed intermediate representation) JSON. Errors during parse or analyze are returned as diagnostics."
    )]
    fn tx3_lower(
        &self,
        Parameters(req): Parameters<LowerRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = tools::run_lower(req);
        json_result(&resp)
    }

    #[tool(
        description = "Lower a Tx3 transaction and apply a JSON object of named arguments. Returns the post-args TIR. Strings starting with `0x` are treated as hex bytes; numbers must fit in i64; nested arrays/objects/nulls are not supported in v1."
    )]
    fn tx3_apply_args(
        &self,
        Parameters(req): Parameters<ApplyArgsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = tools::run_apply_args(req);
        json_result(&resp)
    }

    #[tool(
        description = "Read a `trix.toml` and the project's main .tx3 file, then summarize the parsed AST: parties, assets, policies, and per-transaction shape (params, input/output/mint/burn counts, validity, signers, metadata, cardano blocks)."
    )]
    fn tx3_inspect_project(
        &self,
        Parameters(req): Parameters<InspectProjectRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = tools::run_inspect_project(req);
        json_result(&resp)
    }

    #[tool(
        description = "List the curated Tx3 example programs bundled into this binary. Returns name + one-line summary for each."
    )]
    fn tx3_examples_list(&self) -> Result<CallToolResult, McpError> {
        let resp = tools::run_examples_list();
        json_result(&resp)
    }

    #[tool(
        description = "Return the source of a bundled example by name. Use tx3_examples_list to see available names."
    )]
    fn tx3_example_get(
        &self,
        Parameters(req): Parameters<ExampleGetRequest>,
    ) -> Result<CallToolResult, McpError> {
        let resp = tools::run_example_get(req);
        json_result(&resp)
    }
}

#[tool_handler]
impl ServerHandler for Tx3Server {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::from_build_env())
        .with_protocol_version(ProtocolVersion::V_2024_11_05)
        .with_instructions(
            "tx3-mcp exposes the Tx3 toolchain as MCP tools. \
             Use tx3_parse to inspect AST, tx3_check to surface parse+analyze diagnostics, \
             tx3_lower for TIR of a single transaction, tx3_apply_args to bind arguments \
             and produce post-args TIR, tx3_inspect_project to summarize a trix.toml project, \
             and tx3_examples_list/tx3_example_get for curated learning examples.".to_string(),
        )
    }
}

fn json_result<T: serde::Serialize>(value: &T) -> Result<CallToolResult, McpError> {
    let s = serde_json::to_string(value)
        .map_err(|e| McpError::internal_error(format!("serialization failed: {e}"), None))?;
    Ok(CallToolResult::success(vec![Content::text(s)]))
}
