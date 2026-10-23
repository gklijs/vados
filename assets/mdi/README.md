# Material Design Icons

`paths.txt` holds the SVG path of every icon in [`@mdi/svg`](https://www.npmjs.com/package/@mdi/svg) 7.4.47
(Pictogrammers; the icons are Apache-2.0, see `LICENSE`), one per line as `name path`, followed by the aliases of renamed icons as
`alias =name`. vados inlines these into pages instead of loading the icon font.

To refresh it, download the `@mdi/svg` tarball from npm and, for each entry of its `meta.json`, write the `d`
attribute of `svg/<name>.svg`'s `<path>`, then every alias not already an icon name.
