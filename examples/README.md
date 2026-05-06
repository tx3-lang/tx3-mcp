# Tx3 examples bundled into `tx3-mcp`

Each program here is embedded into the binary via `include_str!` and exposed by the `tx3_examples_list` and `tx3_example_get` MCP tools.

| Name | Summary |
| --- | --- |
| `transfer` | Minimal example: send Ada from one party to another, with change. |
| `vesting` | Time-locked withdrawal protected by a Plutus validator and datum. |
| `faucet` | Public faucet that releases tokens to anyone presenting the right datum. |
| `swap` | Two-party token swap with parameterised amounts. |
| `lang_tour` | A single file that touches every Tx3 language construct — best starting point for a syntax tour. |
| `input_datum` | Spending an input by accessing typed datum fields on the consumed UTxO. |
| `reference_script` | Using a reference script (CIP-31) to attach a validator without inlining it in witnesses. |
| `oracle_reference_datum` | Reading typed data from an oracle UTxO via `reference { ref: ..., datum_is: T }`. |
| `withdrawal` | Cardano-specific staking reward withdrawal with a redeemer (`cardano::withdrawal`). |
| `env_vars` | Declaring an `env { ... }` block and using its fields in a transaction. |

These are curated copies of programs from `tx3-lang/tx3/examples/`. Updates land here when the upstream files change in ways that affect the language surface.
