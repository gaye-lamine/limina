# Limina design system

Dark theme only. Every value below is a CSS variable in `web/src/tokens.css`, the single source for the web app. `web/src/styles.css` maps the tokens into Tailwind's theme and holds the component classes.

## Stack and why

All versions are pinned exactly in `web/package.json`. Each one was checked against Angular 21.2.26 before install.

| Package | Version | Why | Compatibility check |
|---|---|---|---|
| tailwindcss, @tailwindcss/postcss, postcss | 4.3.5, 4.3.5, 8.5.33 | Tailwind v4 is the current stable line. It gives preflight, a theme mapped from our tokens (`@theme inline`) and utilities. The component classes use `@apply` and token variables, so templates stay readable. | Angular's `@angular/build:application` runs PostCSS from `web/.postcssrc.json`. No Angular peer dependency. |
| @angular/cdk | 21.2.14 | `Overlay` hosts the command palette. `A11yModule` provides `cdkTrapFocus` for the palette and both drawers. | peer `@angular/core ^21.0.0 \|\| ^22.0.0` |
| lucide-angular | 1.0.0 | One icon set, used only for functional icons: close, chevrons, download, copy, search, enter, image upload. | peer `@angular/core 13.x - 21.x` |
| @fontsource-variable/inter, @fontsource-variable/jetbrains-mono | 5.3.0 | Self-hosted variable fonts, bundled by the build. No CDN. | CSS only, listed in `angular.json` styles |
| primeng, @primeuix/themes | 21.1.10, 2.0.3 | Drawer, Accordion and Timeline for "How this was checked": a mature, accessible component set (focus trap, Escape, keyboard accordion) instead of hand-made markup. The Aura preset is mapped onto our tokens in `web/src/app/primeng-theme.ts` (dark only, accent #7AA2FF). PrimeNG styles sit in a `primeng` CSS layer, so the app's styles win. The drawer is inside `@defer (when howOpen())`; the theme engine (about 61 kB compressed) loads at start. | primeng 21.1.10 peers `@angular/core ^21.0.7`, cdk, forms and router ^21 (PrimeNG 22 needs Angular 22, so it was not used); themes 2.0.3 uses `@primeuix/styled` ^0.7.4 like primeng 21 |
| pdfmake (+ @types/pdfmake dev) | 0.3.15 (0.3.5) | The downloadable review is a PDF a non-specialist can open, built in the browser from engine results. It is loaded on demand: pdfmake (273 kB compressed) and its Roboto fonts (315 kB) download only when the user presses "Download PDF". | no Angular peer; listed in `allowedCommonJsDependencies` |
| @playwright/test (dev) | 1.65.1 | Screenshots and DOM checks in a real browser. It uses the installed Microsoft Edge (`channel: 'msedge'`), so there is no browser download. | independent of Angular |
| @types/node (dev) | 20.19.43 | Lets the contrast test read `tokens.css` with Node. | matches Node 20 |

Rejected options:
- Angular Material: its theme fights the brief's palette, and only the CDK primitives are needed.
- Icon fonts: they are not tree-shaken and they render pictograms as text.
- A `?raw` CSS import for the contrast test: the unit-test builder returned no text for it.

## Color

| Token | Value | Use |
|---|---|---|
| `--bg` | #0A0C10 | page |
| `--surface-1` | #11141A | panes, claim list, inputs |
| `--surface-2` | #171B22 | cards, input card, tiles, visual |
| `--surface-3` | #1E232B | secondary buttons, toasts, skeletons |
| `--border` | rgba(255,255,255,.08) | hairlines |
| `--border-strong` | rgba(255,255,255,.16) | controls, drawers |
| `--text-primary` | #E7EAEE | text |
| `--text-secondary` | #A3ACB8 | every small secondary text |
| `--text-tertiary` | #6F7885 | **large text and non-text marks only** (step dots, skipped trace) |
| `--accent` / `--accent-hover` | #7AA2FF / #9DBBFF | focus rings, selection, links, primary button (dark text `--accent-ink`) |

Verdicts are always shown with their text label. Each verdict has a 12% tint (`-bg`) and a 35% border (`-border`):

| Verdict | Color |
|---|---|
| impossible | #F2717B |
| conditional | #E8B454 |
| compatible | #5FCB92 |
| needs input (also insufficient data, not analysed) | #8D98A8 |
| disputed | #B39DFA |

`--v-impossible-fill` (14%) fills the region inconsistent with a law in charts, under a diagonal pattern.

### Contrast (WCAG 2.2, tested in `web/src/app/design-tokens.spec.ts`)

| Pair | Ratio | Result |
|---|---|---|
| Primary on bg / s1 / s2 / s3 | 16.22 / 15.28 / 14.31 / 13.08 | pass |
| Secondary on the four surfaces | 8.53 to 6.88 | pass |
| Accent on the four surfaces | 7.87 to 6.34 | pass |
| Each verdict color on the four surfaces | 5.40 or more | pass |
| Verdict color on its own tint over s2 | 4.94 (needs) to 7.21 | pass |
| Dark ink on the accent button | 7.87 | pass |
| **Tertiary on the four surfaces** | **4.38 / 4.13 / 3.87 / 3.53** | **fails AA for normal text** |

Tertiary passes only the 3:1 threshold for large text and non-text marks, so it is restricted to those uses. The test fails if tertiary ever reaches 4.5, so that this rule gets reviewed again.

## Shape, space, type, depth

- Radii: `--radius-sm` 8, `--radius-md` 12, `--radius-lg` 16, `--radius-pill` 999.
- Spacing: a 4px scale (`--space-1` 4 up to `--space-12` 48).
- Type scale: 12 / 13 / 14 / 16 / 20 / 28 / 44 px.
  - Inter Variable for text.
  - Headings use weight 600 and tracking -0.02em.
  - JetBrains Mono Variable with tabular numerals (`.num`, `.mono`) for values, units and engine wording.
  - Italic serif (`--font-symbol`) only for the symbols T, Q and W in the apparatus figures.
  - No visible text is under 12px; the Playwright check enforces this.
- Shadows: `--shadow-1` to `--shadow-3` are layered soft shadows. `--shadow-focus` is the 1px accent ring plus a 4px halo. `--glow-focus` is the input card glow.
- Background: a radial accent glow at 8% at the top center and a 32px grid at 2.5% white, masked so the grid fades out downwards. Backdrop blur appears only on the top bar and the drawers.

## Motion

- Durations are 160, 200 and 240 ms, all with `cubic-bezier(.2,.8,.2,1)`.
- Headline numbers count up over 600 ms (`CountUp` directive).
- Stream cards reveal with a 40 ms stagger.
- The active step pulses and sweeps. Skeletons shimmer. The claim line and the apparatus flow dashes move at a constant speed, which shows direction only.
- The gauge needle sweeps in from the left end of the dial.
- Under `prefers-reduced-motion: reduce`, every animation and transition is off (global rule), and the count-up writes the final value at once. Both behaviours are tested (`count-up.spec.ts`, and the e2e check that no element has an animation name).

## Components

- **Top bar** (56px, sticky, blurred):
  - Wordmark.
  - Engine status: online (green dot), online without extraction (amber dot), offline (red dot), with text.
  - Library and How it works links.
  - Ctrl+K hint, which is also a button.
- **Input card**: surface-2, hairline border, radius 16, glow on focus. It holds the textarea, a character counter and the primary Analyze button.
- **Sample chips**: three illustrative samples fill the textarea.
- **Drop zone**: shown only when the vision capability is available.
- **Running**:
  - A compact one-line bar with the counters.
  - A four-step stepper: Reading, Identifying laws, Researching sources, Computing.
  - Claim cards streaming in, with skeletons for the paragraphs still pending.
- **Summary strip**: a large numeral, then an All chip and one verdict pill per family. A pill is a toggle filter.
- **Claim list**: verdict dot, number, quantity and value. J/K and the arrow keys move through the visible claims.
- **Reading pane** (42%): each claim's evidence is highlighted with its verdict tint, a 2px underline and a numbered badge. It is keyboard reachable.
- **Instrument card** (58%):
  - Verdict pill, quantity, law.
  - One plain sentence from engine data, and the solver sentence.
  - The engine wording as received.
  - Three metric tiles: Claim, Bound, Ratio.
  - A visual: the gauge for a Max law without parameters, otherwise the explorer, with the Carnot or COP apparatus beside it.
  - A Source accordion: domain, quote (with a copy button), date, status, law, references.
- **Charts**:
  - Accent bound line with a soft glow.
  - The inconsistent region filled with the impossible color at 14% plus a faint diagonal pattern.
  - Direct labels ("bound", "claim").
  - A crosshair readout that follows the pointer, and an animated dashed claim line.
  - Heat map cells shaded with the accent.
- **Drawers**: How this was checked (run log and per-claim trace), and Library / How it works. Both are right-side, focus-trapped and closed by Escape or the scrim.
- **Command palette** (Ctrl or Cmd+K, CDK overlay): jump to a claim, open Library or How it works, download the report, toggle the trace drawer, start a new analysis.
- **Toasts**: errors stay until dismissed; info messages leave after 4 s.

## Polish pass

- **Surfaces:** cards use background steps (surface-1 for panes, surface-2 for tiles and parts, surface-3 for candidates) and an inset 1px top highlight at 4 to 5% white. Borders are kept only where they carry meaning: verdict pills and the ratio tile.
- **Reading highlights:** the whole sentence is tinted with the verdict at 12%. The value alone carries a 2px underline, drawn in 520 ms, and the numbered badge. The selected sentence gets a 1px verdict ring and a soft glow in its color.
- **Claim chips:** `shortLabel` gives the law's short quantity name (`UI.short`) and the value. Chips sit in one scroll row with 16px edge fades; nothing is cut.
- **No-verdict card:** "What happened", "What was tried", "What would settle it", plus candidates with their engine bounds (disputed), inputs and "Re-evaluate" (needs input), or the sources searched (no law).
- **Crossfade:** the instrument body fades in over 160 ms on every claim change.
- **Wordmark:** a 20px SVG with an accent arc reaching a dashed limit line, and a dot.

## Printed review (PDF)

- A4 page, white background, Roboto (bundled with pdfmake).
- Contents: a header with the name and the date, the title, the summary counts and how to read the verdicts. Then one block per claim: the verdict, one plain sentence, the quote, the claim, bound and ratio, the law, the status, the formal statement and the references. Claims without a verdict show what would settle them and the sources searched instead. The footer carries the page number and "Generated by Limina; AI-written laws are not validated".
- Verdict colors for paper are darker than the screen tokens so they stay readable on white. Each passes 4.5:1 on white and on its own tint (tested in `review-pdf.spec.ts`):

| Verdict | Ink | Tint |
|---|---|---|
| impossible | #B42332 | #FCE8EA |
| conditional | #8A5A00 | #FBF1DE |
| compatible | #1E7A4C | #E3F4EA |
| needs input | #4F5B6B | #EEF0F3 |
| disputed | #6040C8 | #EEE9FD |

## Image of a text

The drop zone has five states: idle, dragging, reading, done and error.
- **Idle:** a "Choose an image" button opens the file picker; the zone also accepts a dropped image, and an image pasted anywhere on the page.
- **Dragging:** the zone highlights in the accent color.
- **Reading:** a spinner and "Reading the text of <file>…" are shown.
- **Done:** the text box is filled and a toast confirms it.
- **Error:** a plain message is shown.

PNG, JPEG and WebP are accepted. An image larger than about 1.8 MB of base64 is redrawn in the browser at 2000 px on its longest side as JPEG, because the server refuses request bodies over 2 MB.

## Initial bundle

About 618 kB raw (146 kB compressed) at start, mostly the app and the PrimeNG theme engine. The warning budget is 700 kB and the error budget 1 MB. pdfmake and its fonts are lazy chunks.

## Layout

- From 1024px wide and 700px high, the app is one screen: no page scroll. The reading and instrument panes scroll internally only when their content overflows, with a thin styled scrollbar.
- Below 1024px everything stacks and the page scrolls.
- Screenshots of every state at 1920x1080, 1440x900 and 390x844 are in `docs/screens/`, regenerated by `npx playwright test` in `web/` (the app and the API must be running).
