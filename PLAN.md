# Open issue implementation plan

Keep issues as separate acceptance checklists and use focused changes. Related
issues share design decisions; they do not all need to land together. Commits
and pushes remain with the maintainer.

## Order

1. Correctness: **#56 stale image variants**, then **#55 unknown JSON keys**.
   - #56 is already implemented in `b0f9e9a`; verify its regression tests.
   - #55: warn in `check`, refuse authoring rewrites that would discard unknown
     keys, and default omitted `jsFiles` and `cssFiles` to empty lists.
2. **#64 module splits**: a behaviour-preserving refactor before substantial
   further CLI/check changes. Keep moves separate from feature work.
3. Configuration and distribution:
   - **#62 JSON Schemas**, coordinated with #55 on accepted keys, defaults and
     future `$schema` preservation. Keep runtime findings useful on their own.
   - **#58 release binaries + #65 Linux/WSL support** together; decide version
     pinning before changing scaffolds.
   - **#59 dependency updates, security auditing and CI caching** separately;
     it can proceed independently.
4. Features and performance, in this order:
   - **#60 undo commands**: split into menu editing, feature removal and page
     deletion, with the last carrying its own link/deletion safeguards.
   - **#63 social providers**: decide provider scope and compatibility for
     existing custom links.
   - **#57 parallel generation**: build on #56's correct cache behaviour;
     measure performance and verify deterministic output.
5. **#61 README, configuration reference and changelog**: finish the refresh
   once configuration and installation details settle. Each earlier change
   still updates its own documentation.

**#50 real Luma verification** is independent: do it whenever a suitable real
event is available, including embed, registration button and CSP checks.

## Decisions for #55

- Unknown keys are warnings for checking/generation, but block authoring
  rewrites of the affected file.
- No `_` or `x-` exemption: these keys would also be lost when rewritten.
- `$schema` support and preservation belong to #62; until then it is unknown.
- Keep `footerContent` and page `content` required; only the two asset lists
  gain defaults as proposed in #55.

## Validation / progress

- [x] #56 existing image-cache regression tests verified (all five pass).
- [x] #55 implemented with nested-key diagnostics and rewrite protection.
- [x] Formatting and Clippy pass with default features and without default features.
- [x] New key-diagnostic tests and CLI rewrite-protection tests pass.
- [x] Full suite passes in GitHub CI: PR #66 merged with build, browser and
  minimum-Rust-version checks green.

## Progress for #64

- [x] `main.rs` split into `src/cli/` (one file per command group plus
  `prompt.rs`); `main.rs` keeps `Cli`, the top-level `Command` and dispatch.
  Each group exposes `run(command)`; `init` keeps `run_init`.
- [x] `check.rs` moved to `src/check/mod.rs` (the rename is staged on its own),
  then helpers and their tests moved to `config`, `images`, `pages`,
  `content_links`, `events`, `feeds_and_latest` and `bulma`; shared test
  fixtures went to `test_support.rs`. Only visibility changed. All 47 tests are
  kept.
- [x] All 29 `--help`/`-h` outputs, `--version`, the error cases and `check`
  on the `vados_bass` fixture are byte-identical before and after.
- [x] Formatting, Clippy (default and no default features) and the full local
  suite pass (269 library tests plus all integration suites).
- [x] Follow-up: the inline menu/social and hosting blocks of `check()` moved
  into `menu_and_social.rs` (`check_menu`, with the six menu/social tests) and
  `hosting.rs` (`check_static_dir`: missing static directory, `_headers`,
  shadowed static files). Called at the same point, so the order of findings
  is unchanged; `check` output on `vados_bass`, and on a copy broken to trigger
  the menu, icon, social and shadowing findings, is byte-identical to
  `10db018`. The `_headers` path is covered by the integration test.
- [x] First round merged as PR #67 (CLI and check splits), CI green.
- [x] Step 4: `content.rs` moved to `src/content/mod.rs` and `structure.rs` to
  `src/structure/mod.rs` (both renames staged on their own). `content` then
  split into `reference` (content references and reading them), `navigation`
  (navbar, side menu, breadcrumbs), `tiles` (notifications, `latest` tiles,
  reading order) and `page` (main content and page template); `mod.rs` keeps
  `GenericContent`, `ContentHelper` and the shared helpers. The social
  provider table, `SocialItem` and `RecognizedSocialProvider` moved to
  `structure/social.rs`. Re-exports keep every `crate::content::…` and
  `crate::structure::…` path; only imports and visibility changed, and tests
  moved with their code (269 library tests kept).
- [x] Step 4 checks: generated `vados_bass` site, generate log, `check` output
  and all `--help` outputs byte-identical to `557bf43`; formatting, Clippy
  (default and no default features) and the full local suite pass.
- [x] Step 4 merged as PR #68, CI green.
- [x] Check follow-up merged as PR #69, CI green; #64 is done.

## Decisions for #62

- `"$schema"` is always written: by `init` into every file it scaffolds, and by
  `page new`/`image add` into files they create. Authoring rewrites keep it.
  It is accepted at the top level of all four files only (an ordinary struct
  field, so #55's unknown-key check accepts it).
- Address: `https://raw.githubusercontent.com/gklijs/vados/v{version}/schemas/{file}.schema.json`,
  the repository at the release tag, so nothing depends on #58's release
  workflow. Files written by an unreleased build point at a tag that doesn't
  exist yet; editors then just show no hints.
- `check` keeps its own findings and messages and doesn't read the schemas.
- `schemars` and `jsonschema` are dev-dependencies only. The structs derive
  `JsonSchema` under `cfg_attr(test, ...)`; a unit test in `src/schemas.rs`
  fails when `schemas/` is stale (`VADOS_UPDATE_SCHEMAS=1` rewrites it).
- Draft 7, the newest draft every common editor supports. Objects have
  `additionalProperties: false`; optional fields accept `null`, since vados
  writes nulls itself.
- Patterns: page `date`/`updated`, event and Luma `start`/`end` (with a UTC
  offset, in the forms `jiff` reads), Luma `eventId` (`evt-...`) and a social
  link's hex `color`. A Luma event's `title` is required in the schema, since
  `check` reports it missing as an error.
- Field doc comments are written for site authors (they become the schema's
  descriptions); maintainer notes moved to `//` comments next to them.

## Progress for #62

- [x] Schemas generated into `schemas/` and kept in step by a unit test.
- [x] `init`, `page new` and `image add` write `$schema`; rewrites keep it;
  `check`/`generate` accept it without warnings.
- [x] Tests: `vados_bass` fixture, README JSON examples, a fresh scaffold and a
  fully authored project validate; the property tests' generated trees
  validate (except the Luma events deliberately generated with a malformed ID
  or no title); misspelt keys, enum values and malformed dates, times, IDs and
  colours are rejected.
- [x] README "Editor support" section, `vados.allium` (`$schema` accepted;
  schemas excluded as mechanics), CHANGELOG.
- [x] Formatting, Clippy (default and no default features), full local suite
  and `cargo +1.88.0 check --all-targets --locked` pass.
- [x] Merged as PR #70, CI green.
- [ ] After the first release that includes `schemas/`: check that the URLs
  resolve and an editor picks them up, then offer the schemas to SchemaStore
  (needs path patterns, since `main.json`/`menu.json`/`page.json` are common
  names). Not done here: that's an outside submission.

## Decisions for #58 and #65

- Pinned: the scaffolded `netlify.toml` sets `VADOS_VERSION` in
  `[build.environment]` to the version that ran `init`, matching the `$schema`
  addresses. The spec's `AlwaysInstallsCurrentVados` became
  `InstallsTheScaffoldingVersion`. Like `$schema`, an unreleased build scaffolds
  a version that has no release yet.
- The release workflow doesn't publish to crates.io; `cargo publish` stays
  manual.
- No runtime note on native Windows; documentation only.
- Targets `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`, each
  built natively (`ubuntu-24.04`, `ubuntu-24.04-arm`) with `musl-gcc` for
  libwebp's and ring's C code. Static binaries run on Netlify's and
  Cloudflare's images whatever their glibc.
- Archives `vados-{version}-{target}.tar.gz` holding a `vados-{version}-{target}/`
  directory (binary, README, CHANGELOG, licence), plus a `sha256sum` file.
  The Netlify build checks the archive against it; it only guards against a
  corrupt download, since the binary that scaffolds a site can't know its own
  release archive's checksum.
- The tag must equal `v` + the Cargo.toml version. A version with a
  pre-release part makes a GitHub pre-release; its notes fall back to
  "Unreleased" when it has no changelog section.
- `[profile.release]` gets only `strip = true`. Measured on the musl build:
  18.5 MB stripped; fat LTO with `codegen-units = 1` gave 16.6 MB but took 4×
  as long to build (2m19s vs 35s) and didn't make generating `vados_bass`
  faster, and the profile also applies to everyone running `cargo install`.
- musl's allocator was no slower here: generating `vados_bass` took ~3.4 s for
  both the musl and the glibc builds. Measure again for #57's parallel
  generation.

## Progress for #58 and #65

- [x] `.github/workflows/release.yml`: tag check, musl builds, packaging with
  checksum, a smoke test of each archive (checksum, static linking, `--help`,
  `--version`, generating `vados_bass`), release notes from `CHANGELOG.md`.
- [x] `[package.metadata.binstall]` and `strip = true` in `Cargo.toml`.
- [x] `init`: `netlify.toml` downloads and verifies the pinned binary;
  scaffolded README updated; unit test for the build command.
- [x] README (Installation with the platform statement, Hosting, releasing),
  `vados.allium` (Linux/WSL assumption, pinned guarantee), `.gitattributes`
  comment, CHANGELOG.
- [x] Locally: the x86_64 musl build (with Ubuntu's musl-gcc unpacked in a
  scratch directory), the workflow's package/test and release-notes steps,
  and the scaffolded Netlify command against that archive with `curl` and
  `npm` stubbed. Formatting, Clippy (default and no default features), full
  suite and the 1.88.0 check pass.
- [x] Merged as PR #71; build, browser and minimum-Rust-version checks pass
  in GitHub CI.
- [x] #65 complete: README installation guidance and the scaffolded README
  state Linux/WSL support; the spec records the platform assumption;
  `.gitattributes` keeps LF normalization for Windows-side WSL checkouts;
  release targets are Linux only. The optional native-Windows note was
  deliberately omitted (documentation only, as decided above).
- [ ] Pre-release tag (for example `v0.6.0-rc.1`, with that version in
  `Cargo.toml`) to run the workflow for real, including the aarch64 build.
- [ ] One real Netlify deploy of a site scaffolded by a released version.
  These two release/deploy checks remain follow-up validation for #58; no
  release workflow run exists yet as of 2026-11-14.

## Decisions for #59

- Dependabot rather than Renovate: no app to install. Three ecosystems (`cargo`
  at the root, `npm` in `/tests/browser`, `github-actions`), weekly, with minor
  and patch updates grouped per ecosystem and each major on its own.
- `cargo deny` rather than `cargo audit`, so one tool checks advisories and
  licences: `cargo deny check advisories licenses` in `audit.yml`, weekly and on
  changes to `Cargo.lock`, `Cargo.toml` or `deny.toml`. Bans and sources are
  left out.
- Licences: yes, now that release binaries bundle every dependency. Allow-list
  of permissive licences plus MPL-2.0 (cssparser/selectors via scraper). Data
  inside crates (tzdb, syntect dumps) falls under that crate's licence; the
  Material Design Icons in `assets/mdi` aren't a crate and keep their own
  LICENSE file.
- Audit findings on `main` today: RUSTSEC-2026-0433 (pulldown-cmark wikilinks;
  vados doesn't enable them) fixed by updating to 0.13.6 in `Cargo.lock`. The
  two unmaintained transitive crates, `basic-toml` (askama, compile time only)
  and `bincode` (syntect's bundled dumps), are ignored in `deny.toml` with
  reasons, rather than turned into issues: there's nothing to do until askama
  and syntect move on.
- `Swatinem/rust-cache` in all three CI jobs, saved only from `main`; npm cache
  for the browser job. The release workflow stays uncached.
- Bulma bumps: the browser job already compiles the README's module list against
  the bumped Bulma; the Dependabot config and README say a minor or major may
  need the list (and init's `^1.0.4`) updated.

## Progress for #59

- [x] `.github/dependabot.yml`, `deny.toml`, `.github/workflows/audit.yml`,
  caching in `ci.yml`; README "Developing vados" and CHANGELOG.
- [x] Locally: `cargo deny check advisories licenses` passes; library tests,
  the `vados_bass` integration test, formatting and the 1.88.0 check pass
  after the pulldown-cmark update. Workflow YAML parses; actionlint isn't
  installed.
- [ ] Maintainer commits, pushes, PR; audit job green in CI.
- [ ] CI time before and after the cache (the second run on `main` after merge
  is the first to restore), noted in the PR.
- [ ] Dependabot's first pull requests open after merge and pass CI.
- [ ] Not done here: the release archive doesn't include `assets/mdi/LICENSE`
  although the binary embeds the icons; consider adding it.

## Decisions for #60

- Land menu editing first, then feature removal, then page deletion with its
  own link and child-page safeguards, as specified in the order above.
- `menu remove-item` and `menu update-item` use URL-substring `--match` and
  zero-based `--index` among matches, like the social commands.
- Updates keep omitted fields. `--clear-title` and `--clear-icon` explicitly
  clear them; external links must still have a title. `--position` is the
  zero-based final position in the full menu. Duplicate URLs, invalid
  positions, missing/ambiguous matches and requests with no changes reject
  before writing. Internal forward references remain allowed.

## Progress for #60

- [x] Menu remove/update commands, partial field updates and reordering;
  README, CHANGELOG and `vados.allium` updated.
- [x] Focused CLI integration test and seven menu unit tests pass. Coverage
  includes ambiguous/indexed selection, invalid edits leaving bytes unchanged,
  moving in both directions, clearing fields, add/remove restoration,
  preservation of socials and `$schema`, and unknown-key rewrite protection.
- [x] Formatting and Clippy with default and no default features.
- [ ] Maintainer commit and push, focused PR; full suite, browser and
  minimum-Rust-version checks in CI.
- [ ] Feature removal: Luma events, feeds, agendas, search, page event and
  image attachments.
- [ ] Page deletion with child-page and incoming-link safeguards.
- [ ] Follow-up methods in vados-swamp (separate repository).

## Decisions for #63

- Include all 19 candidates: Bluesky, Instagram, Threads, TikTok, Twitch,
  Discord, Reddit, GitLab, Codeberg, Stack Overflow, Medium, Spotify,
  SoundCloud, Vimeo, Pinterest, Telegram, WhatsApp, Signal and email.
- Recognise exact HTTP(S) hosts, optionally prefixed by `www.`, and nonempty
  `mailto:` addresses. Shared `/@user` paths do not identify providers.
  Handle expansion uses user IDs for Discord/Stack Overflow/Spotify,
  international digits for WhatsApp, a share-link token for Signal and an
  address for email. Threads writes threads.com and also accepts threads.net.
- Existing custom links to newly recognised providers keep explicit icons,
  colours and labels; missing metadata gets defaults. The original five
  providers keep their canonical-icon/label precedence. All providers keep
  explicit colour overrides. Mastodon/fediverse creator inference is unchanged.
- Eight missing logos come from Simple Icons 16.0.0 (CC0), with pinned source
  and licence in `assets/mdi`. Use `signal-messenger` so MDI's signal-strength
  icon remains intact. Brand colours come from the same pinned release.
- Contrast uses the existing explicit-colour mechanism. All 19 defaults were
  compared against all ten stock Bulma navbar colours (3:1 for icons); every
  default needs an override on some backgrounds. White on Black/Dark/Link
  and black on the other stock navbars gives at least 5.14:1. README explains
  this; browser tests cover those overrides in both colour schemes. Custom
  stylesheets still need their own contrast check.

## Progress for #63

- [x] Shared provider table, host recognition, CLI values and init wizard list;
  custom-link compatibility and check's effective-icon diagnostics.
- [x] Bundled logos and provenance, README, schema descriptions, CHANGELOG and
  provider rules in `vados.allium`.
- [x] Local social/icon/check tests, all-provider CLI integration test, init
  tests and schema integration tests pass. Brand palette calculations pass.
- [x] Formatting and Clippy (default and no default features), existing
  real-site generation regression tests; browser JavaScript syntax check.
- [ ] Maintainer commit and push, focused PR; full suite, browser contrast
  checks and minimum-Rust-version checks in GitHub CI.
- [ ] Follow-up in vados-swamp: `vados.ts` has the old provider list in its
  provider schema description and `social_add` description. Its input is a
  string, so the new names work once the installed vados includes #63;
  update those descriptions separately.

## Progress for #61

- [x] Rewrote the introduction and feature links, shortened the name story and
  removed the externally hosted wiki image. Updated package description/keywords.
- [x] Added a table per configuration file, including nested keys, types, defaults
  and explanatory section links. A test compares each table against schemas
  generated from the accepted configuration structs.
- [x] Added the four current swamp_sites examples alongside the bass site.
- [x] Kept Unreleased open and documented Markdown alternates.
- [x] Schema/reference unit tests, README/schema integration tests, formatting
  and Clippy (`--all-targets -- -D warnings`) pass locally.
- [ ] Maintainer commit and push, focused PR; full suite, browser and minimum-Rust
  checks pending in CI.
