# Vendored NWScript compiler

Beamdog's official NWScript compiler (the one the game and the Aurora toolset
use), as published in [neverwinter.nim](https://github.com/niv/neverwinter.nim)
under `neverwinter/nwscript/`, commit
`0972fc1ffa73d1a038b9abb00dc40d5a07e28dbe` (2026-08-31).

- `native/`: the compiler sources. GPL-3.0 (see the headers and
  `native/gpl-3.0.txt`); later contributions MIT, the whole GPL-3.0.
- `compilerapi.cpp`, `compilerapi.h`: neverwinter.nim's C API over it (ABI 1),
  MIT-licensed as part of neverwinter.nim.

Unmodified. `crates/mg-script/build.rs` compiles them with the `cc` crate
(C++14, as upstream); `src/compiler.rs` calls the C API.

To update: copy the same files from a newer neverwinter.nim, record the commit
here, and run `cargo test -p mg-corpus-tests --release --test scripts`, which
compares Moonglow's output with `nwn_script_comp` and the shipped NCS.
