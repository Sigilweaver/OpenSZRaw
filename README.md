# OpenSZRaw

Planned Rust and Python reader for Shimadzu LabSolutions mass
spectrometry raw data (`.lcd` LC-MS, `.qgd` GCMS, `.gcd` GC), clean-room
reverse-engineered with no Shimadzu SDK or software dependency.

> Sibling readers in the same stack:
> [OpenTFRaw](https://github.com/Sigilweaver/OpenTFRaw) (Thermo),
> [OpenWRaw](https://github.com/Sigilweaver/OpenWRaw) (Waters),
> [OpenTimsTDF](https://github.com/Sigilweaver/OpenTimsTDF) (Bruker),
> [OpenARaw](https://github.com/Sigilweaver/OpenARaw) (Agilent),
> [OpenSXRaw](https://github.com/Sigilweaver/OpenSXRaw) (SCIEX).

## Status

A Rust reader (`crates/openszraw`) implements all three confirmed raw
data variants: `.qgd` GC-MS (full-scan profile and MRM/targeted), `.lcd`
IT-TOF (run-length-encoded profile spectra over a raw, uncalibrated
time-bin axis), and `.lcd` QTOF (centroid). See `docs/format/` for the
byte-level format specs and `docs/format/06-known-limitations.md` for
what is deliberately not yet resolved (TOF calibration, some MS2
precursor m/z values). Python bindings are not yet implemented. See the
sourcing strategy in the ops repo's
[SCOPING_PLAN.md](https://github.com/Sigilweaver/ops/blob/main/SCOPING_PLAN.md)
and this repo's `re/ROADMAP.md` (local-only, gitignored) for the current
phase.

## License

Apache-2.0.
