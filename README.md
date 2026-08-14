# my-nav

Personal navigation dashboard and password-protected report workspace for **Helloxiaolaodi**. 

This repository contains a static homepage that renders links from `links.json`, a local Tailwind runtime, and a `myfiles` explorer for shared Quarto reports. Cloudflare Pages Functions protect selected report folders.

## Features

- Sidebar link groups and tag filtering rendered from `links.json`
- Search across link titles, URLs, and tags
- Slash commands for theme and common destinations
- Google, Bing, and GitHub search engine switcher
- Light gradient and dark space themes with a background image
- Local click-frequency tracking with most-used-first sorting
- My Files explorer with folder and file metadata from `myfiles/data.json`
- Password-protected report folders backed by Cloudflare Pages Functions
- Local favicon cache under `icons/`

## Repository Layout

| Path | Purpose |
| --- | --- |
| `index.html` | Single-page navigation dashboard |
| `links.json` | Link groups, tags, VPN flags, icons, click counts, and md5 |
| `icons/` | Downloaded favicon assets referenced by `links.json` |
| `tailwind.min.js` | Local Tailwind runtime used by the dashboard |
| `myfiles/` | Shared file explorer and report pages |
| `myfiles/data.json` | Folder and file metadata for the explorer |
| `myfiles/explorer.js` | Explorer rendering, filtering, sorting, and local open action |
| `myfiles/explorer.css` | Explorer styles |
| `functions/myfiles/<area>/` | Cloudflare Pages Functions for login and protected paths |
| `_headers` | Cloudflare Pages cache headers for reports and explorer assets |

## Protected Report Areas

The following folders are exposed through `/myfiles/` and protected by the corresponding Cloudflare Pages Functions:

- `myfiles/targetc/`
- `myfiles/lucuro/`
- `myfiles/galibierhub/`
- `myfiles/global-oral/`

Each area uses the same pattern:

- `_auth.js`: password and cookie constants
- `login.js`: form POST handler
- `_middleware.js`: redirects unauthenticated requests to the login page

Update `PASSWORD` in each `_auth.js` before deploying if the shared access password should change.

## Quarto Reports

TargetC and Global Oral reports are generated from `.qmd` sources with Quarto.

The QMD files include `embed-resources: false`, so generated HTML keeps Plotly, Bootstrap, and chart data in sibling `*_files/` directories instead of inlining them. This keeps each report HTML small and lets browsers reuse cached library assets across pages.

Re-render after editing a report:

```bash
cd myfiles/targetc
quarto render TargetC-phenotypes-analysis-260814.qmd
quarto render TargetC-phenotypes-analysis-delta-260814.qmd

cd ../global-oral
quarto render global_sampling_world_map_plot.qmd
quarto render data_qc_funnel_chart_plot.qmd
```

`_headers` caches report HTML for 24 hours in the browser and 7 days on the CDN, and caches explorer CSS and JS for 7 days.

## Maintenance Scripts

Node.js scripts keep `links.json` and the favicon cache in sync.

- `node download_icons.mjs`: download missing favicons and rewrite icon paths
- `node enhance_links.mjs`: recompute tags, VPN flags, click counts, and md5
- `node repair_icons.mjs`: replace invalid local icon files from fallback services

Run these from the repository root. They require network access for icon downloads.

## Local Development

The homepage is static and can be opened directly in a browser:

```bash
start index.html
```

For the full `myfiles` experience with login redirects, deploy the repository to Cloudflare Pages. The site needs no build step; publish the repository root and let Cloudflare use the `functions/` directory automatically.

## Deployment

1. Push this repository to GitHub.
2. Create a Cloudflare Pages project connected to `https://github.com/Helloxiaolaodi/my-nav.git`.
3. Leave the build command empty and set the output directory to `.`.
4. Deploy the branch.
5. Confirm `/_headers`, `/functions/`, `/myfiles/data.json`, and `/myfiles/explorer.js` are available in the production preview.

## Data Flow

- `index.html` fetches `links.json` and renders group cards.
- Theme, selected search engine, and click stats are stored in browser `localStorage`.
- Click counts update locally and are used to move frequent links to the front.
- `myfiles/explorer.js` fetches `myfiles/data.json` and renders breadcrumbs, folders, files, and badges.
- Protected folder requests hit Cloudflare Pages Functions before the static HTML is served.

## Security Notes

- The login cookie is scoped with `Path=/`, `SameSite=Lax`, and `Secure`.
- Passwords are stored in the source `_auth.js` files; rotate them if the repository is public.
- Do not place private data inside `/myfiles/` unless it is password protected or removed from the public deployment.
