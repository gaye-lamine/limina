/**
 * The physical apparatus of a law in the laboratory (spec laboratory-3d,
 * R-11): which test bench shows it, chosen from its declared dimensions
 * and labels only, so that a law written by AI gets a bench too. No law is
 * named here. Also the pure quantities the benches draw; none of them is a
 * verdict, which always comes from the engine.
 */
import { LimitInfo } from '../api.models';

/** flow: energy and heat; trajectory: fields, motion, relativity; wave: beams and optics; quantum: discrete states, confinement, decay. */
export type Apparatus = 'flow' | 'trajectory' | 'wave' | 'quantum';

/** Unit symbols of each bench, as `GET /v1/limits` declares them. */
const UNITS: Record<Apparatus, readonly string[]> = {
  flow: ['J', 'kJ', 'MJ', 'eV', 'Wh', 'kWh', 'W', 'mW', 'kW', 'MW', 'GW', 'W/m^2', 'lm/W'],
  trajectory: ['kg', 'g', 'mg', 't', 'm/s', 'km/s', 'N', 'm/s^2', 'm^3/s^2', 'kg/m^3', 'g/cm^3', 'au', 'C', 'mC', 'uC', 'nC', 'T', 'mT', 'V/m', 'kV/m'],
  wave: ['Hz', 'kHz', 'MHz', 'GHz', 'THz', 'PHz', 'nm', 'um', 'µm', 'pm', 'rad', 'arcsec', 'bit/s', 'kbit/s', 'Mbit/s', 'Gbit/s'],
  quantum: ['kg*m/s', 'J*s', 'J/K'],
};

/** Words of a parameter's label or id that name what a bench shows. */
const CUES: Partial<Record<Apparatus, RegExp>> = {
  wave: /wavelength|frequency|aperture|bandwidth|signal-to-noise|resolution|spectral/i,
  quantum: /\blevel\b|quantum|half-life|decay|lifetime|uncertainty|occupation/i,
};

/** Order used to break a tie: the most specific bench first. */
const ORDER: Apparatus[] = ['wave', 'quantum', 'trajectory', 'flow'];

function benchOfUnit(unit: string): Apparatus | null {
  for (const a of ORDER) if (UNITS[a].includes(unit)) return a;
  return null;
}

type LawMeta = Pick<LimitInfo, 'claim_unit' | 'parameters' | 'parameter_labels'>;

/**
 * Scores, per bench: the claim unit counts 2 (a ratio claim counts 1 for
 * the energy bench: an efficiency); each parameter 1 by its first unit
 * that belongs to a bench; a temperature 3 for the energy bench; a label
 * cue 3. The highest score wins, ties in ORDER; nothing matched: quantum.
 */
export function apparatusScores(limit: LawMeta): Record<Apparatus, number> {
  const score: Record<Apparatus, number> = { flow: 0, trajectory: 0, wave: 0, quantum: 0 };
  const claim = benchOfUnit(limit.claim_unit);
  if (claim) score[claim] += 2;
  else if (limit.claim_unit === 'dimensionless') score.flow += 1;
  for (const p of limit.parameters) {
    if (p.kind === 'AbsoluteTemperature' || p.kind === 'TemperatureDifference') score.flow += 3;
    const unit = p.units.map(benchOfUnit).find((a) => a !== null) ?? null;
    if (unit) score[unit] += 1;
    const words = `${p.id} ${limit.parameter_labels?.[p.id] ?? ''}`;
    for (const a of ORDER) if (CUES[a]?.test(words)) score[a] += 3;
  }
  return score;
}

export function apparatusFor(limit: LawMeta): Apparatus {
  const score = apparatusScores(limit);
  let best: Apparatus = 'quantum';
  let top = 0;
  for (const a of ORDER)
    if (score[a] > top) {
      best = a;
      top = score[a];
    }
  return best;
}

/** The parameter that counts discrete states (a level or quantum number), if any. */
export function discreteParameter(limit: LawMeta): string | null {
  const p = limit.parameters.find(
    (q) => q.units.includes('dimensionless') && (q.id === 'n' || /\blevel\b|quantum number/i.test(limit.parameter_labels?.[q.id] ?? '')),
  );
  return p?.id ?? null;
}

/** The parameter that sets a mass or a gravitational strength, if any. */
export function massParameter(limit: LawMeta): string | null {
  return limit.parameters.find((p) => p.units.some((u) => ['kg', 'g', 'm^3/s^2'].includes(u)))?.id ?? null;
}

/** The parameter of the wavelength or frequency, and which of the two. */
export function spectralParameter(limit: LawMeta): { id: string; as: 'wavelength' | 'frequency' } | null {
  for (const p of limit.parameters) {
    const words = `${p.id} ${limit.parameter_labels?.[p.id] ?? ''}`;
    if (/wavelength/i.test(words) && p.units.includes('m')) return { id: p.id, as: 'wavelength' };
    if (p.units.includes('Hz') && /frequency/i.test(words)) return { id: p.id, as: 'frequency' };
  }
  return null;
}

/** The parameter that sizes the aperture or lens of the optics bench, if any. */
export function apertureParameter(limit: LawMeta): string | null {
  return limit.parameters.find((p) => /aperture|diameter/i.test(`${p.id} ${limit.parameter_labels?.[p.id] ?? ''}`))?.id ?? null;
}

/** Speed of light, m/s (exact, SI). */
export const C = 299_792_458;

// ---------- energy bench ----------

/** Shares of the drawn flows: input 1; useful output and rejected heat; nothing rejected past the limit. */
export function flowShares(ratio: number | null, impossible: boolean): { input: number; work: number; rejected: number } {
  if (impossible) return { input: 1, work: 1, rejected: 0 };
  const r = ratio === null || !Number.isFinite(ratio) ? 0.5 : Math.min(1, Math.max(0, ratio));
  const work = 0.1 + 0.8 * r;
  return { input: 1, work, rejected: 1 - work };
}

/** Turns per second of the converter: follows the claimed over the limit, kept readable. */
export function converterSpeed(ratio: number | null): number {
  if (ratio === null || !Number.isFinite(ratio)) return 0.4;
  return Math.min(3, Math.max(0.15, ratio));
}

// ---------- field bench ----------

/** Depth of the potential well, from the strength's place on its slider (0 to 1). */
export function wellDepth(strength: number): number {
  return 0.8 + 3.2 * Math.min(1, Math.max(0, strength));
}

/** Speed as a fraction of light, for the relativistic squash of the probe; 0 when the claim is not a speed. */
export function beta(claim: number, unit: string): number {
  if (unit !== 'm/s' || !Number.isFinite(claim) || claim <= 0) return 0;
  return Math.min(0.995, claim / C);
}

/** Length of a body moving at `b` relative to its own (Lorentz contraction), kept visible. */
export function contraction(b: number): number {
  return Math.max(0.1, Math.sqrt(1 - b * b));
}

/** Points of the probe's path in the floor plane: an outward spiral that escapes, or an inward one that falls in. */
export function trajectory(escapes: boolean, steps = 120, from = 2.2, out = 9, core = 0.7): [number, number][] {
  const pts: [number, number][] = [];
  for (let i = 0; i <= steps; i++) {
    const t = i / steps;
    const r = escapes ? from + (out - from) * t * t : from - (from - core) * Math.sqrt(t);
    const a = t * Math.PI * (escapes ? 2.2 : 3.2);
    pts.push([r * Math.cos(a), r * Math.sin(a)]);
  }
  return pts;
}

// ---------- optics bench ----------

/**
 * Display colour of a wavelength in nm (approximation of the visible
 * spectrum by linear pieces, 380 to 780 nm); outside it, a dim violet or red.
 */
export function spectrumRgb(nm: number | null): [number, number, number] | null {
  if (nm === null || !Number.isFinite(nm)) return null;
  if (nm < 380) return [0.35, 0.1, 0.55];
  if (nm > 780) return [0.55, 0.05, 0.05];
  const [r, g, b] = visible(nm);
  // Dimmer at the ends of the eye's range.
  const f = nm < 420 ? 0.3 + (0.7 * (nm - 380)) / 40 : nm > 700 ? 0.3 + (0.7 * (780 - nm)) / 80 : 1;
  return [r * f, g * f, b * f];
}

/** Full-brightness colour of a visible wavelength, by linear pieces between the hues. */
function visible(nm: number): [number, number, number] {
  if (nm < 440) return [(440 - nm) / 60, 0, 1];
  if (nm < 490) return [0, (nm - 440) / 50, 1];
  if (nm < 510) return [0, 1, (510 - nm) / 20];
  if (nm < 580) return [(nm - 510) / 70, 1, 0];
  if (nm < 645) return [1, (645 - nm) / 65, 0];
  return [1, 0, 0];
}

/** The wavelength in nm of a wavelength (m) or frequency (Hz) value. */
export function toNanometres(value: number, as: 'wavelength' | 'frequency'): number | null {
  if (!Number.isFinite(value) || value <= 0) return null;
  return as === 'wavelength' ? value * 1e9 : (C / value) * 1e9;
}

/** First kind Bessel function of order 1, by its integral (Simpson, 32 steps): enough for a drawing. */
export function besselJ1(x: number): number {
  const n = 32;
  const h = Math.PI / n;
  let s = 0;
  for (let i = 0; i <= n; i++) {
    const t = i * h;
    const w = i === 0 || i === n ? 1 : i % 2 === 1 ? 4 : 2;
    s += w * Math.cos(t - x * Math.sin(t));
  }
  return (s * h) / 3 / Math.PI;
}

/** Airy pattern intensity at distance `r`, its first dark ring at `r = width`. */
export function airy(r: number, width: number): number {
  const x = (3.8317 * Math.abs(r)) / width;
  if (x < 1e-6) return 1;
  const v = (2 * besselJ1(x)) / x;
  return v * v;
}

/**
 * Separation of the two drawn peaks, in units of the first dark ring: the
 * claim's place relative to the limit (resolved at 1 or more, merged below),
 * held on the side of the engine's verdict so the picture never contradicts it.
 */
export function peakSeparation(ratio: number | null, direction: LimitInfo['direction'], impossible: boolean, compatible: boolean): number {
  let q = ratio === null || !Number.isFinite(ratio) || ratio <= 0 ? 1 : direction === 'Min' ? ratio : 1 / ratio;
  q = Math.min(2.5, Math.max(0.15, q));
  if (impossible) q = Math.min(q, 0.6);
  else if (compatible) q = Math.max(q, 1.1);
  return q;
}

// ---------- quantum bench ----------

/** Radii of the drawn levels 1..count, as n² (Bohr model), fitted under `outer`. */
export function shellRadii(count: number, outer = 7): number[] {
  const k = Math.max(1, Math.floor(count));
  return Array.from({ length: k }, (_, i) => (outer * (i + 1) ** 2) / k ** 2);
}

/** The level number to highlight: the parameter's value, rounded, at least 1. */
export function levelOf(value: number | null): number {
  return value === null || !Number.isFinite(value) ? 1 : Math.max(1, Math.round(value));
}

/** Half width of the confinement box, from the confining parameter's place on its slider (0 to 1). */
export function boxHalfWidth(position: number): number {
  return 1.5 + 5 * Math.min(1, Math.max(0, position));
}

/** Amplitude of the wave packet: stronger as the box tightens (confinement raises the spread of momentum). */
export function agitation(halfWidth: number): number {
  return Math.min(3.5, 6 / Math.max(0.5, halfWidth));
}
