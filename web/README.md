# axel.md

The Axel website: `apps/www` is the landing page (https://axel.md), `apps/docs` the
documentation (https://docs.axel.md, Fumadocs, pages in `apps/docs/content/*.mdx`).

```bash
pnpm install
pnpm dev:www     # http://localhost:3000
pnpm dev:docs    # http://localhost:3001
pnpm build
```

URLs live in `apps/www/src/lib/site.ts`.

`assets/make-mascot.py` cuts the mascot out of `assets/axel-mascot-source.png` (its checkerboard is painted in) and writes the hero image, icons and social preview. Run it from `web/`.

## README banner

`docs/assets/axel-banner-{dark,light}.png` (2560x800) are rendered from HTML with headless Chrome at 2x, with `web/assets/axel-mascot.png`. Shown at `width="100
## README banner

`docs/assets/axel-banner-dark.png` and `-light.png` (2560x800) are rendered from HTML with
headless Chrome at 2x, around `web/assets/axel-mascot.png`. The README shows them at
`width="100%"`, so keep them wide and short.
