# Contributing to OpenSZRaw

Thanks for your interest in OpenSZRaw. This is a small, single-maintainer
project that ships [Apache-2.0](LICENSE) Rust tooling for the open
mass-spec stack.

Crates in this repo: openszraw.

## Contributing code (pull requests)

PRs are welcome for changes of any size, including large or breaking ones -
there's no requirement to open an issue first. That said, for larger changes
you may want to open an issue before writing code, especially if you're
unsure whether it fits the project's direction: a large PR that conflicts
with the roadmap can still be rejected even if the code itself is solid, and
an issue is a cheap way to check alignment before investing the time.

For any PR:

- Scope it to one logical change.
- Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`
  locally. CI will run them too.
- Run `cargo test --workspace`.
- Update [CHANGELOG.md](CHANGELOG.md) under `## [Unreleased]` with a
  short bullet describing the user-visible change.
- Prefer [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
- Code is ASCII only and `#![forbid(unsafe_code)]`.

## Vendor software and clean-room policy

If you are contributing to the Shimadzu LabSolutions `.lcd`/`.qgd`/`.gcd`
reader, please make sure new format knowledge came from public datasets
and your own analysis - **do not** copy or paste vendor SDK headers,
sources, decompiled code, or proprietary specifications. See
[ATTRIBUTION.md](ATTRIBUTION.md) and [CORPUS.md](CORPUS.md).

This section follows the Sigilweaver [format provenance
policy](https://github.com/Sigilweaver/ops/blob/main/PROVENANCE.md), shared
by every reader in the suite. Where the two differ, the policy wins.

**No vendor software in the project.** Do not depend on Shimadzu's own
tools, or on anything that reads `.lcd`/`.qgd`/`.gcd` data through
Shimadzu's libraries, and do not run them in CI or tests. That covers
LabSolutions, GCMSsolution, GCsolution, and any third-party tool that
itself links a Shimadzu SDK to read this format, such as ProteoWizard
`msconvert` (it counts as vendor software here even though it's open
source). Do not bypass encryption, license checks, or any other technical
protection to read data, and do not contribute format knowledge from anyone
working from vendor-internal information.

Correctness is argued from open references: public data files, the
CFBF/OLE2 container format (a public Microsoft specification, not
Shimadzu-specific), the PSI-MS mzML schema, published open specifications,
roundtrip and self-consistency invariants, and independent open-source
parsers used purely as format checkers (see
[ATTRIBUTION.md](ATTRIBUTION.md) for the one such reference this project
checks itself against, and how). Every layout must be explainable from
those. If you can only explain a field by having watched what LabSolutions
shows for it, don't write that down - keep digging in the bytes instead, or
flag it as unresolved.

Several structural questions remain genuinely open (IT-TOF m/z
calibration, PDA/chromatogram stream decoding) - see
[docs/format/06-known-limitations.md](docs/format/06-known-limitations.md).
Further clean-room byte-level analysis toward closing these is welcome.
Do not disassemble or decompile any Shimadzu binary, and do not send us
key material or specifications obtained from a leak, a breach, or any
other unauthorized disclosure - we will not accept or use it, regardless
of how it's documented. If you believe you have a legitimate external
lead (e.g. something Shimadzu has published themselves), open an issue
describing it before doing anything else so we can assess it together.

**Cross-checking against vendor software is allowed, with disclosure.** If
you are licensed to run Shimadzu software, you may compare values this
project decodes against its output. A comparison can confirm a decoder; it
cannot be the only basis for one. Say so in the pull request: which tool,
which files, and what you compared. Do not paste vendor-tool output into
the repository. If a value's only basis is a vendor comparison (for example
a derived quantity whose formula was matched to vendor output rather than
taken from a public source), document that next to the value. Cross-checks
are recorded in [ATTRIBUTION.md](ATTRIBUTION.md).

If you'd rather not write the fix yourself, or you've found a bug by
comparing against vendor software and don't want to send a pull request,
please open an issue instead. Describe the symptom on the input that
triggers it - what's wrong, and on what file - without pasting vendor tool
output or vendor source. We'll investigate and fix it from public
references. Detailed issue reports are genuinely useful and will be acted
on.

The local corpus (see [CORPUS.md](CORPUS.md)) holds real research data
that can be expensive to redownload. Treat it as read-only except for
adding cache/derived files you clearly own.

Point the tests at it with `OPENSZRAW_CORPUS` (the directory holding
`index.csv` and one folder per accession). Corpus-backed tests skip when a
fixture is missing; set `REQUIRE_CORPUS=1` to make a missing fixture fail
instead:

    OPENSZRAW_CORPUS=/path/to/SZRaw REQUIRE_CORPUS=1 cargo test --workspace

## Security

See [SECURITY.md](SECURITY.md) for the vulnerability reporting process.
