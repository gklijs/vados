# 0.6.0 release handoff

## Prepared PR

Title: Release vados 0.6.0

Promote the validated 0.6.0-rc.1 candidate to stable. Update the package and
lockfile version and publish the accumulated changelog under 0.6.0, preserving
the RC history. Runtime code and dependency versions are unchanged from the RC.
Include the RC validation record and current plan progress. Refs #58, #65.

RC evidence: full Rust suite, browser tests, minimum Rust version, dependency
audit, both native Linux musl archive smoke tests, downloaded checksums and
licences, published schemas, all four swamp_sites builds, editor language-service
checks, and a real Netlify draft deployment passed. See `docs/rc1-release.md`.
Netlify's build ran through the local CLI; hosted build workers and a graphical
editor session were not tested. Two downstream swamp_sites checks blocked by
the sandbox remain recorded follow-ups, not completed validation.

Stable-version local checks passed: offline release build, 11 focused
schema/scaffold tests, formatting, Clippy with default and no default features,
`cargo package --locked --offline
--allow-dirty` including compilation of the packaged crate, and the release
workflow's host archive/checksum/notice/version/fixture/notes smoke checks.
The notes check selects the 0.6.0 section and classifies it as a normal release.
Logs are `/tmp/vados-060-package.log` and
`/tmp/vados-060-release-smoke.log`.
Full stable-commit CI and tag-triggered Linux release builds remain pending.
If pushed on a feature branch, use this text for the focused PR. A direct push
to main needs no PR. Commit, push and tag operations remain with the maintainer.

## Publication

1. Review, commit and push these changes. Wait for CI and Audit on the exact
   stable commit to pass.
2. Tag that reviewed commit `v0.6.0` and push the tag. The release workflow
   builds and tests both static Linux binaries and publishes a normal GitHub
   release using the 0.6.0 changelog section.
3. Check release CI, downloaded checksums, notices and the four stable schema
   URLs. Stable schema URLs do not exist until the tag is pushed.
4. Publishing to crates.io remains a separate manual `cargo publish` step;
   the GitHub workflow does not publish the crate.

No commit, push, stable tag or stable publication has been made by the agent.
