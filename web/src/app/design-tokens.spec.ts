import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { contrast } from './testing-contrast';

const tokens = readFileSync(resolve(process.cwd(), 'src/tokens.css'), 'utf8');

/** Hex value of a token in tokens.css. */
function hex(name: string): [number, number, number] {
  const m = tokens.match(new RegExp(`--${name}:\\s*(#[0-9a-fA-F]{6})`));
  if (!m) throw new Error(`token --${name} is not a hex color`);
  return [1, 3, 5].map((i) => parseInt(m[1].slice(i, i + 2), 16) / 255) as [number, number, number];
}
/** `fg` at `alpha` over `bg`. */
const over = (fg: number[], bg: number[], alpha: number) => fg.map((v, i) => v * alpha + bg[i] * (1 - alpha));

const SURFACES = ['bg', 'surface-1', 'surface-2', 'surface-3'];
const VERDICTS = ['v-impossible', 'v-conditional', 'v-compatible', 'v-needs', 'v-disputed'];

describe('design tokens', () => {
  it('declares the brief palette, one accent and a 12% tint and 35% border per verdict', () => {
    for (const [name, value] of Object.entries({
      bg: '#0a0c10',
      'surface-1': '#11141a',
      'surface-2': '#171b22',
      'surface-3': '#1e232b',
      'text-primary': '#e7eaee',
      'text-secondary': '#a3acb8',
      'text-tertiary': '#6f7885',
      accent: '#7aa2ff',
      'accent-hover': '#9dbbff',
      'v-impossible': '#f2717b',
      'v-conditional': '#e8b454',
      'v-compatible': '#5fcb92',
      'v-needs': '#8d98a8',
      'v-disputed': '#b39dfa',
    }))
      expect(tokens).toContain(`--${name}: ${value};`);
    for (const v of VERDICTS) {
      expect(tokens).toMatch(new RegExp(`--${v}-bg: rgba\\([^)]*0\\.12\\)`));
      expect(tokens).toMatch(new RegExp(`--${v}-border: rgba\\([^)]*0\\.35\\)`));
    }
    expect(tokens).toContain('--ease: cubic-bezier(0.2, 0.8, 0.2, 1)');
  });

  it('meets WCAG AA (4.5:1) for body text: primary, secondary, accent and verdict colors on every surface', () => {
    for (const s of SURFACES)
      for (const t of ['text-primary', 'text-secondary', 'accent', ...VERDICTS])
        expect(contrast(hex(t), hex(s)), `${t} on ${s}`).toBeGreaterThanOrEqual(4.5);
  });

  it('keeps tertiary text at 3:1 or more, so it is used only for large text and non-text marks', () => {
    for (const s of SURFACES) {
      const c = contrast(hex('text-tertiary'), hex(s));
      expect(c, `tertiary on ${s}`).toBeGreaterThanOrEqual(3);
      expect(c, `tertiary on ${s} is below 4.5: never small text`).toBeLessThan(4.5);
    }
  });

  it('meets AA for verdict pills (color on its 12% tint) and highlighted text (primary on the tint)', () => {
    for (const s of ['surface-1', 'surface-2'])
      for (const v of VERDICTS) {
        const tint = over(hex(v), hex(s), 0.12);
        expect(contrast(hex(v), tint), `${v} pill on ${s}`).toBeGreaterThanOrEqual(4.5);
        expect(contrast(hex('text-primary'), tint), `text on ${v} tint over ${s}`).toBeGreaterThanOrEqual(4.5);
        expect(contrast(hex('text-primary'), over(hex(v), hex(s), 0.35)), `text on selected ${v}`).toBeGreaterThanOrEqual(4.5);
      }
  });

  it('meets AA for dark text on the accent button and on the verdict badges', () => {
    expect(contrast(hex('accent-ink'), hex('accent'))).toBeGreaterThanOrEqual(4.5);
    expect(contrast(hex('accent-ink'), hex('accent-hover'))).toBeGreaterThanOrEqual(4.5);
    for (const v of VERDICTS) expect(contrast(hex('accent-ink'), hex(v)), `badge ${v}`).toBeGreaterThanOrEqual(4.5);
  });
});
