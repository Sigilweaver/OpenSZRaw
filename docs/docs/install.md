---
sidebar_position: 2
---

# Install

## Rust

```sh
cargo add openszraw
```

OpenSZRaw needs Rust 1.85 or newer. There are no native or system
dependencies.

## Python

```sh
pip install openszraw
```

Wheels are published for CPython 3.8-3.15 and PyPy on Linux, macOS, and
Windows. To build from source instead (e.g. for a platform without a
prebuilt wheel), use [maturin](https://www.maturin.rs/):

```sh
git clone https://github.com/Sigilweaver/OpenSZRaw
cd OpenSZRaw
pip install maturin
maturin develop --release
```

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
