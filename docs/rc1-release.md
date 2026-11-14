# 0.6.0-rc.1 handoff

## Prepared PR

Title: Prepare 0.6.0-rc.1 and include bundled icon notices

Description:

Prepare the first 0.6 release candidate to validate Linux binary distribution
before the stable release. Set Cargo.toml and Cargo.lock to `0.6.0-rc.1` and
give the accumulated changelog its own dated release section. Release archives
now carry the MDI and Simple Icons licences and provenance; the archive smoke
test compares these files with their repository copies. Refs #58, #65, #59.

Local validation passed: offline release build, 11 schema/scaffold tests,
formatting, Clippy with default and no default features, release workflow YAML
parsing, and the workflow's package, checksum,
notice comparison, binary version, fixture generation and prerelease-notes
steps using the host binary. Static musl linking and aarch64 execution were
not exercised locally. The earlier four-site dry run is recorded in
`docs/release-dry-run.md`.

Pending CI: full Rust suite, browser tests, minimum Rust version and dependency
audit. The tag-triggered release workflow and a real Netlify deployment remain
release validation, not completed checks. Commit and push remain with the
maintainer; open/update this PR once the feature branch is pushed.

## Release sequence

1. Review, commit and push the preparation on a feature branch. Merge after
   candidate CI passes. No commit, push, tag or publication was made here.
2. Tag the reviewed commit `v0.6.0-rc.1` and push that tag. Its version must
   match Cargo.toml. The workflow publishes a GitHub **prerelease**, with notes
   from the explicit RC changelog section. It does not publish to crates.io.
3. Inspect the x86_64 and aarch64 musl jobs and their archive smoke tests.
   Download each archive/checksum pair and verify checksums and bundled notices;
   execute binaries on their matching architectures.
4. Verify the four tagged schema URLs, for example
   `https://raw.githubusercontent.com/gklijs/vados/v0.6.0-rc.1/schemas/page.schema.json`,
   and editor schema completion.
5. Scaffold a fresh site with the released candidate, confirm Netlify's
   `VADOS_VERSION = "0.6.0-rc.1"`, and perform a real Netlify deployment.
   Existing swamp_sites scripts pin 0.5.0; they require an explicit pin update
   to exercise an installed RC through their normal build wrapper.
6. Complete the sandbox-blocked swamp_sites archive-navigation and Chromium
   checks in an environment that permits subprocess capture and local HTTP.
7. If these checks pass, prepare 0.6.0 with its final changelog and version.

The schema URLs and binary-download URLs cannot resolve until the RC tag and
release artifacts have been published.
