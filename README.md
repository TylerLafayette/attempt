# attempt

[![Crates.io](https://img.shields.io/crates/v/attempt.svg)](https://crates.io/crates/attempt)
[![Documentation](https://docs.rs/attempt/badge.svg)](https://docs.rs/attempt/)

A utility crate for retrying failable operations with various configuration options.

## Example

```rust
use attempt::Attempt;
# type Data = ();
# type Error = String;

fn fetch_data_from_unreliable_api() -> Result<Data, Error> {
    // Fetch data from an API which randomly fails for no reason.
    // We've all dealt with one of these.
    # Ok(())
}

fn main() {
    let res: Result<Data, Error> =
        Attempt::to(fetch_data_from_unreliable_api)
            .delay(std::time::Duration::from_secs(1))
            .max_tries(1000)
            .run();

    // Be careful with this one.
    let res: Data =
        Attempt::infinitely(fetch_data_from_unreliable_api);

    // "Sensible" default of 10 max tries with an increasing delay between
    //  each attempt starting at 500ms.
    let res: Result<Data, Error> = Attempt::to(fetch_data_from_unreliable_api).run();
}

async fn fetch_data_from_unreliable_api_async() -> Result<Data, Error> {
    // Fetch data from an API which randomly fails for no reason, but do it async!
    # Ok(())
}

async fn async_attempt_example() -> Result<Data, Error> {
    Attempt::to(fetch_data_from_unreliable_api_async)
        .delay(std::time::Duration::from_secs(1))
        .max_tries(1000)
        .run_async()
        .await
}
```
