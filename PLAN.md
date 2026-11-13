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
- [ ] Check follow-up is PR #69 (`close_64`), CI was still running on
  2026-11-13; once merged, #64 can be closed.

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
- [ ] Maintainer commits, pushes, PR, CI.
- [ ] After the first release that includes `schemas/`: check that the URLs
  resolve and an editor picks them up, then offer the schemas to SchemaStore
  (needs path patterns, since `main.json`/`menu.json`/`page.json` are common
  names). Not done here: that's an outside submission.
