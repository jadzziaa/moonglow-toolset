# Update log

## 2026-10-04
* **Update**: Made `docs/` an Open Knowledge Format (OKF v0.2) bundle: frontmatter on all 43 documents (the plan, [findings](findings.md), proposals, [research](research/index.md), [parity](parity/index.md), [plugins](plugins/index.md) and every chapter of [the manual](manual/README.md)), index files and this log. No content changed. The program's manual reader (`crates/mg-ui/src/manual.rs`) skips a chapter's frontmatter, and the two generated documents get theirs from their generators (`tools/aurora/uiinv/gen.py`, `crates/mg-corpus-tests/examples/observed_schema.rs`).
