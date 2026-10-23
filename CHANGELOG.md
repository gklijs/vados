# Changelog

## Unreleased

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
  only for pages that give a `date` or `updated`. The robots.txt allows all crawlers and names the sitemap, unless
  the static directory has its own.
- Mastodon (#20): social links carry `rel="me"`, so Mastodon can verify the site as the profile's. Pages declare
  `fediverse:creator` from `main.json`'s new `fediverseCreator`, or from a social link with the `mastodon` icon.
  `social add`/`update --provider mastodon --handle user@instance` writes such a link with the Mastodon icon,
  colour and label.
- Static files and unlisted pages (#19): `main.json`'s optional `staticDir` names a directory whose files are
  published as they are at the site root (`robots.txt`, `.well-known/...`, verification files, downloads). A
  `page.json` with `"listed": false` publishes the page but keeps it out of menus, side tiles, feeds and the sitemap,
  e.g. a `404` page. `check` warns about a missing `staticDir` and a static file a generated page replaces.

### Changed

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
