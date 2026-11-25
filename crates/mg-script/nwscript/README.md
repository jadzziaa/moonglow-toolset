# Vendored NWScript compiler

Beamdog's official NWScript compiler (the one the game and the Aurora toolset
use), as published in [neverwinter.nim](https://github.com/niv/neverwinter.nim)
under `neverwinter/nwscript/`, commit
`0972fc1ffa73d1a038b9abb00dc40d5a07e28dbe` (2026-08-31).

- `native/`: the compiler sources. GPL-3.0 (see the headers and
  `native/gpl-3.0.txt`); later contributions MIT, the whole GPL-3.0.
- `compilerapi.cpp`, `compilerapi.h`: neverwinter.nim's C API over it (ABI 1),
  MIT-licensed as part of neverwinter.nim.

Unmodified but for two patches, below. `crates/mg-script/build.rs` compiles
them with the `cc` crate (C++14, as upstream); `src/compiler.rs` calls the C
API.

## Patches

Each is fenced in the source by `// MOONGLOW PATCH (name)` and
`// END MOONGLOW PATCH`; carry them over when updating.

- **return-type-mismatch** (`native/scriptcompparsetree.cpp`,
  `AddUserDefinedIdentifier`). Upstream added a return-type comparison to
  the check that a function's declaration and implementation agree: a
  function declared with one return type and defined with another is
  `FUNCTION IMPLEMENTATION AND DEFINITION DIFFER` there. The game's own
  toolset (Aurora, 8193.37) compiles it as the compiler did before: the
  two are merged on their parameters, and the calls keep the return type
  met first. The base game's `x2_inc_cutscene` needs that
  (`CutCreateObjectCopy` is declared `object` and defined `void`), so with
  the check every script that includes it failed (a builder's report),
  and 338 scripts of the base game and the shipped modules with it. The
  patch sets the comparison's result back to "the same", so Moonglow
  compiles them as Aurora does, byte for byte (the Aurora comparison
  `aurora_scripts`). Moonglow warns of each instead
  (`src/return_types.rs`: the function, both lines, and what the game
  does with it, settled by `engine_return_types`: a definition that
  returns nothing gives the declared type's empty value; one that returns
  a value where none was declared overwrites the caller's last variable;
  one of another type stops the script where it is used).
- **include-trace-line** (`native/scriptcompcore.cpp`, `CompileFile`).
  An error in an include ends with the includes it came through,
  `[via: inc.nss(5)]`, each with the line where it includes the next. The
  line is kept in the include stack's entry only when that file includes
  another, and nothing set it for a file new at its level, so the file the
  error is in was printed with a line left there by an earlier compile of
  the same compiler, or with uninitialized memory: the same error read
  `[via: bad_inc.nss]` in one run and `[via: bad_inc.nss(2)]` in another.
  The patch sets it to 0 as the file takes its level. Upstream has the same
  fault; the compiled code is unchanged (`an_include_s_error_traces_no_line_left_by_another_compile`).

To update: copy the same files from a newer neverwinter.nim, apply the
patches again, record the commit here, and run `cargo test -p
mg-corpus-tests --release --test scripts`, which compares Moonglow's output
with `nwn_script_comp` and the shipped NCS (what the patches change is the
one divergence it allows), and `--test aurora aurora_scripts`.
