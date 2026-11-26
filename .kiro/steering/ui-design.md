---
inclusion: fileMatch
fileMatchPattern: "web/**"
---
# UI design rules (dark premium theme)

## Stack
- Tailwind CSS v4 through PostCSS, Angular CDK (Overlay for the palette, A11yModule for focus traps), lucide-angular as the one icon set, Inter and JetBrains Mono variable fonts self-hosted from npm. No CDN, no remote assets. Check every new package against the Angular version in use before installing, and pin it exactly.
- Icons are functional only: close, chevrons, download, external link, copy, search. No emoji anywhere.

## Tokens
- All colors, radii, spacing, shadows, type sizes and motion values come from `web/src/tokens.css`, documented in `docs/design-system.md`. Never hard-code a new color in a component.
- Dark theme only.
  - Surfaces: bg #0A0C10, surface-1 #11141A, surface-2 #171B22, surface-3 #1E232B.
  - Borders: rgba(255,255,255,.08), strong rgba(255,255,255,.16).
  - Text: primary #E7EAEE, secondary #A3ACB8, tertiary #6F7885. Tertiary is only for large text and non-text marks, because it is below 4.5:1.
- ONE accent, #7AA2FF (hover #9DBBFF), for focus rings, selection, links and the primary button (accent background, dark text).
- Verdict colors, always with a text label: impossible #F2717B, conditional #E8B454, compatible #5FCB92, needs-input #8D98A8, disputed #B39DFA. Each one has a 12% tinted background and a 35% border.
- Radii: 8 / 12 / 16 / 999. Spacing: 4px scale. Layered soft shadows.
- Type scale: 12 / 13 / 14 / 16 / 20 / 28 / 44. Headings: Inter 600, tracking -0.02em. Values: tabular mono numerals. No visible text under 12px.
- Motion: 160-240 ms with cubic-bezier(.2,.8,.2,1). Count-up on headline numbers, staggered reveals of 40 ms. Everything is off under prefers-reduced-motion.
- Background: a subtle radial accent glow at 8% at the top center and a faint 32px grid that fades out. Backdrop blur only on the top bar and the drawers. No heavy glassmorphism, no noise.

## Screens
- Top bar, 56px: wordmark, engine status (online or offline, extraction available or not), Library, How it works, Ctrl+K hint.
- Input: the headline "Check any technical claim against the laws of physics", one supporting line, and the input card (surface-2, hairline border, radius 16, glow on focus) with the textarea, a character counter and the primary Analyze button. Below it, three illustrative sample chips, and an image drop zone only when vision works.
- Running: the input collapses into a compact bar. Claim cards stream in (skeleton shimmer, then content) under a stepper (Reading, Identifying laws, Researching sources, Computing) whose active step is animated. The counters tick up.
- Reviewed:
  - A summary strip: large numerals, then one pill per verdict that also filters.
  - A compact claim list: verdict dot, quantity, value. Arrow keys and J/K navigate.
  - Left pane (42%): the text with each claim highlighted inline (verdict tint, 2px underline, numbered badge). Clicking selects the claim.
  - Right pane (58%): the instrument card. It holds the verdict pill, one plain sentence, three metric tiles (Claim, Bound, Ratio), then the gauge or the explorer with compact controls, the Source accordion (domain, quote, date, status), and the "How this was checked" button opening the right-side trace drawer.
- Command palette (Ctrl or Cmd+K): jump to a claim, open Library or How it works, download the Markdown report, toggle the trace drawer.
- Designed states: empty, loading, error toasts, offline, extraction unavailable (the built-in samples still work through the engine), and the "Download review" Markdown report.
- Both panes fit at 1920x1080 and 1440x900 without page scroll. A pane may scroll internally only for overflow content, with a thin styled scrollbar. Below 1024px everything stacks.

## Review details (polish pass)
- Highlights: the whole claim sentence gets a light verdict tint. Only the value gets the 2px underline, drawn with an animation, plus the numbered badge. The selected sentence glows softly in its verdict color.
- Claim chips: short labels only, never cut with an ellipsis. They show the law's short quantity name (`UI.short`, or the first two words of the quantity) and the value, in one horizontal scroll row with edge fades.
- Running state: split view. The reading panel on the left shows paragraphs not yet answered dimmed. The live feed on the right shows claim cards and skeletons. The stepper moves only on real events: a request sent, an answer that carries the server's law identification, the limits loaded, the solver done.
- A claim without a verdict (disputed, needs input, no law, not analysed) never shows empty metric tiles or an empty visual. It shows a card instead:
  - "What happened".
  - "What was tried": the trace, one short line per step.
  - "What would settle it".
  - Disputed: both candidate formulas and their engine-computed bounds at the claim, with the divergence.
  - Needs input: an input for each missing quantity and "Re-evaluate", which calls `POST /v1/evaluate`.
  - No law: the sources searched.
- A claim with a verdict shows exactly one visual: the Carnot or COP apparatus, the gauge (Max law without parameters), or the curve explorer. A panel with no content is not rendered.
- Surfaces: background steps and a faint top inset highlight instead of borders. The instrument crossfades over 160 ms when the claim changes. The wordmark is a small SVG mark (an arc meeting a dashed limit line) next to the name.
- The DOM check (`web/e2e/ui.spec.ts`) also fails on a scroll container nested in another one, and on a key element overflowing its box.

## Charts and apparatus
- Keep every computation and geometry function. Restyle only.
- Curve and map: a smooth accent line, and the region inconsistent with the law filled with the impossible color at 14% over a subtle diagonal pattern. Direct labels, a crosshair readout that follows the pointer, and an animated claim line.
- Gauge: an arc with a gradient track and an animated needle.
- Carnot and COP apparatus: 1.5px light strokes on the dark surface, and accent flow arrows with a moving dash (direction only, constant speed). Clean mono labels; italic serif only for the symbols T, Q and W. Keep the proportional arrow widths and the text alternatives.

## Always
- No software vocabulary is ever shown to the user: no engine or service status, no model names, no error codes, no step ids, no API, server, pipeline, extraction, JSON or version numbers. Say what happened in plain words ("This paragraph could not be read.", "Limina is unavailable right now."). Physics words stay (heat engine, law, bound). Technical details belong in the logs and in the downloaded report only. Unit and e2e tests fail on these words.
- The verdict is never encoded by color alone: the text label is always shown.
- WCAG 2.2 AA contrast (tested from the token pairs), full keyboard use, visible focus, labelled fields, aria-live for results.
- Display the API wording as received; never invent verdict text; never use the words fraud, lie, scam or hoax.
- All user-visible strings live in `web/src/app/strings.ts`.
- Quality gate: `npm test`, `npm run lint`, `npm run build`, and `npx playwright test` (screenshots into docs/screens/, with DOM checks for horizontal overflow, overlapping boxes, text under 12px, page scroll on desktop sizes and visible focus).
