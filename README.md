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
`menu.json`, a home `page.json`), an `imgroot/` image tree, a Sass entry that overrides Bulma's `$primary`, and a
`netlify.toml` that builds and deploys with no further setup. It asks a few questions interactively (site title, home
intro, primary color, footer text, site language, social handles) and refuses to run if anything it would create
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

Six subcommands change one piece of an already-scaffolded project at a time, instead of reading it wholesale the way
`generate`/`check` do. Each one checks itself against the same problems `check` would find before writing anything.
Any flag left unset is asked for interactively (a plain terminal prompt); a required piece with no terminal available
fails fast with a clear message instead of hanging.

- **`vados page new --source root --path /blog/my-post --title "My Post"`** -- creates a `page.json` for a path not
  yet in the source tree. `--content` names a `.md`/`.html` file or inline HTML; omitted, the page gets a bare
  `<h1>` heading. `--image` attaches an already-declared image as the hero, given `--img-source` to resolve it
  against.
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
  accessible name, since it's rendered as an icon with no visible text).
- **`vados social update --source root --match github --url https://github.com/newhandle`** -- replaces an existing
  social link wholesale (same naming rules as `add`). `--match` is a substring of the existing link's url; `--index`
  disambiguates when more than one matches.
- **`vados social remove --source root --match github`** -- removes an existing social link.
- **`vados footer set --source root --content footer.md`** -- replaces the site-wide footer's content reference.
- **`vados menu add-item --source root --url /blog`** -- appends one entry to the main menu. `--title` is required
  for an external (`https://`) url; an internal one falls back to its target page's own title. There's no
  `remove-item`/`update-item` yet -- edit `menu.json` by hand for that.

## Project layout

A vados project is two plain-text trees plus generated output:

- **Source tree** (`--source`): `main.json` (site title, footer, extra CSS/JS, background, navbar color, language)
  and `menu.json` (main menu, social links) at the root, and one `page.json` per content directory. Every directory
  under the root is a page, whether or not it has its own `page.json` -- one without gets a title from its directory
  name and a bare heading for content.
- **Image tree** (`--img-source`): one `images.json` per directory of images to publish. Each declared image is
  centre-cropped to the closest standard aspect ratio and published as a set of responsively sized WebP variants.
- **Destination** (`--destination`, `generate` only): the published site. Variants already there are reused rather
  than regenerated across runs, and anything belonging to an image no longer declared is pruned.

## Accessibility

Every published page declares the site's language (`main.json`'s `language`, defaulting to `en`) on its root
element, offers a skip-to-content link and marked-up navigation/main/footer landmarks, and gives every icon-only
control (social links, the menu toggles) an accessible name -- a social link's is its provider's own name, or an
explicit `--label` for one vados doesn't recognise. Every declared image requires alternative text; `check` flags
one that doesn't.

## Examples

A website where I share my adventures mastering bass playing serves as example. The site itself is hosted
via [Netlify](https://www.netlify.com/) and can be found [here](https://bass.gklijs.tech). The source code for the
website can be found on [Github](https://github.com/gklijs/vados_bass)

## License

This project is licensed under MIT license ([LICENSE-MIT](LICENSE-MIT) or (http://opensource.org/licenses/MIT)

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Vados by you shall be
MIT licensed as above, without any additional terms or conditions.
