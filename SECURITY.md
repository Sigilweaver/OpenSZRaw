# Security Policy

## Supported Versions

| Version | Supported |
| ------- | --------- |
| latest  | Yes       |
| older   | No        |

Only the latest published release receives security updates.

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report privately via [GitHub Security Advisories](https://github.com/Sigilweaver/OpenSZRaw/security/advisories/new).

Include:

- A description of the vulnerability and its potential impact.
- Steps to reproduce or a proof of concept (a small `.lcd`/`.qgd` file is
  ideal; small synthetic byte sequences are even better).
- The crate version and OS / toolchain.

Expect an initial acknowledgment within 7 days.

## Scope

In scope:

- **Parser correctness on malicious input.** OpenSZRaw parses a CFBF/OLE2
  container (`.lcd`/`.qgd`) and several custom binary stream formats
  inside it. Panics, out-of-bounds reads, undefined behavior, infinite
  loops, or memory exhaustion triggered by a crafted or truncated file
  are in scope.
- **Memory safety**: the `openszraw` crate forbids `unsafe_code`. A
  demonstrated unsafe-code violation reachable from safe API is a
  security bug.
- **Path-traversal or arbitrary-file-write bugs** in any helper that
  derives output paths from input filenames.
- **Supply-chain integrity** of published artifacts on crates.io.

Out of scope:

- Denial of service via legitimately large `.lcd`/`.qgd` files. Real
  acquisitions can be many GB by design.
- Inaccurate decoding of specific instrument acquisition modes. Those are
  correctness bugs - file them as regular issues.
- IT-TOF m/z calibration or PDA/chromatogram stream decoding being
  unresolved. Those are known, documented limitations (see
  [docs/format/06-known-limitations.md](docs/format/06-known-limitations.md)),
  not vulnerabilities.
- Vulnerabilities in third-party crates with no demonstrated exploit path
  through OpenSZRaw.

## Disclosure

We follow coordinated disclosure. Reporters are credited in the release
notes unless they prefer to remain anonymous. We aim to ship a fix within
30 days of confirming a high or critical issue.

## Note on reverse engineering

OpenSZRaw was developed by clean-room reverse engineering of public
artifacts (PRIDE, MassIVE, and MetaboLights deposits, plus the public
CFBF/OLE2 container specification). It does not depend on any Shimadzu
SDK or binary blob, and contains no Shimadzu proprietary code. Bug
reports about parser accuracy or coverage are welcome but are not
security issues unless they involve one of the categories above.

## Stack context

OpenSZRaw is one of several vendor readers in the
[OpenMassSpec](https://github.com/Sigilweaver/OpenMassSpec) stack. Sibling
readers: [OpenTFRaw](https://github.com/Sigilweaver/OpenTFRaw) (Thermo),
[OpenWRaw](https://github.com/Sigilweaver/OpenWRaw) (Waters),
[OpenTimsTDF](https://github.com/Sigilweaver/OpenTimsTDF) (Bruker),
[OpenARaw](https://github.com/Sigilweaver/OpenARaw) (Agilent),
[OpenSXRaw](https://github.com/Sigilweaver/OpenSXRaw) (SCIEX). Shared
foundation: [openmassspec-core](https://github.com/Sigilweaver/OpenMassSpecCore).
