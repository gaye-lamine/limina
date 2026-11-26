import { CurvePoint, CurveResponse, SurfaceResponse } from './analysis.models';
import { formatValue } from './chart-geometry';
import { PLOT } from './workbench-logic';

/** A tick: its position in viewBox units and its label. */
export interface Tick {
  pos: number;
  label: string;
}

/** Everything the curve view draws, in viewBox units. */
export interface GraphGeometry {
  /** 'bound' draws a bound and its forbidden region; 'equality' a prediction and its tolerance band. */
  kind: 'bound' | 'equality';
  /** The decisive bound (bound) or the predicted curve, the mean of the two edges (equality). */
  line: string;
  /** Hatched region inconsistent with the law (bound laws only), else ''. */
  forbidden: string;
  /** Tolerance band between the two edges (equality laws only), else ''. */
  tolerance: string;
  /** Uncertainty band of the bound from the parameter uncertainties, else ''. */
  band: string;
  /** Rectangles of the x axis where a validity condition fails. */
  zones: { x: number; width: number }[];
  claimY: number | null;
  xTicks: Tick[];
  yTicks: Tick[];
  /** True when the x axis is logarithmic (the engine sampled it so). */
  logX: boolean;
  /** True when the y values span three decades or more and the y axis is logarithmic. */
  logY: boolean;
  /** Each defined point of the decisive curve, in viewBox units and in data units, for the crosshair readout. */
  points: { x: number; y: number; vx: number; vy: number }[];
}

const inner = () => ({
  w: PLOT.width - PLOT.left - PLOT.right,
  h: PLOT.height - PLOT.top - PLOT.bottom,
});

/** Five evenly spaced ticks from `a` to `b`. */
export function linearTicks(a: number, b: number, place: (v: number) => number): Tick[] {
  return [0, 0.25, 0.5, 0.75, 1].map((t) => {
    const v = a + t * (b - a);
    return { pos: place(v), label: formatValue(v) };
  });
}

/** The path through the defined values; an undefined value breaks the line. */
function path<T extends CurvePoint>(
  points: T[],
  value: (p: T) => number | null,
  sx: (x: number) => number,
  sy: (y: number) => number,
): string {
  let d = '';
  let pen = false;
  for (const p of points) {
    const v = value(p);
    if (v === null) {
      pen = false;
      continue;
    }
    d += `${pen ? 'L' : 'M'}${sx(p.x).toFixed(1)},${sy(v).toFixed(1)} `;
    pen = true;
  }
  return d.trim();
}

/** The closed polygon between two edges over the points where both are defined. */
function between<T extends CurvePoint>(
  points: T[],
  top: (p: T) => number | null,
  bottom: (p: T) => number | null,
  sx: (x: number) => number,
  sy: (y: number) => number,
): string {
  const ok = points.filter((p) => top(p) !== null && bottom(p) !== null);
  if (ok.length < 2) return '';
  const up = ok.map((p) => `${sx(p.x).toFixed(1)},${sy(top(p) as number).toFixed(1)}`);
  const down = ok
    .slice()
    .reverse()
    .map((p) => `${sx(p.x).toFixed(1)},${sy(bottom(p) as number).toFixed(1)}`);
  return `M${up.join(' L')} L${down.join(' L')} Z`;
}

/**
 * Geometry of the curve view of one law, from the engine's curve only. A
 * bound law shows the bound, the region above (Max) or below (Min) it, the
 * uncertainty band and the claim line. An equality law shows the predicted
 * curve, its tolerance band and the claim line. Validity zones are the x
 * intervals where the law says nothing. Null when fewer than two points are
 * defined.
 */
/**
 * The bound drawn at a point: the engine's decisive bound, except when it
 * stands for "no limit" (0 for a minimum, a stand-in a factor 1e20 or more
 * above the other end for a maximum); then the strictest bound of the
 * point, as on the metric tile.
 */
export function shownBound(p: CurvePoint, direction: 'Max' | 'Min'): number | null {
  const d = p.decisive;
  if (d === null) return null;
  if (direction === 'Min' && d === 0 && p.b_max !== null && p.b_max > 0) return p.b_max;
  if (direction === 'Max' && p.b_min !== null && p.b_min > 0 && d / p.b_min >= 1e20) return p.b_min;
  return d;
}

/** Where a value of the free parameter sits on the x axis (linear or logarithmic, as the curve). */
export function xPosition(curve: CurveResponse, v: number): number | null {
  const [lo, hi] = curve.range;
  if (v < lo || v > hi) return null;
  const { w } = inner();
  if (curve.spacing === 'log' && lo > 0) return PLOT.left + (Math.log(v / lo) / Math.log(hi / lo || 1)) * w;
  return PLOT.left + ((v - lo) / (hi - lo || 1)) * w;
}

export function graphGeometry(raw: CurveResponse, claim: number | null): GraphGeometry | null {
  const curve = raw.kind === 'equality' ? raw : { ...raw, points: raw.points.map((p) => ({ ...p, decisive: shownBound(p, raw.direction) })) };
  const equality = curve.kind === 'equality' && !!curve.lower;
  const upper = (p: CurvePoint) => p.decisive;
  const lowerPoints = curve.lower ?? [];
  const lowerAt = (i: number) => lowerPoints[i]?.decisive ?? null;
  const defined = curve.points.filter((p) => p.decisive !== null);
  if (defined.length < 2) return null;

  const ys: number[] = [];
  for (const p of curve.points) if (p.decisive !== null) ys.push(p.decisive);
  if (equality) for (const p of lowerPoints) if (p.decisive !== null) ys.push(p.decisive);
  for (const p of curve.band ?? []) {
    if (p.b_min !== null) ys.push(p.b_min);
    if (p.b_max !== null) ys.push(p.b_max);
  }
  if (claim !== null && Number.isFinite(claim)) ys.push(claim);
  const [x0, x1] = curve.range;
  const { w, h } = inner();
  const logX = curve.spacing === 'log' && x0 > 0;
  const logY = wideLogRange(ys);
  let y0 = Math.min(0, ...ys);
  let y1 = Math.max(...ys) * 1.15;
  if (y1 <= y0) y1 = y0 + 1;
  if (y0 < 0) y0 *= 1.15;
  if (logY) {
    y0 = Math.min(...ys) / 2;
    y1 = Math.max(...ys) * 2;
  }
  const sx = logX
    ? (x: number) => PLOT.left + (Math.log(x / x0) / Math.log(x1 / x0)) * w
    : (x: number) => PLOT.left + ((x - x0) / (x1 - x0 || 1)) * w;
  const sy = logY
    ? (y: number) => PLOT.top + h - (Math.log(Math.max(y, y0) / y0) / Math.log(y1 / y0)) * h
    : (y: number) => PLOT.top + h - ((y - y0) / (y1 - y0)) * h;

  const indexed = curve.points.map((p, i) => ({ ...p, i }));
  const edgeY = curve.direction === 'Max' ? PLOT.top : PLOT.top + h;
  const first = defined[0];
  const last = defined[defined.length - 1];
  const forbidden = equality
    ? ''
    : `M${sx(first.x).toFixed(1)},${edgeY} ` +
      defined
        .map((p) => `L${sx(p.x).toFixed(1)},${sy(p.decisive as number).toFixed(1)}`)
        .join(' ') +
      ` L${sx(last.x).toFixed(1)},${edgeY} Z`;
  const mean = (p: (typeof indexed)[number]) => {
    const u = upper(p);
    const l = lowerAt(p.i);
    return u === null || l === null ? null : (u + l) / 2;
  };
  const band = curve.band ?? [];
  return {
    kind: equality ? 'equality' : 'bound',
    line: equality ? path(indexed, mean, sx, sy) : path(curve.points, upper, sx, sy),
    forbidden,
    tolerance: equality ? between(indexed, upper, (p) => lowerAt(p.i), sx, sy) : '',
    band: between(
      band,
      (p) => p.b_max,
      (p) => p.b_min,
      sx,
      sy,
    ),
    zones: (curve.validity_zones ?? []).map(([a, b]) => ({
      x: sx(a),
      width: Math.max(sx(b) - sx(a), 1),
    })),
    claimY:
      claim !== null && Number.isFinite(claim) && (!logY || claim > 0) ? sy(claim) : null,
    xTicks: logX ? logTicks(x0, x1, sx) : linearTicks(x0, x1, sx),
    yTicks: logY ? logTicks(y0, y1, sy) : linearTicks(y0, y1, sy),
    logX,
    logY,
    points: defined.map((p) => ({ x: sx(p.x), y: sy(p.decisive as number), vx: p.x, vy: p.decisive as number })),
  };
}

/** Values spanning three decades or more, all positive: drawn on a log axis. */
export function wideLogRange(values: number[]): boolean {
  const v = values.filter((x) => Number.isFinite(x));
  if (v.length < 2 || v.some((x) => x <= 0)) return false;
  return Math.max(...v) / Math.min(...v) >= 1e3;
}

/** Five ticks evenly spaced in logarithm from `a` to `b` (both positive). */
export function logTicks(a: number, b: number, place: (v: number) => number): Tick[] {
  return [0, 0.25, 0.5, 0.75, 1].map((t) => {
    const v = Math.exp(Math.log(a) + t * (Math.log(b) - Math.log(a)));
    return { pos: place(v), label: v.toExponential(1) };
  });
}

/** One cell of the heat map. `level` is 0 (smallest bound) to 1 (largest); null where undefined. */
export interface HeatCell {
  x: number;
  y: number;
  width: number;
  height: number;
  level: number | null;
  /** Grey level 0 to 255 of the fill, black for the largest bound. */
  grey: number | null;
}

/** The heat map of a surface and the claim contour between cells. */
export interface HeatGeometry {
  cells: HeatCell[];
  /** Cell edges across which the bound crosses the claim: the claim contour. */
  contour: string;
  xTicks: Tick[];
  yTicks: Tick[];
  min: number;
  max: number;
}

/** Light grey for the smallest bound, mid grey for the largest. Black and white stay free for the contour. */
const GREY_LIGHT = 245;
const GREY_DARK = 110;

/**
 * Heat map of the bound over two parameters, from the engine's surface only.
 * Cells are shaded in grey by the bound; undefined cells have no level. The
 * claim contour is drawn on the edges between a cell below and a cell above
 * the claim, a cell-resolution approximation of the level line. Null when no
 * cell is defined.
 */
export function heatGeometry(s: SurfaceResponse, claim: number | null): HeatGeometry | null {
  const all = s.values.flat().filter((v): v is number => v !== null);
  if (all.length === 0 || s.x.length < 2 || s.y.length < 2) return null;
  const min = Math.min(...all);
  const max = Math.max(...all);
  const { w, h } = inner();
  const cw = w / s.x.length;
  const ch = h / s.y.length;
  const cx = (i: number) => PLOT.left + i * cw;
  const cy = (j: number) => PLOT.top + h - (j + 1) * ch;
  const cells: HeatCell[] = [];
  s.values.forEach((row, j) =>
    row.forEach((v, i) => {
      const level = v === null ? null : max > min ? (v - min) / (max - min) : 0.5;
      cells.push({
        x: cx(i),
        y: cy(j),
        width: cw,
        height: ch,
        level,
        grey: level === null ? null : Math.round(GREY_LIGHT - level * (GREY_LIGHT - GREY_DARK)),
      });
    }),
  );
  const above = (v: number | null) => (v === null || claim === null ? null : v > claim);
  let contour = '';
  s.values.forEach((row, j) =>
    row.forEach((v, i) => {
      const right = row[i + 1];
      if (
        right !== undefined &&
        above(v) !== null &&
        above(right) !== null &&
        above(v) !== above(right)
      ) {
        contour += `M${cx(i + 1).toFixed(1)},${cy(j).toFixed(1)} v${ch.toFixed(1)} `;
      }
      const up = s.values[j + 1]?.[i];
      if (up !== undefined && above(v) !== null && above(up) !== null && above(v) !== above(up)) {
        contour += `M${cx(i).toFixed(1)},${cy(j).toFixed(1)} h${cw.toFixed(1)} `;
      }
    }),
  );
  const x0 = s.x[0];
  const x1 = s.x[s.x.length - 1];
  const y0 = s.y[0];
  const y1 = s.y[s.y.length - 1];
  const px = (x: number) => PLOT.left + ((x - x0) / (x1 - x0)) * w;
  const py = (y: number) => PLOT.top + h - ((y - y0) / (y1 - y0)) * h;
  return {
    cells,
    contour: contour.trim(),
    xTicks: linearTicks(x0, x1, px),
    yTicks: linearTicks(y0, y1, py),
    min,
    max,
  };
}

/** Claim and bound on a logarithmic axis, for a law with no parameter. */
export interface ComparisonScale {
  claimX: number;
  boundX: number;
  /** Axis ticks at powers of ten. */
  ticks: Tick[];
  /** Where the region inconsistent with the law starts, on the side away from the bound. */
  hatch: { x: number; width: number };
  claimBeyond: boolean;
}

/**
 * The comparison scale of a claim and the bound that decides IMPOSSIBLE, on
 * a log10 axis spanning the decades around both. Null when either is not
 * strictly positive: a log axis cannot show it, and the caller says so.
 */
export function comparisonScale(
  claim: number,
  bound: number,
  direction: 'Max' | 'Min',
): ComparisonScale | null {
  if (!(claim > 0 && bound > 0 && Number.isFinite(claim) && Number.isFinite(bound))) return null;
  const lo = Math.floor(Math.log10(Math.min(claim, bound)) - 0.3);
  const hi = Math.ceil(Math.log10(Math.max(claim, bound)) + 0.3);
  const { w } = inner();
  const at = (v: number) => PLOT.left + ((Math.log10(v) - lo) / (hi - lo)) * w;
  const ticks: Tick[] = [];
  for (let e = lo; e <= hi; e++)
    ticks.push({ pos: PLOT.left + ((e - lo) / (hi - lo)) * w, label: `1e${e}` });
  const boundX = at(bound);
  const end = PLOT.left + w;
  return {
    claimX: at(claim),
    boundX,
    ticks,
    hatch:
      direction === 'Max'
        ? { x: boundX, width: end - boundX }
        : { x: PLOT.left, width: boundX - PLOT.left },
    claimBeyond: direction === 'Max' ? claim > bound : claim < bound,
  };
}
