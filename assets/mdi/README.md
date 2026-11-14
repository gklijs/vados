# Material Design Icons

`paths.txt` holds the SVG path of every icon in [`@mdi/svg`](https://www.npmjs.com/package/@mdi/svg) 7.4.47
(Pictogrammers; the icons are Apache-2.0, see `LICENSE`), one per line as `name path`, followed by the aliases of renamed icons as
`alias =name`. vados inlines these into pages instead of loading the icon font.

To refresh it, download the `@mdi/svg` tarball from npm and, for each entry of its `meta.json`, write the `d`
attribute of `svg/<name>.svg`'s `<path>`, then every alias not already an icon name.

## Additional provider icons

`bluesky`, `threads`, `tiktok`, `discord`, `codeberg`, `medium`, `telegram` and
`signal-messenger` are SVG paths copied without modification from
[Simple Icons 16.0.0](https://github.com/simple-icons/simple-icons/tree/16.0.0/icons)
(commit `b4ff542f289101683c2f20e446b0c191c7016e5d`), under CC0-1.0; see
`LICENSE-SIMPLE-ICONS`. `signal-messenger` comes from `signal.svg`, to avoid
replacing MDI's existing `signal` (signal strength). These paths also use a
24×24 viewBox. Keep these appended entries when refreshing the MDI set.

Default brand colours for the new providers come from that release's
`data/simple-icons.json`; email uses black. Colours are defaults, overridable
for contrast with the site's navbar (see the README's Bulma and dark mode section).
