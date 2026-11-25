---
inclusion: fileMatch
fileMatchPattern: "web/**"
---
# UI design rules
- Palette: black, white and light greys only. No accent color of any kind.
- No emoji. No icon fonts, no pictograms, no external assets. Hierarchy comes from type size and weight, tabular numerals, thin rules and white space.
- Verdicts are never encoded by color. IMPOSSIBLE: solid black block with a white uppercase label. CONDITIONAL: dashed border. COMPATIBLE: thin solid border on a white background. INSUFFICIENT_DATA: dotted grey border. The text label is always shown.
- Typography: system UI sans-serif stack for text, system monospace for numbers and units. No external fonts, CDN or remote assets.
- Visual direction: editorial and restrained, scientific-chart sobriety, developer-tool seriousness. Nothing decorative.
- Accessibility: WCAG 2.2 AA contrast, full keyboard use, visible focus, labelled form fields, aria-live for results, prefers-reduced-motion respected, no meaning carried by color alone.
- Wording: display the API wording field as received; never invent verdict text; never use the words fraud, lie, scam, hoax.
- All user-visible strings live in one file so that French and English can be added later.
- Layout: one column on narrow screens, two columns from 960px, bounded content width.
- Technical schematic SVG drawn in code is allowed and encouraged, in the style of a textbook or lab-notebook figure: 1.5px black strokes, hatching for fills, dimension lines with arrowheads, leader lines for callouts, labels in system monospace, symbols such as Th, Tc, Qh, Qc, W in italic serif with real subscripts, a caption "Fig. 1: ..." in italic serif under each figure, graduated scales.
- Page background: very light neutral grey (paper). A faint 24px grid appears behind the apparatus only.
- The wording sentence may use the system serif stack (Georgia, Times New Roman, serif) for editorial contrast.
- Native form controls must be restyled: appearance none, 1px solid border, no browser default chrome, visible custom focus ring.
- Animation shows direction only (for example a moving dash along a flow arrow); speed never encodes a value; no animation under prefers-reduced-motion.
