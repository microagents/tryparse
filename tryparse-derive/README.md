# tryparse-derive

Procedural macro for [`tryparse`](https://crates.io/crates/tryparse). Provides the `LlmDeserialize` derive macro for parsing messy LLM outputs with fuzzy field matching, enum matching, and union types.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
tryparse = { version = "0.3", features = ["derive"] }
tryparse-derive = "0.3"
```

## What This Provides

The `#[derive(LlmDeserialize)]` macro generates fuzzy deserialization implementations that handle:

- **Fuzzy field matching**: camelCase ↔ snake_case, case-insensitive
- **Fuzzy enum matching**: case-insensitive, substring matching, edit distance
- **Union types**: Automatic variant selection with `#[llm(union)]`
- **Type coercion**: String numbers, array unwrapping, etc.

## LlmDeserialize

### Without LlmDeserialize (serde only)

```rust
use serde::Deserialize;
use tryparse::parse;

#[derive(Deserialize)]
struct User {
    user_name: String,  // Must match exactly "user_name"
    max_count: i64,
}

// ✅ Works - exact match
let json = r#"{"user_name": "Alice", "max_count": 30}"#;
let user: User = parse(json).unwrap();

// ❌ Fails - field name mismatch
let json = r#"{"userName": "Alice", "maxCount": 30}"#;
let user: User = parse(json).unwrap(); // Error: unknown field `userName`
```

### With LlmDeserialize

```rust
use tryparse::parse_llm;
use tryparse_derive::LlmDeserialize;

#[derive(LlmDeserialize)]
struct User {
    user_name: String,  // Matches: userName, UserName, user-name, USER_NAME, etc.
    max_count: i64,
}

// ✅ All of these work
let json = r#"{"userName": "Alice", "maxCount": 30}"#;
let user: User = parse_llm(json).unwrap();

let json = r#"{"UserName": "Alice", "MaxCount": 30}"#;
let user: User = parse_llm(json).unwrap();

let json = r#"{"user-name": "Alice", "max-count": 30}"#;
let user: User = parse_llm(json).unwrap();
```

### Fuzzy Enum Matching

```rust
use tryparse_derive::LlmDeserialize;

#[derive(LlmDeserialize)]
enum Status {
    InProgress,  // Matches: "in_progress", "in-progress", "inprogress", "in progress", "IN_PROGRESS"
    Completed,   // Matches: "complete", "completed", "COMPLETED", "done"
    Cancelled,
}

// All of these parse correctly
let s: Status = parse_llm(r#""in-progress""#).unwrap();  // Status::InProgress
let s: Status = parse_llm(r#""complete""#).unwrap();      // Status::Completed
let s: Status = parse_llm(r#""CANCELLED""#).unwrap();     // Status::Cancelled
```

### Union Types

Automatically picks the best matching variant based on structure:

```rust
use tryparse_derive::LlmDeserialize;

#[derive(LlmDeserialize)]
#[llm(union)]  // Required attribute for union behavior
enum Value {
    Number(i64),
    Text(String),
}

// Parses as Number(42)
let v: Value = parse_llm("42").unwrap();

// Parses as Text("hello")
let v: Value = parse_llm(r#""hello""#).unwrap();
```

Union matching uses a scoring algorithm to pick the variant with the least type coercions.

### Implied Key (Single-Field Unwrapping)

When a struct has a single field, the value can be provided directly:

```rust
use tryparse_derive::LlmDeserialize;

#[derive(LlmDeserialize)]
struct Wrapper {
    data: String,
}

// Instead of requiring {"data": "hello"}
// You can pass the value directly
let w: Wrapper = parse_llm(r#""hello world""#).unwrap();
assert_eq!(w.data, "hello world");
```

## When to Use

| Scenario | Use This |
|----------|----------|
| Strict JSON from well-behaved APIs | `serde::Deserialize` (no derive macro needed) |
| LLM responses with inconsistent field names | `#[derive(LlmDeserialize)]` |
| Need to handle multiple possible types | `#[derive(LlmDeserialize)]` with `#[llm(union)]` |
| Parsing enums where LLM might use different casings | `#[derive(LlmDeserialize)]` |
| LLM outputs with typos in enum variants | `#[derive(LlmDeserialize)]` (edit-distance matching) |

## Technical Notes

- This is a procedural macro crate (separate from `tryparse` due to Rust compiler requirements)
- `LlmDeserialize` generates implementations that use BAML's fuzzy matching algorithms
- Field matching normalizes to snake_case and matches case-insensitively
- Union types try strict matching first, then fall back to lenient matching with scoring
- All transformations are tracked for debugging (see `tryparse` docs)

## Example: Complete Usage

```rust
use tryparse::parse_llm;
use tryparse_derive::LlmDeserialize;

#[derive(Debug, LlmDeserialize)]
struct Config {
    api_key: String,
    max_retries: i64,
    timeout_ms: Option<i64>,
    status: Status,
}

#[derive(Debug, LlmDeserialize)]
enum Status {
    Enabled,
    Disabled,
}

// LLM returns inconsistent format - handles all of these issues:
// - camelCase instead of snake_case (apiKey → api_key)
// - String number ("3" → 3)
// - Case mismatch ("enabled" → Status::Enabled)
// - Missing optional field (timeout_ms)
let llm_output = r#"
{
  "apiKey": "secret",
  "maxRetries": "3",
  "status": "enabled"
}
"#;

let config: Config = parse_llm(llm_output).unwrap();
println!("{:?}", config);
// Config {
//   api_key: "secret",
//   max_retries: 3,
//   timeout_ms: None,
//   status: Status::Enabled
// }
```

## License

Apache-2.0
