# Multi-Select (MultiEnum) Support

## Summary

Add a `MultiEnum` argument type that allows users to select multiple items from a list. Selected values are joined with `,` and substituted into the command template as a single string.

## Domain Changes

### `ArgumentType` enum (`src/domain/workflow.rs`)

Add `MultiEnum` variant:

```rust
pub enum ArgumentType {
    Text,
    Enum,
    MultiEnum,
    Number,
    Boolean,
}
```

### `WorkflowArgument` struct (`src/domain/workflow.rs`)

Add two optional fields:

```rust
pub min_selections: Option<usize>,
pub max_selections: Option<usize>,
```

These only apply to `MultiEnum`. They are ignored for other argument types.

### Example YAML

```yaml
arguments:
  - name: namespaces
    arg_type: MultiEnum
    description: "Select target namespaces"
    enum_command: "kubectl get namespaces --no-headers | awk '{print $1}'"
    enum_name: "namespaces"
    min_selections: 1
    max_selections: 5

  - name: environments
    arg_type: MultiEnum
    description: "Select environments"
    enum_variants:
      - dev
      - staging
      - prod
    min_selections: 1
```

## Resolver Changes (`src/adapter/resolver.rs`)

### New import

Add `inquire::MultiSelect`.

### New match arm

In `resolve_argument`, add a `MultiEnum` arm that mirrors the `Enum` arm but delegates to multi-select methods.

### New methods

- `resolve_static_multi_enum_argument(arg, variants) -> Result<String>`
  - Builds options from `enum_variants` (no custom value entry)
  - Prompts with `MultiSelect::new().with_page_size(10)`
  - Applies min/max validators via inquire's built-in validator support
  - Joins selected items with `,`

- `resolve_dynamic_multi_enum_argument(arg, enum_command, current_values) -> Result<String>`
  - Resolves `dynamic_resolution` references (same as `Enum`)
  - Executes command, parses output lines (same as `Enum`)
  - Prompts with `MultiSelect` (no custom value entry)
  - Applies min/max validators
  - Joins selected items with `,`

### Shared helper

Extract the command-execution + output-parsing logic from `resolve_dynamic_enum_argument` into a private helper method so both `Enum` and `MultiEnum` can reuse it:

```rust
async fn execute_enum_command(
    arg: &WorkflowArgument,
    enum_command: &str,
    current_values: &HashMap<String, String>,
) -> Result<Vec<String>, WorkflowError>
```

## No changes needed

- **`command.rs`**: Resolved values are already `HashMap<String, String>`. Multi-select values are plain comma-separated strings.
- **Template rendering**: Tera receives a string like `"dev,staging,prod"` — no special handling.
- **Events/State**: `WorkflowArgumentsResolvedEvent` stores `HashMap<String, String>` — unchanged.
- **Validation in `ResolveArgumentsCommand`**: Already checks that every argument name has a resolved value — unchanged.

## i18n

Add one new key to `config/i18n/en.json` and `config/i18n/es.json`:

```json
"prompt_multi_select": "Select one or more {0}"
```

## Separator

Selected values are always joined with `,` (comma, no space). Example: `dev,staging,prod`.
