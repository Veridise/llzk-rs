# llzk-sys-build-support

Helper crate used while building `llzk-sys`. Includes functionality for building LLZK and generating Rust bindings.

## Design 

The crate revolves around 3 configurator traits, one for each supported tool: [CMake](https://crates.io/crates/cmake), [Bindgen](https://crates.io/crates/bindgen), and [CC](https://crates.io/crates/cc). The different configurations implement these traits and to run a tool you first prepare a sequence of configurators then pass the corresponding builder through it. 
