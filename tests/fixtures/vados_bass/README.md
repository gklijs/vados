# vados_bass fixture

A frozen snapshot of `root/` and `imgroot/` from
[gklijs/vados_bass](https://github.com/gklijs/vados_bass), the content
repository behind the live site at <https://bass.gklijs.tech/>. Used by
`tests/vados_bass_integration.rs` as real-world content: nested pages five
directories deep, real photos (some with EXIF, one PNG with an alpha
channel), a mix of recognised and unrecognised social providers, and
notifications with and without images.

Vendored rather than cloned at test time so `cargo test` stays offline and
deterministic: it never breaks because the source repo changed or because
GitHub is unreachable.

- Source commit: `6f500051e1ec3d60a337b79113eb760525163f01` (2022-08-07)
- `root/` is `vados_bass`'s source tree (`--source` for `vados generate`).
- `imgroot/` is its image tree (`--img-source`).
- Nothing here is modified from the source repo.

## Refreshing

```sh
git clone --depth 1 https://github.com/gklijs/vados_bass /tmp/vados_bass
rm -rf tests/fixtures/vados_bass/root tests/fixtures/vados_bass/imgroot
cp -r /tmp/vados_bass/root tests/fixtures/vados_bass/root
cp -r /tmp/vados_bass/imgroot tests/fixtures/vados_bass/imgroot
```

Then update the source commit above, and re-check the content facts baked
into `tests/vados_bass_integration.rs` against <https://bass.gklijs.tech/> --
titles, subtitles, footer text and nav structure change if the real site's
content changes.
