# Releasing OpenSZRaw

Standard procedure for cutting an OpenSZRaw release. This repo ships one
crate and one PyPI package from a single tag:

| Artifact | Kind | Built by | Published to |
| --- | --- | --- | --- |
| `openszraw` | Rust crate (`crates/openszraw`) | `cargo publish` | crates.io |
| `openszraw` | maturin wheel + sdist (`crates/openszraw-py`) | `.github/workflows/publish.yml` | PyPI |

The crate version lives in one place, `[workspace.package] version` in the
root `Cargo.toml`; both `crates/openszraw` and `crates/openszraw-py` inherit
it via `version.workspace = true`. The PyPI package version is read from the
same Cargo manifest by maturin (`dynamic = ["version"]` in `pyproject.toml`),
so there is nothing to bump on the Python side.

## Steps

1. **Bump the version.** Edit `[workspace.package] version` in `Cargo.toml`.
   The version must not already exist on crates.io or PyPI - publishes are
   irreversible, you cannot overwrite or re-upload a version.

2. **Update the changelog.** In `CHANGELOG.md`, add a new
   `## [X.Y.Z] - YYYY-MM-DD` heading directly under `## [Unreleased]`, above
   the accumulated `### Added` / `### Fixed` / etc. bullets that have been
   landing there as PRs merged. Leave `## [Unreleased]` itself empty for the
   next round. Commit this as `release: vX.Y.Z`.

3. **Confirm CI and audit are green.** Run:

   ```sh
   scripts/check-release-ready.sh
   ```

   against the commit you're about to tag (defaults to `HEAD`). This checks
   the most recent `ci.yml` and `audit.yml` runs for that exact commit SHA
   and fails if either hasn't run, is still in progress, or didn't succeed.
   `publish.yml` triggers directly on the tag push and has no way to depend
   on jobs in `ci.yml` or `audit.yml` (GitHub Actions can't cross-reference
   workflow files), so this is the only gate - do not tag if it fails.

   Note: `audit.yml` only runs on pushes to `main` that touch
   `Cargo.toml`/`Cargo.lock`, plus a weekly schedule - it does not run on
   every commit. If the release commit itself didn't touch a manifest, the
   check will report no audit run for that SHA even though nothing
   dependency-related changed. In that case, check the most recent
   `audit.yml` run on `main` by hand (`gh run list -w audit.yml -L 1`) and
   confirm no `Cargo.toml`/`Cargo.lock` changes have landed since it ran,
   rather than tagging on a hard failure.

4. **Tag and push.**

   ```sh
   git tag -a vX.Y.Z -m "vX.Y.Z"
   git push origin vX.Y.Z
   ```

   The tag push triggers `publish.yml`: `cargo publish` for the crate, then
   wheel builds (Linux/macOS/Windows) plus an sdist, then the PyPI publish.

5. **Watch the run.** `gh run watch` or check the Actions tab. The PyPI job
   `needs: [build-wheels, build-sdist]`, so a runner flake in one wheel leg
   will hold up the publish rather than skip it silently.

6. **Verify.**

   ```sh
   cargo info openszraw          # or: curl -s https://crates.io/api/v1/crates/openszraw
   pip index versions openszraw  # or check https://pypi.org/project/openszraw/
   ```

   Confirm the new version shows up on both registries.

7. **Update downstream docs.** If install/status pages elsewhere (e.g.
   `docs/`) mention the crates.io/PyPI release state, update them in a
   follow-up commit (see `c191ca0` for the pattern from v0.1.0).
