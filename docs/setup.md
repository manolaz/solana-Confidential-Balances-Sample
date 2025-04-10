# Setup

## Version Requirements
- `solana-cli` v2.1.7

## Environment Setup
Use the [.env.example](../.env.example) file to create a `.env` file.
This is the bare minimum setup to run the recipes.

Note: The `.env` file will be overwritten during test runs to save generated data allowing you to re-run recipes without re-generating data.

## .env File Behavior
The `.env` file in this project has a unique behavior compared to standard `.env` files:

1. **Dynamic Updates**: The file is updated during recipe execution (particularly in `basic_transfer_recipe`) to serialize transaction results for post-run inspection.

2. **Reusability**: This approach enables:
   - Running individual ingredients in isolation using data from prior recipe runs
   - Collecting all resulting private keys from a single recipe execution

3. **Overwrite Behavior**: Recipes are programmed to overwrite values on every run. For maximum predictability:
   - Clear any extra variables not found in `.env.example`
   - This is analogous to a `build clean` in typical project build configurations

This behavior is implemented using the [dotenvy](https://github.com/allan2/dotenvy) crate's [modifying API](https://github.com/allan2/dotenvy/blob/main/README.md#modifying-api).

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