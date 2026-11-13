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
- [ ] Full suite passes in GitHub CI. Local library run: 266 passed; three
  existing link-check tests were blocked by sandbox restrictions on binding a
  local HTTP server. The full run stopped there, before integration suites.
- PR #66 CI: browser and minimum-Rust-version checks passed. Both build runs
  failed on an outdated integration assertion requiring `jsFiles` and
  `cssFiles`. Updated it to require only mandatory fields and explicitly
  exclude the optional asset lists from errors; all 10 generation-error
  integration tests, formatting and default-feature Clippy pass locally.
- Next: maintainer commits and pushes the PR #66 test correction, then rerun
  GitHub CI for the full suite. #56 already exists in the checkout; no
  duplicate implementation PR is needed.
