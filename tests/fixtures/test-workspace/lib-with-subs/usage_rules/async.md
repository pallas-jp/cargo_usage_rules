# Async Patterns

This sub-file contains async usage patterns for lib-with-subs.

## Async Runtime

Always use tokio runtime with this library.

```rust
#[tokio::main]
async fn main() {
    // async code here
}
```
