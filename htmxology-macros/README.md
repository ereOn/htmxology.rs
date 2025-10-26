# htmxology-macros

Procedural macros for the htmxology framework.

## Architecture Guidelines

This crate follows a consistent organizational pattern for all derive macros to maintain clarity and scalability.

### Directory Structure

Each derive macro should be organized in its own subdirectory with the following structure:

```
src/
├── macro_name/
│   ├── mod.rs           # Main derive function and exports
│   ├── config.rs        # Data structures for parsed configuration
│   ├── codegen.rs       # Code generation helpers
│   ├── snapshots/       # Insta snapshot test files
│   └── *.rs             # Other supporting modules as needed
├── utils.rs             # Shared utilities across all macros
└── lib.rs               # Crate entry point
```

### Module Responsibilities

#### `mod.rs` - Main Entry Point
- Contains the public `derive()` function
- High-level orchestration of parsing and code generation
- Should be relatively thin (~300-500 lines)
- Contains snapshot tests

**Example structure:**
```rust
mod config;
mod codegen;

pub fn derive(input: &mut syn::DeriveInput) -> syn::Result<TokenStream> {
    let data = crate::utils::expect_enum(input, "MacroName")?;
    let configs = parse_configs(data)?;
    generate_implementation(configs)
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;
    use crate::utils::testing::test_derive;
    use insta::assert_snapshot;

    fn test_macro_name(input: &str) -> String {
        test_derive(input, derive)
    }

    // ... test cases
}
```

#### `config.rs` - Configuration Structures
- **Purpose**: Separate parsing from code generation
- Contains intermediate data structures representing parsed configuration
- Implements parsing logic from `syn` types into config types
- Should be focused on **what** the configuration is, not **how** to generate code

**Key principles:**
- Use descriptive struct/enum names (e.g., `VariantConfig`, `FieldConfig`, `FieldRole`)
- Include `From` implementations or parsing functions for `syn` types
- Add validation logic here
- Keep structures `Clone` for flexibility
- Avoid `TokenStream` - this module should be pure data

**Example:**
```rust
#[derive(Clone)]
pub struct VariantConfig {
    pub ident: Ident,
    pub fields: FieldsConfig,
    // ... other config
}

impl VariantConfig {
    pub fn from_variant(variant: &Variant) -> syn::Result<Self> {
        // Parse and validate
    }
}
```

#### `codegen.rs` - Code Generation Helpers
- **Purpose**: Reusable code generation functions
- Contains functions that generate `TokenStream` from config structures
- Each function should be focused and composable
- Should handle both Named and Unnamed variants uniformly when possible

**Key principles:**
- Functions take `&Config` types and return `TokenStream` or `syn::Result<TokenStream>`
- Use descriptive function names (e.g., `generate_pattern`, `generate_url_format`)
- Document expected output with examples in docstrings
- Keep functions pure and testable
- Use helper functions to avoid duplication

**Example:**
```rust
/// Generates a match pattern for a variant.
///
/// # Example Output
///
/// ```ignore
/// Self::Variant { field1, field2 }
/// ```
pub fn generate_pattern(config: &VariantConfig) -> TokenStream {
    // ...
}
```

#### `snapshots/` - Test Snapshots
- One `.snap` file per test case
- Named consistently: `{crate}__{module}__snapshot_tests__{test_name}.snap`
- Snapshots capture the **generated code** to prevent regressions
- Review snapshots carefully when they change - don't blindly accept

### Shared Utilities (`utils.rs`)

Common functionality shared across all derive macros:

- **`expect_enum()`** - Validates input is an enum (not struct/union)
- **`testing::test_derive()`** - Wrapper for snapshot tests

When adding new shared utilities, consider:
- Will this be used by multiple derive macros?
- Does it eliminate meaningful duplication?
- Is it generic enough to be reusable?

### Testing Guidelines

All derive macros must have comprehensive snapshot tests:

1. **Coverage**: Test all supported patterns and edge cases
2. **Naming**: Use descriptive test names (e.g., `unit_variant_get`, `named_body_param`)
3. **Organization**: Keep tests in `mod snapshot_tests` within `mod.rs`
4. **Verification**: Run `cargo insta review` after changes to inspect diffs
5. **Regression Prevention**: Never delete snapshots unless removing functionality

**Test structure:**
```rust
#[test]
fn descriptive_test_name() {
    let input = r#"
        enum Example {
            #[attribute]
            Variant,
        }
    "#;
    assert_snapshot!(test_macro_name(input));
}
```

### Code Quality Standards

Before committing changes:

1. **Format**: `cargo fmt --all`
2. **Test**: `cargo test -p htmxology-macros` (all tests must pass)
3. **Lint**: `cargo clippy -p htmxology-macros` (zero warnings)
4. **Review Snapshots**: `cargo insta review` (verify changes are correct)

### Migration Guide

When refactoring an existing derive macro to follow this pattern:

1. Create subdirectory: `src/macro_name/`
2. Move main file to `src/macro_name/mod.rs`
3. Extract config structures to `config.rs`
4. Extract code generation to `codegen.rs`
5. **Move (don't regenerate) snapshots** to `snapshots/` subdirectory
6. Update imports in `lib.rs`
7. Verify all tests still pass with `cargo test`
8. Verify snapshots are identical with `git diff`

**Critical**: When moving snapshots, use `git mv` or ensure file contents are identical to prevent false regressions.

### Benefits of This Pattern

- **Separation of Concerns**: Parsing vs. code generation
- **Testability**: Small, focused functions are easier to test
- **Maintainability**: Clear structure makes code easy to navigate
- **Reusability**: Helpers can be shared between variants
- **Scalability**: Easy to add new features without increasing complexity
- **Documentation**: Self-documenting through module organization

### Examples

See `src/route/` for a complete example following this pattern.
