# Setup

## Version Requirements
- `solana-cli` v2.1.7

## Environment Setup
Use the [.env.example](../.env.example) file to create a `.env` file.
This is the bare minimum setup to run the recipes.

Note: The `.env` file will be overwritten during test runs to save generated data allowing you to re-run recipes without re-generating data.
  
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