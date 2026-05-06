use rmcp::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// (name, summary, source) for each bundled example.
const EXAMPLES: &[(&str, &str, &str)] = &[
    (
        "transfer",
        "Minimal example: send Ada from one party to another, with change.",
        include_str!("../../examples/transfer.tx3"),
    ),
    (
        "vesting",
        "Time-locked withdrawal protected by a Plutus validator and datum.",
        include_str!("../../examples/vesting.tx3"),
    ),
    (
        "faucet",
        "Public faucet that releases tokens to anyone presenting the right datum.",
        include_str!("../../examples/faucet.tx3"),
    ),
    (
        "swap",
        "Two-party token swap with parameterised amounts.",
        include_str!("../../examples/swap.tx3"),
    ),
    (
        "lang_tour",
        "A single file that touches every Tx3 language construct — best starting point for a syntax tour.",
        include_str!("../../examples/lang_tour.tx3"),
    ),
    (
        "input_datum",
        "Spending an input by accessing typed datum fields on the consumed UTxO.",
        include_str!("../../examples/input_datum.tx3"),
    ),
    (
        "reference_script",
        "Using a reference script (CIP-31) to attach a validator without inlining it in witnesses.",
        include_str!("../../examples/reference_script.tx3"),
    ),
    (
        "oracle_reference_datum",
        "Reading typed data from an oracle UTxO via `reference { ref: ..., datum_is: T }`.",
        include_str!("../../examples/oracle_reference_datum.tx3"),
    ),
    (
        "withdrawal",
        "Cardano-specific staking reward withdrawal with a redeemer (`cardano::withdrawal`).",
        include_str!("../../examples/withdrawal.tx3"),
    ),
    (
        "env_vars",
        "Declaring an `env { ... }` block and using its fields in a transaction.",
        include_str!("../../examples/env_vars.tx3"),
    ),
];

#[derive(Debug, Serialize)]
pub struct ExampleSummary {
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct ExamplesListResponse {
    pub examples: Vec<ExampleSummary>,
}

pub fn run_list() -> ExamplesListResponse {
    ExamplesListResponse {
        examples: EXAMPLES
            .iter()
            .map(|(name, summary, _)| ExampleSummary {
                name: (*name).to_string(),
                summary: (*summary).to_string(),
            })
            .collect(),
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExampleGetRequest {
    /// Name from `tx3_examples_list` (e.g. "transfer", "vesting", "lang_tour").
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ExampleGetResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

pub fn run_get(req: ExampleGetRequest) -> ExampleGetResponse {
    match EXAMPLES.iter().find(|(name, _, _)| *name == req.name) {
        Some((_, _, source)) => ExampleGetResponse {
            ok: true,
            source: Some((*source).to_string()),
            error: None,
        },
        None => ExampleGetResponse {
            ok: false,
            source: None,
            error: Some(format!(
                "no example named `{}`. Call tx3_examples_list to see available names.",
                req.name
            )),
        },
    }
}
