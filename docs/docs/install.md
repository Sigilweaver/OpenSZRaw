---
sidebar_position: 2
---

# Install

OpenSZRaw is not yet published to crates.io or PyPI. Until then, use it
from source.

## Rust

```toml
[dependencies]
openszraw = { git = "https://github.com/Sigilweaver/OpenSZRaw" }
```

OpenSZRaw needs Rust 1.85 or newer. There are no native or system
dependencies.

## Python

Python bindings (`crates/openszraw-py`, a PyO3 crate) exist in the
repository but are not yet packaged or published. Build the wheel
locally with [maturin](https://www.maturin.rs/):

```sh
git clone https://github.com/Sigilweaver/OpenSZRaw
cd OpenSZRaw
pip install maturin
maturin develop --release
```

This builds and installs the `openszraw` Python module into your active
virtualenv.

## Verifying the install

```sh
cargo test --workspace
```

## Optional: corpus fetcher

The validation corpus is not redistributed. It is pulled on demand from
public repositories (PRIDE, MassIVE, MetaboLights) using local research
tooling (not part of the published crate). See
[`CORPUS.md`](https://github.com/Sigilweaver/OpenSZRaw/blob/main/CORPUS.md)
for the file list and provenance.
