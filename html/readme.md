# Browser terminal

Plain HTML, CSS, JavaScript, the bundled Alpine, and a Rust Wasm module.
No npm, wasm-bindgen, or JavaScript build step.
The Wasm crate depends only on `cvar`.
Its three example states duplicate the native examples in `../examples`.

## Build and run

Install the Rust target once:

```text
rustup target add wasm32-unknown-unknown
```

Run `html\build.bat` on Windows or `sh html/build.sh` on Linux/macOS.
Both scripts build the module and copy it into `html/public/cvar.wasm`.
The underlying Cargo command is:

```text
cargo build -p cvar-html --release --target wasm32-unknown-unknown
```
