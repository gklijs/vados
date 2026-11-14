# Changelog

## Unreleased

### Added

- `menu remove-item` and `menu update-item` select entries by URL substring and optional match index (#60).
  Updates keep omitted fields, can clear titles and icons, and can reorder entries with `--position`.

- Ready-made Linux binaries for each release (#58), for x86_64 and aarch64, statically linked and with a checksum,
  built and published by a workflow when a version tag is pushed. `cargo binstall vados` finds them.

- JSON Schemas for `main.json`, `menu.json`, `page.json` and `images.json` in `schemas/` (#62), so editors complete
  keys, describe them and flag mistakes while typing. They are generated from the configuration structs and checked
  in tests. `vados init`, `page new` and `image add` write `"$schema"` into the files they create, pointing at the
  schemas of the running version; every command accepts it and keeps it when rewriting a file.

- `check` warns about unknown configuration keys with nested paths and typo suggestions (#55).
  Authoring commands refuse rewrites that would discard them; omitted `jsFiles` and `cssFiles` default to empty.

- Mermaid diagrams in fenced Markdown blocks (#33), with per-page script loading,
  theme switching, readable source fallback, a self-hosting override and source checks.

### Changed

- A site scaffolded by `vados init` downloads the vados release binary on Netlify instead of installing Rust and
  compiling vados on every build (#58). It is pinned to the version that scaffolded it, as `VADOS_VERSION` in
  `netlify.toml`, so a new release can't break a deploy.

- The release binary is stripped, which makes it smaller.

- vados states that it supports Linux, including WSL on Windows (#65). Other platforms are untested.

- Dependabot keeps the crates, the browser tests' packages and the GitHub Actions up to date, a weekly audit checks
  the dependencies for RustSec advisories and disallowed licences, and CI caches compiled dependencies (#59).
- pulldown-cmark 0.13.6, for RUSTSEC-2026-0433 (wikilinks, which vados doesn't enable, so it wasn't affected).

- The library's `generator::generate` now returns `Result<GenerateReport, GenerateError>` (#54).
- `check` treats unreadable image manifests and missing image source files as errors, matching generation.

### Fixed

- Replacing an image under the same filename regenerates its responsive variants and link-preview crop (#56).
  A versioned source fingerprint cache skips decoding unchanged images and rebuilds when variants are missing
  or processing settings change.

- `generate` reports invalid source files, image-processing failures and output I/O failures with their paths and
  exits non-zero, instead of panicking or silently succeeding (#54). Configuration validation happens before any
  output is written; invalid events are still omitted and returned as counted warnings.

- `generate` removes files recorded by an earlier run that are no longer needed (#53), including removed or renamed
  pages, disabled features, static files and image variants. A destination manifest preserves cached images and
  leaves unrelated build output alone. Without a previous manifest, no files are deleted.

## 0.5.0 - 2026-11-02

### Added

- Main menu dropdowns get a real button (#34), following the WAI disclosure pattern: keyboard, touch and screen readers
  can open and close them, and `Escape` closes them or the burger menu. The button's name is translated. Without
  JavaScript hovering still opens them. No `.button` styling, so it needs no CSS of a site's own.
- `check` suggests close icon names for an unknown one (#36).
- `main.json`'s `navbarTheme` (`light` or `dark`) for a colour a site's stylesheet recolours, and the custom properties
  `--vados-nav-bg`, `-ink`, `-accent`, `-hover`, `-dropdown-bg` and `-dropdown-ink` (#37).
- `main.json`'s `menuDropdownLimit`: dropdowns stop at 12 children by default, and end in a link to the section (#35).
- `main.json`'s `spareTile`: a block of links in the cells child tiles leave over (#38).
- A README list of the Bulma modules the templates need, with a test that it stays complete (#39).
- Feeds select entries with `from`, `depth` and `includeUnlisted`, and `content: "summary"` leaves out full content.
  `check` warns about a feed with no entries, and about a `from` matching no page (#40).
- `page.json`'s `noindex` (#43), with a `check` warning for a noindexed page that is still listed.
- `main.json`'s `search`: a translated site search without outside requests, loaded only when used (#44).
- `page.json`'s `event` with strict time validation, `Event` structured data and `.ics` files, and `agenda` to list
  events (#45). Adds the `jiff` dependency, which bundles the time zone database.
- `vados check --output <dir>` and `vados generate --check-links` check the generated site's links, fragments and
  assets, without any network access (#47).
- `--external` on `check --output` and `generate --check-links` also asks other sites, as warnings only. Adds the
  `ureq` dependency, behind the default `external-links` feature.
- Plain `check` warns about links in content to pages that don't exist, without a build.
- Luma events can say when they are (`start`, `end`, `timeZone`), which lists them in agendas and calendars.
  `page add-luma-event` takes `--start`, `--end` and `--time-zone`.
- `check` warns about comment lines and over-long values in a `_headers` file, which Cloudflare Pages can reject. The
  README has a hosting section covering Cloudflare Pages and Netlify, and so does what `vados init` scaffolds.
- `list` is a short name for the `format-list-bulleted` icon.
- Menu items carry `aria-current="page"` on the current page.
- Browser tests in `tests/browser`, run in CI.
- Commands: `page set-event`, `page enable-agenda`, `page set-noindex`, `search enable`, `spare-tile set`/`remove`,
  `navbar set`, and `page enable-feed --from/--depth/--include-unlisted/--summaries-only`.

### Fixed

- A menu item is active by whole path segments: `/blog` no longer lights up on `/blogging`, and a menu entry for the
  home page is no longer active on every page.
- An empty feed no longer changes on every build: its `updated` is the page's own date, or a fixed one.

### Changed

- The inline `<style>` changed and now comes before the stylesheets, so a rule of a site's own with the same weight
  wins. Sites that hash it for a Content-Security-Policy get a new hash.
- A main menu too wide for its bar wraps instead of running off the screen, and the page stays clear of the taller bar
  (#35). The hard-coded 1023px rule is gone, so a raised `$navbar-breakpoint` just works.
- The current page in the navbar takes the bar's text colour instead of Bulma's link colour, plus heavier text and a
  line. Every `navbarColor` now reaches at least 4.5:1 with Bulma's defaults; so do its burger, which used the link
  colour too, and dropdown text (#37).
- An icon name vados doesn't know no longer leaves an empty box (#36).
- `vados init`'s Netlify build and README use `--check-links`, so a broken internal link fails a new site's deploy.
- `rust-version` is now 1.88.0, which is what the dependencies have needed for a while, and CI keeps it true.
- Without a spare tile, a child tile with an image no longer stretches into an empty block; the cell stays empty (#38).
- Sites with a navigation script of their own that adds buttons to `.has-dropdown` now get two: remove it, or guard it
  against `[data-nav-disclosure]`.

## 0.4.0 - 2026-10-24

### Added

- `"colorScheme": "light"` or `"dark"` in `main.json` pins every page to that Bulma theme (`data-theme` on `<html>`)
  instead of following the reader's colour-scheme preference, for a site with a fixed palette. An unknown value is a
  `main.json` error in `check`.
- `check` warns about a `sass/main.scss` that still loads Bulma with `@import`.
- A README section on moving an existing site to Bulma 1.0: `@use ... with`, the removed `$colors` map, footer colour
  variables and the tile CSS vados no longer emits.

### Changed

- **Breaking:** Bulma 1.0 (#31). The default stylesheet is `bulma@1.0.4`. Pages are laid out with Bulma columns and a
  two-column Bulma grid instead of Bulma 0.9's tiles, which Bulma 1.0 no longer has. A site that sets its own
  `cssFiles`, or compiles Bulma through `package.json`, must use Bulma 1.0 or later, or its pages lose their layout;
  `check` fails on a stylesheet or dependency still on Bulma 0.x.
- Dark mode: Bulma follows the reader's colour-scheme preference. The default background is now
  `has-background-light-soft`, which follows it too (`has-background-light` stays light in the dark theme).
  Highlighted code gets a dark set of colours (base16 Ocean) in Bulma's dark theme; both sets stay at 4.5:1 or more
  on Bulma's code background in their theme.
- Child tiles are placed row by row, the left one of each pair first, so the reading order is the document order at
  every width; the inline CSS `order` and the custom CSS that stacked the columns on a phone are gone. The page
  still fills evenly: the shorter column's last tile spans the rows left over, the tiles grow to end with a taller
  side column. With `"stretchSideNotification": true` in `main.json` the last side notification also grows to end with
  the page's content; it is off by default. The page's own content box now spans its column.
- Notifications and the navbar carry the Bulma theme matching their colour (`theme-light` or `theme-dark`), so
  Markdown headings, code and menus on them stay readable in either colour scheme.
- `vados init` scaffolds `"bulma": "^1.0.4"` and a `sass/main.scss` that uses `@use "bulma/sass" with ($primary: ...)`
  instead of the deprecated `@import`; `npm run build` passes `--load-path=node_modules --quiet-deps`. An existing
  site moving to Bulma 1.0 can make the same change by hand.

## 0.3.0 - 2026-10-23

### Added

- Atom feeds: a page with `"feed": true` (or `{ "title": ..., "limit": ... }`) in its `page.json` publishes
  `<path>/feed.xml`, listing its dated children newest first with their summary and full content. Every page of the
  section links the feed in its `<head>`, and the section's page shows a "Subscribe" link in the site's language.
- `main.json` takes an optional `siteUrl`, which feeds need to give their links in full. `vados init` asks for it
  (leave it blank to set it later).
- `vados page enable-feed --path /blog [--title] [--limit] [--site-url]` turns on a page's feed, and records the site
  URL in `main.json` when it has none yet.
- `check` reports a feed without a usable `siteUrl`, and a malformed `siteUrl`, as errors, and an undated page under
  a feed as a warning.
- `page.json` takes an optional `date` (`YYYY-MM-DD`, optionally with `THH:MM[:SS]`), shown under the page's title
  and on side tiles advertising it, written the way the site's language writes dates.
- `page.json` takes an optional `childOrder`: `order` (the default), `dateDesc` or `dateAsc`, so a blog or news
  section can list its newest posts first without setting `order` by hand. Undated pages come last. In a `dateDesc`
  section the side tiles show the newest pages.
- `vados page new --date` and `--child-order order|date-desc|date-asc`; under a parent ordered by date the date
  defaults to today.
- `check` reports an unreadable `date` as an error, and an undated page under a parent ordered by date as a
  warning.

- Link previews and canonical URLs (#16): every page declares `og:type` (`article`, with `article:published_time`,
  for a dated page), `og:locale`, a `twitter:card` and `robots: max-image-preview:large`; with a `siteUrl`, also
  `<link rel="canonical">`, `og:url` and an `og:image` with its size and alt text. The image is the page's hero
  image or `main.json`'s new `defaultImage`, published as a 1200×630 JPEG crop (`<name>-og.jpg`), since LinkedIn
  doesn't reliably show WebP. `check` warns when pages have images but no `siteUrl`, and about a `defaultImage`
  that matches no declared image.

- JSON-LD structured data (#17): a dated page carries a schema.org `BlogPosting` (headline, description, dates,
  image, author with the social links as `sameAs`, page URL); the home page a `WebSite` and its `Person`.
  `main.json` takes an optional `author` (`name`, `url`), also used as the feeds' author, and `page.json` an optional
  `updated` date, used as the modified date in structured data and feeds. `check` warns about a dated page without
  a `summary`, and reports an unreadable `updated` like an unreadable `date`.

- `sitemap.xml` and `robots.txt` (#18): with a `siteUrl`, the sitemap lists every listed page, with a `<lastmod>`
  only for pages that give a `date` or `updated`. A robots.txt allowing all crawlers is always published (#29),
  naming the sitemap when there is a `siteUrl`, unless the static directory has its own.
- Mastodon (#20): social links carry `rel="me"`, so Mastodon can verify the site as the profile's. Pages declare
  `fediverse:creator` from `main.json`'s new `fediverseCreator`, or from a social link with the `mastodon` icon.
  `social add`/`update --provider mastodon --handle user@instance` writes such a link with the Mastodon icon,
  colour and label.
- Syntax highlighting (#10): fenced Markdown code blocks in a known language are highlighted at build time, with
  CSS classes and a small stylesheet (`/css/vados-highlight.css`, linked only by pages showing highlighted code)
  whose colours all reach 4.5:1 contrast on Bulma's code background. `main.json`'s `"codeHighlight": false` turns it
  off; the cargo feature `highlight` (on by default) leaves it out of the build.
- Latest pages (#26): `page.json`'s `"latest": { "from": "/blog", "limit": 4, "depth": 2 }` shows the newest dated
  pages below one or more sections, at any depth, as tiles before the page's own notifications, e.g. a blog's latest
  posts on the home page. `check` warns about a `from` matching no page.
- Static files and unlisted pages (#19): `main.json`'s optional `staticDir` names a directory whose files are
  published as they are at the site root (`robots.txt`, `.well-known/...`, verification files, downloads). A
  `page.json` with `"listed": false` publishes the page but keeps it out of menus, side tiles, feeds and the sitemap,
  e.g. a `404` page. `check` warns about a missing `staticDir` and a static file a generated page replaces.
  `vados init` scaffolds `root/static/` (holding only a `.gitkeep`, which is never published) and sets
  `"staticDir": "static"` (#29).

### Changed

- Icons are inlined as SVG instead of loaded from the Material Design Icons web font (#22): no more ~400 KB of
  render-blocking CSS and font from a CDN for a handful of icons. The default CSS no longer includes the icon font,
  and `vados init` no longer adds it to `cssFiles`. **Content that uses the font's classes itself**
  (`<span class="mdi mdi-github">`) now needs the stylesheet in `cssFiles`. Icons come from `@mdi/svg` 7.4.47, so
  names renamed since 6.5.95 still work by their old name. `check` warns about an unknown icon name.
- Faster Largest Contentful Paint (#23): a page's hero image is fetched with `fetchpriority="high"`, and the first row
  of child-tile images loads eagerly instead of lazily. Every `<img>` gives `width` and `height`, and its fallback
  `src` is the widest variant up to 986 pixels instead of the 159 pixel thumbnail, for browsers and crawlers that
  don't read `srcset`.
- Every page's `<title>` now ends with the site title (`My post · My site`), except the home page's, which is just
  the site title, and pages declare the site title as `og:site_name` (#15). `main.json`'s optional `titleSeparator`
  changes the ` · `.
- A page's title is now an `<h1>`, and the titles of tiles and Luma events are `<h2>`s, so the headings outline the
  page (#14). Bulma styles them by class, so nothing looks different. `check` warns about a top-level heading (`#` or
  `<h1>`) in a page's content, since that would make a second one.
- A page without content of its own gets a placeholder paragraph instead of a heading.
- The default CSS links Bulma 0.9.4 instead of 0.9.3, and dependencies are updated.

### Fixed

- A social link `color` written with a leading `#` (`"#ffffff"`) rendered as `##ffffff` and was dropped; it is now
  read with or without the `#`, and `social add`/`update --brand-color` store it without (#21).
- An explicit `color` on a recognised provider's social link (GitHub, LinkedIn, ...) is no longer ignored, so a site
  can show all its social icons in one color (#21).
- X/Twitter links show a boxed X in black instead of the old bird in Twitter blue (#21).
- On phones, child tiles read in the order they pair up on wider screens (left 0, right 0, left 1, ...) instead of
  the whole left column first (#24).
- On phones, at most two social icons sit next to the site title; the others are in the burger menu, as a row. The
  burger can no longer be pushed off-screen, and a long site title is cut off with an ellipsis instead (#25).

## 0.2.0 - 2026-10-14

### Added

- `main.json` takes an optional `description`, published as every page's `<meta name="description">` and
  `og:description` unless the page has a `summary` of its own.
- `main.json` takes an optional `favicon`: a file in the source root (SVG, PNG or ICO), published at the site root
  and linked as every page's icon. `check` warns when the file doesn't exist. `vados init` asks for one, copying the
  given SVG, PNG or ICO file into the source root; leaving the answer blank skips it.

### Changed

- The text vados adds to every page -- the skip link, the navigation, breadcrumb and menu-toggle names, and a Luma
  register button's default label -- now follows the site language, in English, Dutch, German, French and Spanish.
  `check` warns when the site language is none of these, since that text then falls back to English.

### Fixed

- The current page in the breadcrumbs was announced as "page" instead of its title; it's now marked with
  `aria-current="page"`.

- A generated `<img>`'s `src` carried the width descriptor that belongs only in `srcset` (e.g.
  `src="/img/logo-w159.webp 159w"`), so browsers without srcset support requested a file that doesn't exist.
  `src` is now the bare URL of the smallest variant.

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
