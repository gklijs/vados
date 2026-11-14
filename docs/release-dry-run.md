# Release dry run: swamp_sites

Date: 2026-11-14. Candidate: `8ca3065` (Cargo version still `0.5.0`).

Built the candidate with `cargo build --release --locked --offline`, then ran
the four sites from a temporary copy of the current local `swamp_sites` tree.
That snapshot includes uncommitted news updates in the two English sites.
The original sites, their build output and deployment configuration were not
changed. Nothing was deployed.

## Results

All four existing `scripts/build.sh` pipelines passed: content preparation,
Sass compilation, generation, shared JavaScript, redirects, asset versioning,
CSP/response headers and final `vados check --output` validation. Each reported
zero errors and zero warnings.

Separate clean serial (`--jobs 1`), clean default-parallel and repeated cached
generation runs produced identical relative filenames and SHA-256 file hashes,
including output manifests and image-cache metadata.

| Site | Generated files compared | Serial seconds | Parallel seconds | Cached seconds |
|---|---:|---:|---:|---:|
| basic-income | 1,935 | 18.29 | 2.82 | 1.48 |
| basisinkomen | 652 | 8.45 | 1.38 | 0.34 |
| neurodiversiteit | 1,035 | 12.78 | 1.61 | 0.51 |
| neurodiversity | 3,578 | 26.73 | 4.26 | 3.16 |

These are single-run observations, not a controlled benchmark or a comparison
against the published 0.5.0 binary.

Ten site regression scripts passed: alt-text, clusters, events, freshness,
import-agenda, metadata, smoke, subscribe, topics and version-assets.

## Pending validation

- `generate-pages.test.mjs` stopped with `spawnSync /usr/bin/node EPERM`.
  This is a sandbox restriction, not a failed assertion; rerun in CI.
- The Chromium accessibility review stopped with `listen EPERM` on
  `127.0.0.1`; defer to CI or an environment permitting a local HTTP server.
- This dry run does not validate release archives, aarch64 packaging, hosted
  schema URLs, Netlify deployment or live Cloudflare responses.

The unmodified site scripts accept the candidate because both it and their pin
currently report 0.5.0. This does not test installation or a version-pin upgrade.

Temporary snapshot, per-command logs, runner and machine-readable results:
`/tmp/vados-release-dry-run/` (not permanent artifacts).
