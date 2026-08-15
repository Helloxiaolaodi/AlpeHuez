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
| `panel/` | Local developer panel (Node zero-dependency server + browser UI) |
| `src-tauri/` | Tauri v2 desktop app (AlpeHuez) backend |
| `myfiles/softwares/software-data.json` | Software download list data (categories + entries) |

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

## Developer Panel

The panel (`panel/`) manages the site from a local UI — link cards, My Files, software downloads, git push, maintenance scripts, and system stats.

**Browser mode** (unchanged):

```bash
node panel/server.mjs
# open http://localhost:5173/panel/
```

**Desktop mode**: bundled inside the AlpeHuez desktop app (see below). The panel runs in its own window; open it from the homepage via the **Dev Panel** button.

Panel features:

- Login + change password (password stored in `panel/server.mjs` config / Rust command)
- Card groups, My Files folders, software downloads editing with instant save to disk
- Git status / log / push
- Maintenance script runner (`download_icons` / `enhance_links` / `repair_icons`)
- System stats and background/sidebar image management

## Desktop App (AlpeHuez)

`src-tauri/` is a Tauri v2 (Rust) desktop app that bundles the developer panel and a local preview of the website.

- Main window loads the site via the custom `nav://` protocol (`http://nav.localhost/index.html`), serving repository files locally.
- The **Dev Panel** button on the homepage opens the panel window (single instance; focus existing window if already open). The panel window is created hidden and shown only after the page finishes loading, avoiding the WebView2 `about:blank` white flash.
- Requires Rust toolchain + VS Build Tools (C++ desktop workload) on Windows.

Build:

```bash
export PATH="/d/Rust/.cargo/bin:$PATH"
RUSTUP_HOME=/d/Rust/.rustup CARGO_HOME=/d/Rust/.cargo
node /c/Users/Lenovo/AppData/Roaming/npm/node_modules/@tauri-apps/cli/tauri.js build
# release exe: src-tauri/target/release/my-nav-panel.exe
# NSIS installer: src-tauri/target/release/bundle/nsis/AlpeHuez_<version>_x64-setup.exe
```

`src-tauri/` is only static content for Cloudflare Pages and does not affect the deployed site.

## Software Download List

`myfiles/softwares/Windows Software Downloads.html` is data-driven: it fetches `myfiles/softwares/software-data.json` (9 categories, 51 entries) and renders the list — no HTML edits needed when adding software.

To edit entries:

- **From the desktop app**: open the panel → My Files → `softwares` folder → the manage (grid) button next to `Windows Software Downloads.html`.
- **From the browser panel**: same flow via `http://localhost:5173/panel/`.
- **Directly**: edit `myfiles/softwares/software-data.json` and redeploy.

Each entry: `cat`, `name`, `en`, `zh`, `url`, and optional `extra` `{en, zh, url}` (e.g. a GitHub link).

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

## Page Background Color Rules

The repository follows a layered background strategy. The rule is driven by **page depth**, not by content type.

| Page type | File(s) | Background |
| --- | --- | --- |
| Navigation dashboard | `index.html` | Light gradient **or** dark space — user switchable in the UI |
| My Files root explorer | `myfiles/index.html` | Light gradient (`#B1E3FF` -> `#F8FAFC` -> `#FFE0AE`) |
| My Files folder explorer | `myfiles/global-oral/index.html`<br>`myfiles/targetc/index.html`<br>`myfiles/lucuro/index.html`<br>`myfiles/galibierhub/index.html`<br>`myfiles/softwares/index.html` | Light gradient (same as above) |
| Login pages | `myfiles/global-oral/login.html`<br>`myfiles/targetc/login.html`<br>`myfiles/lucuro/login.html`<br>`myfiles/galibierhub/login.html` | Light gradient (same as above) |
| Report / detail HTML | `myfiles/global-oral/*.html`<br>`myfiles/targetc/*.html`<br>`myfiles/lucuro/*.html`<br>`myfiles/galibierhub/*.html` | Pure white (`#ffffff`) |
| Software download page | `myfiles/softwares/Windows Software Downloads.html` | Pure white (`#ffffff`) |

**Why some pages still look wrong online**

- If a folder index page still shows a white background after deployment, it is almost certainly **CDN cache** on `explorer.css` or the HTML itself. The `_headers` file is configured to prevent this for explorer assets and folder index pages, but you may need to wait a few minutes for the CDN edge to expire, or purge the Cloudflare cache manually.
- Re-generated Quarto reports may also keep the old background until the browser or CDN cache expires. The `index.html` files in each folder are set to `max-age=0, must-revalidate` so they refresh immediately.

**How to apply when adding new pages:**
- Folder-level index pages or login forms -> use the light gradient.
- Leaf HTML documents (reports, charts, downloads, generated pages) -> keep the background pure white so content is easy to read without competing with the surrounding page chrome.
- When a generated HTML already includes its own `body { background: ... }` rule, override it inline if it must match the white rule.

## Security Notes

- The login cookie is scoped with `Path=/`, `SameSite=Lax`, and `Secure`.
- Passwords are stored in the source `_auth.js` files; rotate them if the repository is public.
- Do not place private data inside `/myfiles/` unless it is password protected or removed from the public deployment.
