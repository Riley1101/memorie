# Assets

| File | Used by | Source |
| --- | --- | --- |
| `icon.png` | apple-touch-icon | copy of `src-tauri/icons/icon.png` (512 × 512) |
| `icon-256.png` | favicon, nav wordmark | the same icon, downscaled to 256 × 256 |
| `og-image.png` | `og:image` / social cards (1200 × 630) | the Paper page-style screenshot on the window frame colour |
| `shots/*-light.webp`, `shots/*-dark.webp` | home page hero, page-style tabs and tour | real app screenshots, one per theme |
| `pages.css` | privacy, support and changelog pages | hand-written |

The home page shows the `-light` or `-dark` screenshot to match the visitor's theme.

## Retaking the screenshots

The shots are the real app running in a browser (`bunx vite dev`) with a stubbed
Tauri `invoke` and a demo binder, captured with Playwright at 1800 px wide and
converted to WebP. Retake them whenever the editor's look changes, in both light
and dark, for each page style (`paper`, `default`, `minimal`, `technical`).
