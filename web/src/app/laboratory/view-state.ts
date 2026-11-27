/**
 * Pure helpers of the laboratory's 3D views (spec laboratory-3d, R-05):
 * where a value goes in the scene, and which side of a drawn bound a point
 * is on. Geometry only: the verdict shown is always the engine's.
 */
import { LimitInfo } from '../api.models';
import { isLog } from './lab-model';

/** Half width of the scene's floor used by the views, in scene units. */
export const HALF = 8;
/** Lowest and highest drawing heights. */
export const FLOOR = 0.3;
export const CEIL = 8;

/** A map from a range of values to a range of scene coordinates, linear or logarithmic. */
export interface Scale {
  lo: number;
  hi: number;
  log: boolean;
  to: (v: number) => number;
  /** The value at a scene coordinate (inverse of `to`, without its clamp): for dragging. */
  from: (c: number) => number;
}

export function scale(lo: number, hi: number, out0: number, out1: number, log = isLog(lo, hi)): Scale {
  const useLog = log && lo > 0 && hi > 0;
  const f = (v: number) => (useLog ? Math.log10(Math.max(v, Number.MIN_VALUE)) : v);
  const a = f(lo);
  const b = f(hi);
  const span = b - a || 1;
  return {
    lo,
    hi,
    log: useLog,
    to: (v: number) => {
      const t = (f(v) - a) / span;
      return out0 + Math.min(1.2, Math.max(-0.2, t)) * (out1 - out0);
    },
    from: (c: number) => {
      const v = a + ((c - out0) / (out1 - out0 || 1)) * span;
      return useLog ? 10 ** v : v;
    },
  };
}

/** The finite values among `values`, with the claim and the bound, give the height range. */
export function heightRange(values: (number | null | undefined)[], claim: number, bound: number | null): [number, number] {
  const all = [...values, claim, bound].filter((v): v is number => typeof v === 'number' && Number.isFinite(v));
  if (all.length === 0) return [0, 1];
  let lo = Math.min(...all);
  let hi = Math.max(...all);
  if (lo === hi) {
    lo = lo === 0 ? -1 : lo * 0.5;
    hi = hi === 0 ? 1 : hi * 1.5;
  }
  // A positive range keeps a little room under its lowest value.
  if (lo > 0 && !isLog(lo, hi)) lo = Math.max(0, lo - (hi - lo) * 0.1);
  return [Math.min(lo, hi), Math.max(lo, hi)];
}

/** Whether a claim at this height lies past a drawn bound value: above a maximum, below a minimum. */
export function pastBound(claim: number, bound: number, direction: LimitInfo['direction']): boolean {
  if (direction === 'Min') return claim < bound;
  if (direction === 'Max') return claim > bound;
  return false;
}

/** The core radius of the chamber, relative to the shell: the claim over the bound, kept visible. */
export function coreRatio(claim: number, bound: number | null): number {
  if (bound === null || !Number.isFinite(bound) || bound <= 0 || !Number.isFinite(claim)) return 0.5;
  return Math.min(2.5, Math.max(0.05, claim / bound));
}

/** Narrowest drawn width of a zone, so a zone of a single point stays visible. */
export const MIN_ZONE = 0.12;

/**
 * The scene spans of the corridor's zones outside the law's validity
 * domain (`validity_zones` of `/v1/curve`), clamped to the floor and
 * widened to MIN_ZONE when narrower; empty spans and zones off the range are dropped.
 */
export function zoneSpans(zones: [number, number][] | undefined, xs: Scale): [number, number][] {
  const out: [number, number][] = [];
  for (const [a, b] of zones ?? []) {
    if (!Number.isFinite(a) || !Number.isFinite(b)) continue;
    const lo = Math.min(a, b);
    const hi = Math.max(a, b);
    if (hi < xs.lo || lo > xs.hi) continue;
    let x0 = Math.max(-HALF, xs.to(Math.max(lo, xs.lo)));
    let x1 = Math.min(HALF, xs.to(Math.min(hi, xs.hi)));
    if (x1 - x0 < MIN_ZONE) {
      // Centred on the zone, slid back inside the floor at its ends.
      const mid = Math.min(HALF - MIN_ZONE / 2, Math.max(-HALF + MIN_ZONE / 2, (x0 + x1) / 2));
      x0 = mid - MIN_ZONE / 2;
      x1 = mid + MIN_ZONE / 2;
    }
    out.push([x0, x1]);
  }
  return out;
}

/** Whether a surface cell lies past the bound: all four corners defined and their mean value passed. */
export function cellPast(corners: (number | null)[], claim: number, direction: LimitInfo['direction']): boolean {
  if (corners.some((c) => c === null)) return false;
  const mean = (corners as number[]).reduce((a, b) => a + b, 0) / corners.length;
  return pastBound(claim, mean, direction);
}

/** Positions of the discrete ticks of an axis: `n` evenly spaced marks from one end to the other. */
export function ticks(from: number, to: number, n = 5): number[] {
  if (n < 2) return [from];
  return Array.from({ length: n }, (_, i) => from + ((to - from) * i) / (n - 1));
}

/** Value of the 3D grid of a slider in its range; null when it has no axis in this view. */
export function axisValue(value: number | undefined, s: Scale | null): number | null {
  return value === undefined || s === null ? null : s.to(value);
}
