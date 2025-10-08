# switzerland-power-calc

The underlying Switzerland Power system for the Switzerland tournament series.

To build:

```shell
cargo binstall cargo-vcpkg
cargo vcpkg build --manifest-path switzerland-power-animated/Cargo.toml
cargo build --release -p switzerland-power-calc
```
