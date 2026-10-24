# Vados

![Vados after destroying the planet](https://static.wikia.nocookie.net/dragonball/images/d/d0/U61.jpg)
source: [Dragon Ball Wiki](https://dragonball.fandom.com/wiki/Vados#Golden_Frieza_Saga)

## Introduction

This project will make it possible to quickly generate html files based on some file structure. It's main use case is to
make it simple to have a blog/log which can be easily maintained, having most of the content in markdown files. I create
this mainly to scratch my own itch and for learning. The scope can easily grow out of control, so I probably only add
things I would use myself. At this point it's much WIP, where braking changes can be expected. The name is based on that
it assumes [Bulma](https://bulma.io/) is assumed for the css. The generator is likely fast, and Bulma is also a
character in
[Dragon Ball Z](https://en.wikipedia.org/wiki/Dragon_Ball_Z). So I looked
for [fast Dragon Ball Z characters](https://www.cbr.com/dragon-ball-fastest-characters-ranked/) and thus ended up with
[Vados](https://dragonball.fandom.com/wiki/Vados). As it turns out she can also easily destroy and recreate planets,
which is kind of what this project does as well.

## Installation

```
cargo install vados
```

Requires a fairly recent stable Rust toolchain (see `rust-version` in `Cargo.toml`).

## Getting started

`vados init` scaffolds a fresh, deployable project into the current directory: a `root/` source tree (`main.json`,
`menu.json`, a home `page.json`, an empty `static/` directory for [static files](#static-files-and-unlisted-pages)),
an `imgroot/` image tree, a Sass entry that loads Bulma with its `$primary` overridden, and a
`netlify.toml` that builds and deploys with no further setup. It asks a few questions interactively (site title, home
intro, primary color, footer text, site language, an optional favicon file to copy in, an optional site URL, social handles) and refuses to run if anything it would create
already exists.

```
vados init
```

From there:

```
vados check --source root --img-source imgroot                       # validate before publishing
vados generate --source root --img-source imgroot --destination public  # publish the site
```

## Commands

Run `vados <command> --help` (or `vados <command> <subcommand> --help`) for the full flag list; this is a summary.

### `generate`

Reads the source and image trees and publishes a complete site to `--destination`.

```
vados generate --source root --img-source imgroot --destination public
```

### `check`

Reads the same trees as `generate` and reports every problem it can find -- unreadable files, dangling content or
image references, incomplete social links, and so on -- without writing anything. Exits non-zero iff at least one
error-severity finding turned up; warnings are reported but never fail the check. Safe (and fast) to run on every
save.

```
vados check --source root --img-source imgroot
```

### Authoring content into an existing project

Eight subcommands change one piece of an already-scaffolded project at a time, instead of reading it wholesale the way
`generate`/`check` do. Each one checks itself against the same problems `check` would find before writing anything.
Any flag left unset is asked for interactively (a plain terminal prompt); a required piece with no terminal available
fails fast with a clear message instead of hanging.

- **`vados page new --source root --path /blog/my-post --title "My Post"`** -- creates a `page.json` for a path not
  yet in the source tree. `--content` names a `.md`/`.html` file or inline HTML; omitted, the page gets a bare
  `<h1>` heading. `--image` attaches an already-declared image as the hero, given `--img-source` to resolve it
  against. `--date 2026-10-22` (or `2026-10-22T09:30`) sets its publication date; under a parent ordered by date
  (see [Blogs and dated pages](#blogs-and-dated-pages)) it defaults to today. `--child-order date-desc` (or
  `date-asc`, or `order`, the default) sets how the new page lists its own children -- e.g. for a new blog section.
- **`vados image add --img-source imgroot --dir team --file-name alice.jpg --alt-text "Alice smiling"`** -- registers
  one image in an `images.json`, creating the file if the directory has none yet. `--alt-text` is required; images
  without it fail `check`.
- **`vados page add-image --source root --img-source imgroot --path /blog/my-post --existing-reference /team/alice`**
  -- attaches an image (freshly registered via `--dir`/`--file-name`, or already declared via
  `--existing-reference`) to a page. The first image attached to a page becomes its hero; every one after becomes a
  notification, unless `--as hero`/`--as notification` overrides that.
- **`vados social add --source root --provider github --handle gklijs`** -- adds a social link, named either by
  `--provider`/`--handle` for a recognised provider (github, linkedin, facebook, youtube, twitter) or by `--url`
  (plus `--icon`/`--brand-color`/`--label` for one vados doesn't recognise automatically -- `--label` is the link's
  accessible name, since it's rendered as an icon with no visible text). `--brand-color` (hex, with or without `#`)
  also overrides a recognised provider's own color, for example to show every icon in white on a dark navbar.
  `--provider mastodon --handle you@instance` adds a Mastodon link with its icon, color and label filled in.
- **`vados social update --source root --match github --url https://github.com/newhandle`** -- replaces an existing
  social link wholesale (same naming rules as `add`). `--match` is a substring of the existing link's url; `--index`
  disambiguates when more than one matches.
- **`vados social remove --source root --match github`** -- removes an existing social link.
- **`vados footer set --source root --content footer.md`** -- replaces the site-wide footer's content reference.
- **`vados menu add-item --source root --url /blog`** -- appends one entry to the main menu. `--title` is required
  for an external (`https://`) url; an internal one falls back to its target page's own title. There's no
  `remove-item`/`update-item` yet -- edit `menu.json` by hand for that.
- **`vados page add-luma-event --source root --path /events --event-id evt-AbC123 --title "Spring jam"`** -- links
  an event hosted on [Luma](https://luma.com) to a page; see [Luma events](#luma-events). `--display` picks
  `event-page` (the default), `register-button` or `both`; `--button-label` changes the button's text (default
  "Register", in the site's language).
- **`vados page enable-feed --source root --path /blog`** -- publishes an Atom feed of a page's dated children; see
  [Feeds](#feeds). `--title` and `--limit` set the feed's title and size. A feed needs the site's address, so when
  `main.json` has no `siteUrl` yet, `--site-url https://example.com` is required (or asked for) and written there.

## Project layout

A vados project is two plain-text trees plus generated output:

- **Source tree** (`--source`): `main.json` (site title, footer, extra CSS/JS, background, navbar color, language,
  description, favicon, site URL) and `menu.json` (main menu, social links) at the root, and one `page.json` per content directory. Every directory
  under the root is a page, whether or not it has its own `page.json` -- one without gets a title from its directory
  name and a placeholder paragraph for content.
- **Image tree** (`--img-source`): one `images.json` per directory of images to publish. Each declared image is
  centre-cropped to the closest standard aspect ratio and published as a set of responsively sized WebP variants.
  A page's hero image is fetched first (`fetchpriority="high"`), and the first row of its tile images eagerly; the
  rest load lazily.
- **Destination** (`--destination`, `generate` only): the published site. Variants already there are reused rather
  than regenerated across runs, and anything belonging to an image no longer declared is pruned.

## Luma events

A page can show events hosted on [Luma](https://luma.com) in its main area, right after its own content. Each one is
listed under `lumaEvents` in the page's `page.json`; most pages link one, but any number is fine:

```json
"lumaEvents": [
  { "eventId": "evt-AbC123", "title": "Spring jam", "display": "both", "buttonLabel": "Sign up" }
]
```

- `eventId` is Luma's event ID (`evt-...`), shown in the event's embed options on Luma.
- `title` is required: it's the text of a plain link to the event on Luma, always shown, and the accessible name of
  the embedded event page.
- `display` is `eventPage` (Luma's embedded event page, which includes registration -- the default),
  `registerButton` (a button opening Luma's registration over the page), or `both`.
- `buttonLabel` is the register button's text, defaulting to "Register" in the site's language.

An event is shown only on the page that lists it, not on pages beneath it. Luma's button script is loaded only on
pages that show a register button; pages without a Luma event make no requests to Luma. `check` reports an event
ID that isn't of the form `evt-...` or an event without a title as errors (`generate` leaves such an event off the
page), and the same event listed twice on one page as a warning. Nothing is fetched from Luma by `check` or
`generate`, so whether the event actually exists isn't checked.

## Blogs and dated pages

A page can carry a publication date in its `page.json`, and a section can list its pages by that date instead of by
`order`, so a new post appears first without renumbering anything:

```jsonc
// blog/page.json
{ "title": "Blog", "content": "posts.md", "childOrder": "dateDesc" }

// blog/my-post/page.json
{ "title": "My post", "content": "post.md", "date": "2026-10-22" }
```

- `date` is `YYYY-MM-DD`, optionally with a time (`2026-10-22T09:30`) to order posts published the same day. It's
  shown under the page's title and on the tiles advertising it, written the way the site's language writes dates
  (`22 October 2026`, `22 oktober 2026`, ...).
- `childOrder` is `order` (the default: by `order`, then title), `dateDesc` (newest first) or `dateAsc` (oldest
  first). It applies wherever the section's pages are listed: the main menu dropdown, the side menu and the side
  tiles. Undated pages come after the dated ones; pages with the same date fall back to `order`, then title.
- In a `dateDesc` section, the side tiles show the newest pages.

A page can also show the newest dated pages from elsewhere -- a home page showing the blog's latest posts, at any
depth below it (`/blog/2026/my-post`):

```json
"latest": { "from": "/blog", "limit": 4, "depth": 2 }
```

`from` is a path or a list of them; `limit` defaults to 4, and `depth` (how many levels below `from` to look) to any.
They're shown as tiles with their date, summary and image, newest first, before the page's own notifications. `check`
warns about a `from` that matches no page.

`vados page new --path /blog --title Blog --child-order date-desc` creates such a section; every `page new` beneath it
then gets today's date unless `--date` says otherwise.

`check` reports a `date` it can't read as an error (`generate` treats that page as undated), and an undated page
under a parent ordered by date as a warning.

## Feeds

A page can publish an [Atom](https://en.wikipedia.org/wiki/Atom_(web_standard)) feed of its dated children, so
readers can subscribe to a blog or news section:

```jsonc
// main.json
"siteUrl": "https://example.com"

// blog/page.json
{ "title": "Blog", "content": "posts.md", "childOrder": "dateDesc", "feed": true }
```

- `feed` is `true`, or `{ "title": "My blog", "limit": 10 }` to name the feed (default: the site title plus the
  page's) and cap its size (default 20). It's published at the page's path plus `/feed.xml`, e.g. `/blog/feed.xml`,
  or `/feed.xml` for the home page.
- It lists the page's children that have a `date`, newest first whatever their `childOrder`, each with its title,
  `summary` and full content. A feed has no time zone to go by, so dates are given as UTC.
- `siteUrl` is required: a feed is read away from the site, so every link in it is given in full. Without a usable
  one, `generate` publishes the site without feeds.
- Every page of the section links the feed in its `<head>`, where feed readers look for it, and the section's own
  page shows a "Subscribe" link in the site's language.

`check` reports a feed without a usable `siteUrl`, and a `siteUrl` that isn't an absolute http(s) URL, as errors, and an
undated page under a feed (which the feed leaves out) as a warning.

## Search engines and browser tabs

Two optional `main.json` settings describe the site to search engines and browsers:

```json
"description": "A short description of the site, shown by search engines.",
"favicon": "favicon.svg"
```

- `description` becomes every page's `<meta name="description">` (and link-preview description), except on a page
  with a `summary` of its own in its `page.json`, which describes that page instead.
- Every page's `<title>` is its own title followed by the site title (`My post · My site`), so browser tabs and
  search results say whose site it is; the home page's is just the site title. `titleSeparator` changes the ` · `
  in between. Link previews get the bare page title, plus the site title as `og:site_name`.
- Link previews (LinkedIn, Mastodon, Slack, X, ...) get the page's type (`article` for a dated page), language and,
  with a `siteUrl`, its canonical URL and an image: the page's hero `image`, or else `main.json`'s `defaultImage`
  (an image reference, like a page's). vados publishes a 1200×630 JPEG crop of every image used this way next to its
  WebP variants (e.g. `/img/photos/batman-og.jpg`), since LinkedIn doesn't reliably show WebP. `check` warns when
  pages have images but there's no `siteUrl` to point previews at them.
- Every dated page carries [schema.org](https://schema.org/BlogPosting) structured data for search engines: a
  `BlogPosting` with its title, `summary`, date, preview image and author; the home page describes the site and its
  author. The author is `main.json`'s `"author": { "name": "...", "url": "..." }` (both optional, defaulting to the
  site title and home page), with the social links as their other profiles. A page's optional `updated` date (same
  form as `date`) becomes its modified date there and in feeds. `check` warns about a dated page without a `summary`.
- Social links carry `rel="me"`, so Mastodon can verify the site as yours (the green checkmark on your profile).
  Every page also names your fediverse handle as `fediverse:creator`, which Mastodon shows as the author under link
  previews: `main.json`'s `"fediverseCreator": "@you@instance"`, or else taken from a social link with the
  `mastodon` icon. `vados social add --source root --provider mastodon --handle you@hachyderm.io` adds such a link.
- `favicon` is a file in the source root -- SVG, PNG or ICO -- published at the site root and linked as every
  page's icon. `vados init` asks for one (leave it blank to skip). `check` warns when the file is missing; `generate` then publishes the site without an icon.

## Sitemap

With a `siteUrl`, vados publishes `/sitemap.xml`, listing every page (except unlisted ones) by its full address, to
submit in Google Search Console or Bing Webmaster Tools. A page's `<lastmod>` is its `updated` date, or else its
`date`; pages with neither get none, since search engines distrust a date that just records the build.

A `/robots.txt` allowing all crawlers is always published, so crawlers never get a 404 for it. With a `siteUrl` it
also names the sitemap. A `robots.txt` in the static directory replaces it.

## Static files and unlisted pages

Files a site needs at a fixed address -- `robots.txt`, search engine verification files, `.well-known/...`,
`_headers`/`_redirects` for Netlify, downloads -- go in a static directory, named in `main.json`:

```json
"staticDir": "static"
```

Everything in it is published as it is, at the same path under the site root (`static/.well-known/security.txt`
becomes `/.well-known/security.txt`). The path is relative to the source root; a directory inside it isn't a page.
Pages are written after static files, so a page at the same path replaces one; `check` warns about that, and about a
`staticDir` that doesn't exist. A `.gitkeep` is never published, so it can keep an empty static directory in git;
`vados init` scaffolds `root/static/` with one and sets `staticDir` for you.

A page with `"listed": false` in its `page.json` is published in the site's layout but listed nowhere: not in menus,
side tiles, feeds or the sitemap. That suits a 404 page: `root/404/page.json` publishes `404.html`, which Netlify,
GitHub Pages and Cloudflare Pages serve for unknown paths.

## Bulma and dark mode

Pages are styled with [Bulma](https://bulma.io/) 1.0, by default loaded from jsDelivr. Bulma follows the reader's
colour-scheme preference, so pages show its dark theme to readers who prefer one, and so does the default background
(`has-background-light-soft`). Colours a site chooses -- the navbar, notification colours, brand colours of social
icons -- stay as chosen in both themes, so pick ones that read well on either.

A page's own content sits in Bulma columns, with its left and right notifications in a two-column Bulma grid below it,
read row by row; on a phone they stack in that same order. A site that sets its own `cssFiles` must load Bulma 1.0 or
later: Bulma 0.9's tiles are no longer used, and `check` warns about a stylesheet still loading Bulma 0.x.

## Code blocks

Fenced Markdown code blocks that name a language (```` ```rust ````, ```` ```python ````, ...) are highlighted when the
site is generated: no script, and they read the same without one. Blocks without a language, in one vados doesn't
know, or indented stay plain, as does inline code. The colours (GitHub's light theme, and base16 Ocean in Bulma's dark
theme, each adjusted where needed to stay readable on Bulma's code background in that theme) come from `/css/vados-highlight.css`, which only pages with highlighted code
load. `"codeHighlight": false` in `main.json` turns it off. Building vados with `--no-default-features` leaves the
highlighter out, about 1 MB of the binary.

## Icons

Icons -- on menu entries (`icon` in `menu.json` and `page.json`), social links, breadcrumbs and the side menu -- are
[Material Design Icons](https://pictogrammers.com/library/mdi/) names (`home`, `github`, `rss`, ...), drawn inline as
SVG in the colour of the surrounding text. Pages load no icon font. `check` warns about a name that isn't in the set;
that icon is then left out. Content that uses the font's own classes (`<span class="mdi mdi-github">`) needs its
stylesheet in `cssFiles`.

## Accessibility

Every published page declares the site's language (`main.json`'s `language`, defaulting to `en`) on its root
element, offers a skip-to-content link and marked-up navigation/main/footer landmarks, and gives every icon-only
control (social links, the menu toggles) an accessible name -- a social link's is its provider's own name, or an
explicit `--label` for one vados doesn't recognise. Every declared image requires alternative text; `check` flags
one that doesn't.

A page's title is its one `<h1>`, and the titles of its tiles and Luma events are `<h2>`s under it, so headings
outline the page for search engines and for screen-reader users jumping between headings. Start a page's own content
at `##` (or `<h2>`): `check` warns about a `#` heading or `<h1>` in it, since that would make a second one.

The text vados adds itself -- the skip link, the names of the navigation landmarks and menu toggles, and a Luma
register button's default label -- follows the site language. It's available in English, Dutch, German, French and
Spanish (matched on the language alone, so `nl-BE` gets Dutch); for any other language it falls back to English and
`check` warns about it.

## Examples

A website where I share my adventures mastering bass playing serves as example. The site itself is hosted
via [Netlify](https://www.netlify.com/) and can be found [here](https://bass.gklijs.tech). The source code for the
website can be found on [Github](https://github.com/gklijs/vados_bass)

## License

This project is licensed under MIT license ([LICENSE-MIT](LICENSE-MIT) or (http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Vados by you shall be
MIT licensed as above, without any additional terms or conditions.
