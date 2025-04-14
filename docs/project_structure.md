## Project Structure

```
.(root)
├── ingredients/     # Individual testable modules
├── recipes/         # Complex flows combining ingredients
└── utils/           # Shared utilities
```

Each ingredient demonstrates isolated functionality.  
Recipes combine these ingredients in specific sequences to demonstrate more complex flows. 

To experiment with an individual ingredient, append the following to the respective ingredient's `src/lib.rs` file:
```rs
#[cfg(test)]
mod tests {
    use super::*;
        
    #[tokio::test]
    async fn my_test() -> Result<(), Box<dyn Error>> {
        // [Your experimental logic here.]
        Ok(())
    }
}
```
### Dependency Organization
All dependencies live in the workspace [`Cargo.toml`](../Cargo.toml).  
Each ingredient/recipe references dependencies in the workspace.

