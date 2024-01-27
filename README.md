# easy-wire | pipewire-rs

### Rust bindings for pipewire and SPA libraries

- Documentation
    - [`pipewire`](https://pipewire.pages.freedesktop.org/pipewire-rs/pipewire/)
    - [`libspa`](https://pipewire.pages.freedesktop.org/pipewire-rs/libspa/index.html)
- [Examples](https://gitlab.freedesktop.org/pipewire/pipewire-rs/-/tree/main/pipewire/examples)
- [How to contribute](https://gitlab.freedesktop.org/pipewire/pipewire-rs/-/blob/main/docs/CONTRIBUTING.md)

### **This wrapper is in progress but is a POC of removing the need to set up the pipewire main loop in your program**
### **These bindings are work-in-progress. Expect frequent breakage, bugs, and missing features.**
### Example pw-mon is currently the only somewhat working example. Note: I am still setting up the base wrapper and will start cleaning up the code.

## Requirements
- Rust 1.64 or newer
- PipeWire 0.3 development files
- Clang (see [bindgen requirements](https://rust-lang.github.io/rust-bindgen/requirements.html))

## Getting help
You can ask questions related to the rust bindings at [#pipewire-rs](irc://irc.oftc.net:6667/pipewire-rs), and general pipewire questions at [#pipewire](irc://irc.oftc.net:6667/pipewire) via IRC on [OFTC](https://www.oftc.net/).

## License
The pipewire-rs project is distributed under the terms of the MIT license.

See [LICENSE](LICENSE) for more information.
