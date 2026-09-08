# Docs site

The official docs site lives in `site/`. Astro emits static pages. Pagefind provides
in-browser full-text search. Main content is read from `docs/*.md`. The catalog and
titles are defined in `site/scripts/catalog.mjs`. Code samples, internal links, and
raw file downloads are handled at build time. Rust and the web player are not
required.

The site UI is English by default and can switch to Chinese. The choice is stored
in `localStorage` as `renrs-lang`. Each catalog page has a translation in
`site/i18n/docs/`.

## Local development

Node.js 22.12 or later is required.

```sh
cd site
npm ci
npm run dev
```

The dev server defaults to `http://127.0.0.1:4321`. The full-text index is built
for production. In development, search can still find docs by page title.

## Check a production build

```sh
npm run check
npm run build
npm run preview
```

The build checks in-site pages, static assets, and hash links. Tests use a real
browser for desktop, mobile, catalog, search, theme, language, and copy. Install
Chromium first:

```sh
npx playwright install chromium
npm test
```

## Local gate and deployment

The docs quality gate runs locally. `npm run check` and `npm run build` are the
authoritative checks; run the existing `npm test` separately when browser regression
coverage is needed. This round does not add GitHub Actions CI.

The existing `.github/workflows/docs.yml` remains only as an optional artifact and
GitHub Pages publishing channel, not a replacement for local acceptance. To publish,
choose **GitHub Actions** under **Settings → Pages → Build and deployment**. The
repository must already be on GitHub. Passing local checks does not mean a live
publish succeeded.

## Site URL and subpath

The default Pages URL is derived from the current repository: a project repo
publishes to `/<repository>/`, and an `<owner>.github.io` repo publishes to `/`.
Navigation, images, the full-text index, and page assets all use the same base
path.

Override the deploy location with GitHub repository variables:

| Variable | Example | Use |
| --- | --- | --- |
| `DOCS_SITE` | `https://docs.example.com` | Canonical and sitemap origin |
| `DOCS_BASE` | `/` or `/renrs/` | Deploy subpath |

A custom domain also needs the Pages domain and DNS. This site does not assume a
real domain.

## Add a document

Add Markdown under `docs/`, then register the source file, path, titles, summary,
and group in `site/scripts/catalog.mjs`. Add the other-language translation under
`site/i18n/docs/en/` or `site/i18n/docs/zh/`. Docs can keep relative `.md` links;
the site rewrites them to page URLs. Historical records are labeled separately.
Current capability is the manuals plus [Current status](NEXT_PRODUCT_WORK.md).
