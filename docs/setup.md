# Setup

## Version Requirements
- `solana-cli` v2.1.7

## Environment Setup
Use the [.env.example](../.env.example) file to create a `.env` file.
This is the bare minimum setup to run the recipes.

## Environment File Behavior
This project uses two environment files:

1. **`.env`**: Contains initial configuration values that remain unchanged during runtime.

2. **`runtime_output.env`**: Generated during execution to store all runtime values and execution results.

This approach provides several advantages:

1. **Clean Separation**: The original `.env` configuration remains untouched during recipe execution.

2. **Runtime Value Storage**: All dynamically generated values (keypairs, transaction results, etc.) are serialized to `runtime_output.env`.

3. **Prioritized Loading**: When a recipe or ingredient runs:
   - Values are first searched for in `runtime_output.env` (from prior executions)
   - If not found, the system falls back to the original `.env` file
   - If not found in either, a new value is generated and stored in `runtime_output.env`

4. **Reusability**: This approach enables:
   - Running individual ingredients in isolation using data from prior recipe runs
   - Collecting all resulting private keys from a single recipe execution

5. **Reset Capability**: To reset to a clean state, simply delete `runtime_output.env`

This behavior is implemented using the [dotenvy](https://github.com/allan2/dotenvy) crate.

## Test Commands

### Running Individual Ingredients

```bash
# Run all tests in an ingredient
cargo test -p setup_participants

# Run a specific test from an ingredient
cargo test -p setup_participants setup_basic_participant
```

### Running Recipes (Test Sequences)

```bash
# Run all recipes
cargo test -p test-runner

# Run a specific recipe
cargo test -p test-runner recipe::basic_transfer_recipe
```

### Test Output Options

```bash
# Show log output mid-test
cargo test -- --nocapture

# Show test execution time
cargo test -- --show-output

```