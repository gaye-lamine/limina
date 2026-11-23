import { formatValue, tickStep } from './chart-geometry';

/** One mark on the dial: its value, label and angle in degrees (180 = left end, 0 = right end). */
export interface DialMark {
  value: number;
  label: string;
  angle: number;
}

/** Geometry of the semicircular gauge. */
export interface GaugeGeometry {
  /** Value at the right end of the dial: about 1.25 times the largest of claim and bounds. */
  max: number;
  /** Angle of the claim needle. */
  needleAngle: number;
  /** One tick per distinct configuration bound, ascending. */
  boundTicks: DialMark[];
  /** Graduations of the scale, from 0. */
  scaleTicks: DialMark[];
  /** Angle where the hatched zone beyond the largest bound starts; it ends at 0. */
  hatchStart: number;
  /** True when the claim is strictly beyond the largest bound. */
  claimBeyond: boolean;
}

const HEADROOM = 1.25;

/** Angle of a value on a dial from 0 to `max`; values outside are drawn at the ends. */
export function dialAngle(value: number, max: number): number {
  const v = Math.min(Math.max(value, 0), max);
  return 180 - (180 * v) / max;
}

/** Pure geometry of the gauge for a claim and the bounds of the configurations. */
export function gaugeGeometry(claim: number, bounds: number[]): GaugeGeometry {
  const distinct = [...new Set(bounds.filter((b) => Number.isFinite(b)))].sort((a, b) => a - b);
  const largestBound = distinct.length > 0 ? distinct[distinct.length - 1] : 0;
  const largest = Math.max(claim, largestBound);
  const max = largest > 0 ? largest * HEADROOM : 1;
  const step = tickStep(max);
  const count = Math.floor(max / step + 1e-9) + 1;
  const scaleTicks = Array.from({ length: count }, (_, i) => {
    const value = Number((i * step).toPrecision(12));
    return { value, label: String(value), angle: dialAngle(value, max) };
  });
  return {
    max,
    needleAngle: dialAngle(claim, max),
    boundTicks: distinct.map((b) => ({ value: b, label: formatValue(b), angle: dialAngle(b, max) })),
    scaleTicks,
    hatchStart: dialAngle(largestBound, max),
    claimBeyond: claim > largestBound,
  };
}

/** Where a bound label is drawn: baseline position and text anchor. */
export interface LabelPlacement {
  x: number;
  y: number;
  anchor: 'start' | 'middle' | 'end';
}

/** Frame of the labels outside the dial. */
export interface LabelFrame {
  cx: number;
  cy: number;
  /** Outer radius of the dial, ticks included. */
  outer: number;
  /** Radius of the label anchor points. */
  radius: number;
  /** Font size; a label box spans 0.75 size above the baseline and 0.25 below. */
  size: number;
}

/** Ticks within this many degrees of the top are labelled centred above. */
const TOP_ZONE = 6;

/**
 * Places the bound labels outside the dial. Labels right of the top start at
 * their tick and run right, labels left of it end at their tick and run left.
 * On each side, a label closer than one line to the previous one is pushed
 * down, then moved out until its box clears the dial.
 */
export function placeBoundLabels(angles: number[], frame: LabelFrame): LabelPlacement[] {
  const { cx, cy, outer, radius, size } = frame;
  const line = size * 1.15;
  const placed: LabelPlacement[] = angles.map((angle) => {
    const p = polar(cx, cy, radius, angle);
    const anchor = Math.abs(angle - 90) <= TOP_ZONE ? 'middle' : angle < 90 ? 'start' : 'end';
    return { x: p.x, y: p.y + size * 0.35, anchor };
  });
  for (const side of ['start', 'end'] as const) {
    const idx = placed.map((p, i) => i).filter((i) => placed[i].anchor === side);
    idx.sort((a, b) => placed[a].y - placed[b].y);
    for (let k = 1; k < idx.length; k++) {
      const prev = placed[idx[k - 1]];
      const cur = placed[idx[k]];
      if (cur.y - prev.y < line) cur.y = prev.y + line;
    }
    for (const i of idx) {
      const p = placed[i];
      const top = p.y - 0.75 * size;
      const bottom = p.y + 0.25 * size;
      const dy = top <= cy && bottom >= cy ? 0 : Math.min(Math.abs(cy - top), Math.abs(cy - bottom));
      const reach = dy >= outer ? 0 : Math.sqrt(outer * outer - dy * dy);
      const gap = size * 0.4;
      p.x = side === 'start' ? Math.max(p.x, cx + reach + gap) : Math.min(p.x, cx - reach - gap);
    }
  }
  return placed;
}

/** Point on a circle of radius `r` around (cx, cy) at `angle` degrees (0 = right, 90 = up). */
export function polar(cx: number, cy: number, r: number, angle: number): { x: number; y: number } {
  const rad = (angle * Math.PI) / 180;
  return { x: cx + r * Math.cos(rad), y: cy - r * Math.sin(rad) };
}

/** SVG path of the annular sector between two angles (from > to), outer radius `r`, inner `r0`. */
export function sectorPath(cx: number, cy: number, r: number, r0: number, from: number, to: number): string {
  const a = polar(cx, cy, r, from);
  const b = polar(cx, cy, r, to);
  const c = polar(cx, cy, r0, to);
  const d = polar(cx, cy, r0, from);
  const large = from - to > 180 ? 1 : 0;
  const f = (n: number) => n.toFixed(2);
  return (
    `M ${f(a.x)} ${f(a.y)} A ${r} ${r} 0 ${large} 1 ${f(b.x)} ${f(b.y)} ` +
    `L ${f(c.x)} ${f(c.y)} A ${r0} ${r0} 0 ${large} 0 ${f(d.x)} ${f(d.y)} Z`
  );
}
