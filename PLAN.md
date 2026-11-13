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
- [ ] Open: menu/social and hosting checks are inline in `check()`, so a pure
  move gives them no file. Extracting those contiguous blocks into
  `menu_and_social.rs`/`hosting.rs` would be a follow-up. It preserves
  behaviour but isn't a pure move.
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
- [ ] Maintainer commits (renames, then the content/structure split), pushes,
  PR, CI.
