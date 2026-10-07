# Changelog

## 0.1.0 - 2026-10-07

The first release with a `vados` command-line tool; 0.0.3 was a library only. The library's
`generator::generate(source, img_source, destination)` is unchanged.

### Added

- `vados generate`: publishes a site from a source tree and an image tree.
- `vados check`: reports every problem it can find in the source and image trees -- unreadable files, dangling
  content or image references, missing alt text, incomplete social links, and more -- without writing anything.
  Exits non-zero only on errors; warnings are reported but never fail it.
- `vados init`: scaffolds a fresh, deployable project (source tree, image tree, a Sass entry overriding Bulma's
  `$primary`, and a `netlify.toml` that builds and deploys it), asking for the site title, home intro, primary
  color, footer, language and social handles.
- Content-authoring commands that change one piece of an existing project, each checked against the same problems
  `check` finds before anything is written: `page new`, `image add`, `page add-image`,
  `social add`/`update`/`remove`, `footer set` and `menu add-item`.
- Luma events: a page can list events hosted on [Luma](https://luma.com) under `lumaEvents` in its `page.json`,
  shown after its content as Luma's embedded event page, a register button, or both. Added with
  `vados page add-luma-event`. See the README's "Luma events" section.
- Accessibility: `main.json`'s `language` (default `en`) is declared on every page; every page has a skip link and
  navigation/main/footer landmarks; icon-only controls (social links, menu toggles) carry an accessible name.
  Social links for providers vados doesn't recognise take a `label`.
- Social links recognise `x.com` as well as `twitter.com`.

### Changed

- All dependencies updated to their current major versions; the image crate is trimmed to the formats vados
  reads.
- Requires Rust 1.87 or newer.

### Fixed

- A single-page site (a root page with no children) no longer panics.
- Images with partial transparency no longer come out corrupted in every responsive variant after the first.
- Resizing no longer hits undefined behaviour in `fast_image_resize`'s AVX2 path on real photos (upgraded to 6.x).
