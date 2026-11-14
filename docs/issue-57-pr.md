# Parallelize image processing and page publication

Prepared PR title and description; commit and push remain with the maintainer.

Generation now processes independent images and renders, highlights, minifies
and writes pages using bounded scoped workers. Each image owns its resizer;
page structure insertion stays ordered and rendering starts after sorting.
`generate --jobs N` limits workers, `--jobs 1` selects serial work, and the
existing library entry point defaults to available CPUs. There is no new
dependency. Dev builds optimize the four image codec/resize packages.

Task diagnostics replay in input order, and preflight findings sort by location
and message. Duplicate image stems and root/index page collisions retain serial
precedence. Static assets still copy before pages; shared generated assets and
manifest cleanup follow successful task completion. Failures can leave partial
output, as before, but never advance the completed manifest.

Fixes [#57](https://github.com/gklijs/vados/issues/57).

## Measurements

Linux x86_64, 22 available CPUs, release profile. Timings below are medians of
three runs, with other builds stopped. Bass uses the committed fixture (13
images); the page workload has a home page and 2,000 Markdown child pages, each
with bold text and a three-line Rust block. Filesystem caches are warm; “cold”
means an empty destination, not an OS cache flush. Run from the repository root:

```sh
# Save the release binary built before applying this change:
cp target/release/vados /tmp/vados-before
# Apply change and rebuild:
cargo build --release --locked
python3 scripts/benchmark-generation.py --before /tmp/vados-before
```

| Workload | Before | After, jobs=1 | After, default (22) | Speedup vs before |
| --- | ---: | ---: | ---: | ---: |
| Bass, empty destination | 3.226 s | 3.270 s | 0.923 s | 3.50× |
| Bass, cached variants | 0.244 s | 0.241 s | 0.049 s | 4.98× |
| 2,000 Markdown pages | 7.833 s | 7.495 s | 0.988 s | 7.93× |

All three repetitions of every workload produced identical trees across the
before binary, serial implementation and default parallel implementation.

The benchmark compares file names and SHA-256 content hashes for every file,
including image variants, image-cache records and output manifests, after each
run and across all three modes.

Temporary `Instant` instrumentation before parallelization measured the bass
fixture's decoding at 0.202 s, resizing at 0.735 s, WebP encoding at 2.711 s,
page rendering at 0.010 s and HTML minification at 0.526 s. For the 2,000-page
fixture, rendering was 0.708 s (including 0.427 s highlighting), and minification
was 7.186 s. Timers wrapped decode, each resize and encode, the page-rendering
section and minifier call; highlighting was timed inside its existing helper.
These are summed stage times from one instrumented run, not independent wall
clock measurements; rendering includes highlighting. The bass fixture does not
request link-preview crops. Instrumentation was removed before the final build.

The focused bass integration binary, including cold image generation, completed
in 19.06 s in the dev/test profile here; the issue reported about 90 s for its
unoptimized image processing alone. This is indicative, not a controlled
before/after debug benchmark.

## Validation

Passed locally:

- Two new CLI integration tests: serial/parallel/warm-cache tree and diagnostic
  equivalence; root/index and image-stem collision precedence; repeated ordered
  decode failures; previous manifest and untracked-file preservation; jobs=0 rejection.
- Worker diagnostic/order unit test and all five existing image-cache tests.
- All four real bass integration tests and all 43 site-feature integration tests.
- Formatting, whitespace checks, Clippy `--all-targets -- -D warnings` with
  default features and with `--no-default-features`.

Pending GitHub Actions after maintainer commit/push: full suite, unchanged
browser tests and minimum-Rust-version checks. Validation remains pending until
those required CI checks pass.
