/**
 * The laboratory's model, from the law's metadata only (spec
 * laboratory-3d, R-02 to R-04, R-08): controls, default assumptions, the
 * requests sent to the engine and the reading of its answers. No law is
 * named here, and no verdict is computed here: it is the engine's.
 */
import { ApiFailure, EvaluateRequest, EvaluateResponse, LimitInfo, ParameterSpec, ParameterValue } from '../api.models';
import { LAB, STRINGS } from '../strings';
import { Inspect } from './inspect';
import { convert, displayUnits } from './lab-units';

/** SI units of the families the laboratory knows; a declared unit among them is preferred as the base. */
const SI = ['K', 'dimensionless', 'm', 's', 'kg', 'm/s', 'Hz', 'J', 'W', 'C', 'T', 'V/m', 'm^2', 'kg/m^3', 'bit/s', 'm^3/s^2', 'kg*m/s', 'rad', 'N', 'V', 'W/m^2', 'lm/W'];

/** A range is drawn and moved on a log scale when it spans three decades or more. */
export function isLog(lo: number, hi: number): boolean {
  return lo > 0 && hi / lo >= 1e3;
}

/** The middle of a range: geometric on a log scale. */
export function middle(lo: number, hi: number): number {
  return isLog(lo, hi) ? Math.sqrt(lo * hi) : (lo + hi) / 2;
}

/** The unit values are kept and sent in: the first declared SI unit, else the first declared unit. */
export function baseUnit(units: string[]): string {
  return units.find((u) => SI.includes(u)) ?? units[0] ?? 'dimensionless';
}

/** One slider of a parameter or of the claim. */
export interface Slider {
  id: string;
  label: string;
  /** Unit of `value`, `min`, `max`: the one sent to the engine. */
  base: string;
  /** Units it can be shown and typed in (base first). */
  units: string[];
  min: number;
  max: number;
  log: boolean;
  value: number;
}

/** One chooser of an assumption with more than one option; `value` '' means "not stated". */
export interface Chooser {
  id: string;
  label: string;
  options: { id: string; label: string }[];
  value: string;
}

/** Range used when the engine gives none for a parameter (no test range declared). */
export const FALLBACK_RANGE: [number, number] = [1, 1000];

export function parameterSlider(limit: LimitInfo, spec: ParameterSpec, range: [number, number] = FALLBACK_RANGE): Slider {
  const base = baseUnit(spec.units);
  const [lo, hi] = range;
  return {
    id: spec.id,
    label: limit.parameter_labels[spec.id] ?? spec.id,
    base,
    units: displayUnits(base),
    min: lo,
    max: hi,
    log: isLog(lo, hi),
    value: middle(lo, hi),
  };
}

/**
 * The assumptions to choose, each set to its first declared option. In the
 * library the first option is the one under which the law's bound applies
 * (a heat engine cycle, an unshrouded rotor, spherical accretion...), so no
 * law starts without a verdict. "Not stated" stays available.
 */
export function choosers(limit: LimitInfo): Chooser[] {
  return limit.assumptions
    .filter((a) => a.options.length > 1)
    .map((a) => ({
      id: a.id,
      label: limit.assumption_labels[a.id] ?? a.id,
      options: a.options.map((o) => ({ id: o, label: limit.option_labels[o] ?? o })),
      value: a.options[0],
    }));
}

export function constraintsOf(cs: Chooser[]): Record<string, string[]> {
  return Object.fromEntries(cs.filter((c) => c.value !== '').map((c) => [c.id, [c.value]]));
}

export function parametersOf(sliders: Slider[], except: string[] = []): Record<string, ParameterValue> {
  return Object.fromEntries(sliders.filter((s) => !except.includes(s.id)).map((s) => [s.id, { value: s.value, unit: s.base }]));
}

export function evaluateRequest(limit: LimitInfo, claim: Slider, params: Slider[], cs: Chooser[]): EvaluateRequest {
  return {
    limit_id: limit.id,
    claim: { value: claim.value, unit: claim.base },
    constraints: constraintsOf(cs),
    parameters: parametersOf(params),
  };
}

/** The claim's slider around the engine's bound: from a hundredth to three times it, on a log scale. */
export function claimSlider(limit: LimitInfo, bound: number | null, value?: number): Slider {
  const base = limit.claim_unit || 'dimensionless';
  const b = bound !== null && Number.isFinite(bound) && bound > 0 ? bound : 1;
  const min = b / 100;
  const max = b * 3;
  return {
    id: 'claim',
    label: LAB.claimLabel,
    base,
    units: displayUnits(base),
    min,
    max,
    log: true,
    value: value ?? initialClaim(limit.direction, b),
  };
}

/** A first claim near the bound, on its allowed side, so the scene starts from a readable state. */
export function initialClaim(direction: LimitInfo['direction'], bound: number): number {
  if (direction === 'Min') return bound * 1.25;
  if (direction === 'Equal') return bound;
  return bound * 0.8;
}

/** Whether a bound stands for "no limit": 0 for a minimum, a stand-in far above the other end for a maximum. */
function noLimit(v: number, other: number, direction: string): boolean {
  return direction === 'Min' ? v === 0 : other > 0 && v / other >= 1e20;
}

/**
 * The bound that decides, out of the engine's range, as on the review
 * screen: for IMPOSSIBLE the loosest end the claim still breaks, otherwise
 * the strictest end it meets; a "no limit" end is never chosen.
 */
export function decisiveBound(r: EvaluateResponse, direction: LimitInfo['direction']): number | null {
  const b = r.bounds?.permissive;
  if (!b) return null;
  if (b.b_min === b.b_max) return b.b_min;
  if (direction === 'Equal') return (b.b_min + b.b_max) / 2;
  if (direction === 'Min') {
    if (noLimit(b.b_min, b.b_max, direction)) return b.b_max;
    return r.verdict === 'IMPOSSIBLE' ? b.b_min : b.b_max;
  }
  if (noLimit(b.b_max, b.b_min, direction)) return b.b_min;
  return r.verdict === 'IMPOSSIBLE' ? b.b_max : b.b_min;
}

/** What the status panel shows: always a text label with the verdict. */
export interface LabResult {
  kind: 'impossible' | 'conditional' | 'compatible' | 'insufficient' | 'outside' | 'needs';
  label: string;
  /** The engine's wording, as received; a plain sentence when there is no verdict. */
  wording: string;
  bound: number | null;
  ratio: number | null;
}

export function resultOf(r: EvaluateResponse, claim: number, direction: LimitInfo['direction']): LabResult {
  const kind = ({ IMPOSSIBLE: 'impossible', CONDITIONAL: 'conditional', COMPATIBLE: 'compatible', INSUFFICIENT_DATA: 'insufficient' } as const)[r.verdict];
  const bound = decisiveBound(r, direction);
  return {
    kind,
    label: STRINGS.verdictLabel[r.verdict],
    wording: r.wording,
    bound,
    ratio: bound !== null && bound !== 0 ? claim / bound : null,
  };
}

/** The answer when the engine gives no verdict: outside the law's domain, or an assumption to state. */
export function failureOf(f: ApiFailure): LabResult {
  if (f.code === 'OutsideModelDomain') return { kind: 'outside', label: LAB.outside, wording: LAB.outsideSentence, bound: null, ratio: null };
  if (f.code === 'NotApplicable') return { kind: 'needs', label: LAB.needs, wording: LAB.notApplicable, bound: null, ratio: null };
  return { kind: 'needs', label: LAB.needs, wording: LAB.noAnswer, bound: null, ratio: null };
}

/** A value of an analysed claim in the slider's base unit; null when its unit cannot be converted. */
export function inBase(v: ParameterValue, base: string): number | null {
  return convert(v.value, v.unit, base);
}

/** The sliders and choosers with the analysed claim's values where they fit; ranges widened to hold them. */
export function applyInspect(inspect: Inspect, params: Slider[], cs: Chooser[]): { params: Slider[]; choosers: Chooser[] } {
  const next = params.map((s) => {
    const given = inspect.parameters[s.id];
    const v = given ? inBase(given, s.base) : null;
    if (v === null || !Number.isFinite(v)) return s;
    const min = Math.min(s.min, v);
    const max = Math.max(s.max, v);
    return { ...s, value: v, min, max, log: isLog(min, max) };
  });
  const chosen = cs.map((c) => {
    const o = inspect.constraints[c.id];
    return o && c.options.some((x) => x.id === o) ? { ...c, value: o } : c;
  });
  return { params: next, choosers: chosen };
}

/** Steps of a slider's track. */
export const STEPS = 1000;

/** The track position (0 to STEPS) of a slider's value, log or linear. */
export function toPosition(s: Slider): number {
  const f = (v: number) => (s.log ? Math.log10(v) : v);
  const span = f(s.max) - f(s.min);
  if (!(span > 0)) return 0;
  return Math.round(Math.min(1, Math.max(0, (f(s.value) - f(s.min)) / span)) * STEPS);
}

/** The value at a track position. */
export function fromPosition(s: Slider, position: number): number {
  const t = Math.min(1, Math.max(0, position / STEPS));
  if (s.log) return 10 ** (Math.log10(s.min) + t * (Math.log10(s.max) - Math.log10(s.min)));
  return s.min + t * (s.max - s.min);
}

/** The slider set to `value`, its range widened to hold it. */
export function withValue(s: Slider, value: number): Slider {
  if (!Number.isFinite(value)) return s;
  const min = Math.min(s.min, value);
  const max = Math.max(s.max, value);
  return { ...s, value, min, max, log: isLog(min, max) || (s.log && min > 0) };
}

/** The slider with the engine's range; its value kept when inside, else set to the middle. */
export function withRange(s: Slider, range: [number, number]): Slider {
  const [lo, hi] = range;
  if (!(Number.isFinite(lo) && Number.isFinite(hi) && hi > lo)) return s;
  const value = s.value >= lo && s.value <= hi ? s.value : middle(lo, hi);
  return { ...s, min: lo, max: hi, log: isLog(lo, hi), value };
}

/** A number for the panel: three significant digits, scientific notation far from 1. */
export function fmt(v: number): string {
  if (!Number.isFinite(v)) return '—';
  if (v !== 0 && (Math.abs(v) >= 1e5 || Math.abs(v) < 1e-3)) return v.toExponential(2);
  return String(Number(v.toPrecision(4)));
}

/** A unit for the panel: nothing for a plain ratio. */
export function unitText(unit: string): string {
  return unit === 'dimensionless' ? '' : unit;
}
