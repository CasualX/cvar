# Terminal demonstrations

The README uses three looping GIFs, in order:

- [program-state.gif](../docs/program-state.gif): property reads and writes, an action, and invalid input.
- [nesting-techniques.gif](../docs/nesting-techniques.gif): a dotted property alongside a real list, with actions before and after assignments.
- [runtime-properties.gif](../docs/runtime-properties.gif): creating, editing, and removing properties under a shared dotted scope.

The recordings contain actual example output captured through a terminal.
The renderer adds paced typing, a cursor, and a consistent terminal theme.
Each GIF uses a fixed palette and changed frame rectangles. When ImageMagick is available,
its layer optimizer is also tried, keeping the smaller GIF if its frames and timing match.

To regenerate them, install Python's `Pillow` and `pexpect` packages and the
DejaVu Sans Mono font, then run from the repository root:

```sh
python3 scripts/render-demos.py
```

The script requires `cargo` and `fc-match` on `PATH`. ImageMagick's `convert` is optional.
It builds and runs the examples, checks key output, writes the GIFs to `docs/`, and saves
the captured commands and output in `scripts/demo-recordings.json`.

The ProgramState example is always interactive:

```sh
cargo run --example program-state
```

Press Enter on an empty line to list the properties and actions.
