# 0.6.0-rc.1 handoff

## Published RC validation (2026-11-14)

The maintainer pushed `7f0ee2d` directly to main and tagged `v0.6.0-rc.1`.
No PR was needed. The [GitHub prerelease](https://github.com/gklijs/vados/releases/tag/v0.6.0-rc.1)
is published with both Linux archives and their checksums.

- [CI](https://github.com/gklijs/vados/actions/runs/43613012034) passed:
  full Rust tests, browser tests, minimum Rust version, formatting and Clippy.
- [Audit](https://github.com/gklijs/vados/actions/runs/43613012028) passed.
- [Release](https://github.com/gklijs/vados/actions/runs/43613012032) passed:
  both native musl builds, archive execution and publication.
- Both downloaded archive checksums match, both binaries are statically linked,
  and all three icon notices match the repository copies. The downloaded x86_64
  binary reports `vados 0.6.0-rc.1` locally; ARM execution was verified in CI.
- All four public raw schema URLs resolve and their JSON matches the committed
  schemas. Automated editor language-service checks are recorded below.
- The downloaded x86_64 musl binary rebuilt all four sites from the prior
  temporary swamp_sites snapshot. Only that copy's version pin was changed to
  `0.6.0-rc.1`. Full build pipelines and final generated-link checks passed
  with zero errors and warnings. Original sites and deployment settings were
  unchanged.

Downloads and local verification logs are under `/tmp/vados-published-rc1/`.
The sandbox-blocked swamp_sites archive-navigation and Chromium checks remain
pending; the vados repository's own browser CI passed as recorded above.

## Editor and Netlify checks

The published x86_64 RC scaffolded a fresh project in
`/tmp/vados-rc1-netlify/`, pinned to `VADOS_VERSION = "0.6.0-rc.1"`.
`netlify build --context deploy-preview` passed using the generated build
command unchanged: actual GitHub release download, checksum verification,
`npm install`, Sass compilation and `generate --check-links`. Generation
reported zero warnings and the link check passed. This build ran locally
through Netlify CLI, not on Netlify's hosted build workers.

The resulting site was uploaded as a draft under the existing blog project:

- [Preview](https://6af88c54f61310805a8105af--calm-biscochitos-e05de4.netlify.app)
- [Deploy record](https://app.netlify.com/projects/calm-biscochitos-e05de4/deploys/6af88c54f61310805a8105af)

HTTPS smoke checks passed: homepage returns 200 with the expected release-check
title, and its stylesheet returns 200 with CSS content. Netlify's production
deploy remains `6aeb5698dbac3267e667c952`; the live blog was not replaced.
Build/deploy/smoke logs are `/tmp/vados-rc1-netlify-build.log`,
`/tmp/vados-rc1-netlify-deploy.json` and `/tmp/vados-rc1-netlify-smoke.json`.

VS Code's `vscode-json-languageservice` 6.0.0-next.3 passed property completion,
valid-configuration checks and unknown-key diagnostics for all four schemas,
plus page `childOrder` enum suggestions. The language service resolved the
RC `$schema` addresses through a loader serving the previously downloaded,
verified public schema files. This checks editor language behavior, not a GUI
editor's network configuration. Runner, dependency lockfile and results are
under `/tmp/vados-rc1-editor/`. No dependencies were added to the repository.

## Preparation record

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

The checks pending at preparation are reported above with their final CI
results. The original feature-branch PR proposal was superseded by the
maintainer's direct push to main.

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

The tagged schema URLs and binary-download URLs now resolve for this RC.
