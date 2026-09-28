# tx3-mcp

A [Model Context Protocol](https://modelcontextprotocol.io) server that exposes the [Tx3](https://github.com/tx3-lang/tx3) toolchain as structured tools for AI agents and editors.

## What it does

`tx3-mcp` runs as a stdio MCP server and offers eight tools backed by the same `tx3-lang`, `tx3-cardano`, and `tx3-tir` crates that power `trix` and the official LSP:

| Tool | Purpose |
| --- | --- |
| `tx3_parse` | Parse a Tx3 source string and return its AST or structured parse errors. |
| `tx3_check` | Run the analyzer over a Tx3 source string or file and return diagnostics with line/column spans. |
| `tx3_lower` | Lower a single named transaction to its TIR JSON. |
| `tx3_apply_args` | Lower a transaction, apply a JSON object of named arguments, and return the post-args TIR. |
| `tx3_inspect_project` | Read a `trix.toml`, build a workspace, and summarize transactions/parties/assets. |
| `tx3_invoke` | Resolve a transaction against the project's TRP endpoint and return the unsigned tx CBOR and hash. It does not sign or submit. |
| `tx3_examples_list` | List the curated example programs bundled into the binary. |
| `tx3_example_get` | Return the source of a bundled example. |

Diagnostics are rendered as structured JSON with severity, code, message, help, and per-span line/column offsets — usable directly by editors and agents.

## Installation

`tx3-mcp` ships with the Tx3 toolchain and is installed by [`tx3up`](https://github.com/tx3-lang/tx3up), which manages the entire toolchain. If you don't have `tx3up` yet, install it with its bootstrap script:

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/tx3-lang/tx3up/releases/latest/download/tx3up-installer.sh | sh
```

Then install the toolchain:

```sh
tx3up
```

This places `tx3-mcp` in `~/.tx3/<channel>/bin/`, which `tx3up` adds to your `PATH`. Verify with:

```sh
tx3-mcp --version
```

`tx3-mcp` is not published to crates.io. Prebuilt binaries are also attached to each [GitHub release](https://github.com/tx3-lang/tx3-mcp/releases).

## Use in Claude Code

The companion [`tx3-skills`](https://github.com/tx3-lang/tx3-skills) plugin wires `tx3-mcp` into Claude Code along with two skills (`tx3-language`, `tx3-project`), four slash commands (`/tx3:new`, `/tx3:check`, `/tx3:inspect`, `/tx3:explain`), and a save hook that runs `tx3_check` on `.tx3` edits.

```sh
claude plugin install https://github.com/tx3-lang/tx3-skills
```

## Use directly

To use `tx3-mcp` from any MCP-compatible client (editor extensions, custom agents), point it at the binary:

```json
{
  "mcpServers": {
    "tx3": { "command": "tx3-mcp" }
  }
}
```

## Compatibility

`tx3-mcp` 0.5.x is built against **tx3-lang 0.22** (`tx3-lang = "=0.22.0"` in `Cargo.toml`), so new tx3 minor releases require a corresponding tx3-mcp release. The [`tx3-lang/toolchain`](https://github.com/tx3-lang/toolchain) channel manifest consumed by `tx3up` pins the tx3-mcp version that matches the rest of the toolchain.

## License

Apache-2.0
