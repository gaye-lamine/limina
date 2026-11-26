/**
 * Display of engine results for people: formulas with real symbols, the
 * formal statement naming the law, values in the unit the text uses, the
 * values used and the calculation, links to references. Formatting only:
 * every number shown comes from the API.
 */
import { AnalyzedClaim } from './analysis.models';
import { EvaluateResponse, LimitInfo } from './api.models';
import { formatValue } from './chart-geometry';
import { UI } from './strings';

/** A formula with real symbols: ≤, ≥, ·, −. */
export function prettyFormula(f: string): string {
  return f
    .replace(/<=/g, '≤')
    .replace(/>=/g, '≥')
    .replace(/\s*\*\s*/g, '·')
    .replace(/ - /g, ' − ')
    .replace(/\(\s*([^()]*?)-([^()]*?)\s*\)/g, '($1−$2)');
}

/** The engine's formal statement, with the law named by its title instead of its id. */
export function formalStatement(wording: string, limit: LimitInfo | null | undefined): string {
  const named = limit ? wording.replace(new RegExp(`\\blimit ${limit.id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`), `the ${limit.title}`) : wording;
  return readableAssumptions(named);
}

/**
 * The assumptions part of a statement in words: "under assumptions none"
 * is dropped, "a = x" reads "a: x" and "a in {x, y}" reads "a: x or y".
 */
export function readableAssumptions(s: string): string {
  return s
    .replace(/ under assumptions none\b/, '')
    .replace(/ in \{([^}]*)\}/g, (_, list: string) => `: ${list.split(/,\s*/).join(` ${UI.or} `)}`)
    .replace(/ = /g, ': ');
}

const PREFIX: Record<string, number> = { p: 1e-12, n: 1e-9, u: 1e-6, µ: 1e-6, m: 1e-3, c: 1e-2, k: 1e3, M: 1e6, G: 1e9, T: 1e12 };

/** The factor from the unit the text uses to the law's unit, when it is a known prefix or %; null otherwise. */
export function toCanonical(rawUnit: string, unit: string): number | null {
  const raw = rawUnit.trim();
  if (raw === unit) return 1;
  if (raw === '%' && (unit === 'dimensionless' || unit === '')) return 0.01;
  if (raw.length > 1 && raw.slice(1) === unit && PREFIX[raw[0]] !== undefined) return PREFIX[raw[0]];
  return null;
}

/** An engine value in the unit the claim was written in ("50.0 %", "250 nm"); in the law's unit otherwise. */
export function inClaimUnit(v: number, c: AnalyzedClaim): string {
  const k = toCanonical(c.raw_unit, c.unit);
  if (k !== null && k !== 1) return `${formatValue(v / k)} ${c.raw_unit}`;
  return c.unit && c.unit !== 'dimensionless' ? `${formatValue(v)} ${c.unit}` : formatValue(v);
}

/** The parameters the engine used: "hot reservoir temperature Th = 600 K, cold reservoir temperature Tc = 300 K". */
export function valuesUsed(c: AnalyzedClaim, limit?: LimitInfo | null): string {
  return Object.entries(c.parameters)
    .filter(([, p]) => p.value !== null)
    .map(([id, p]) => {
      const label = limit?.parameter_labels[id];
      const name = !label || label === id ? id : label.split(/\s+/).includes(id) ? label : `${label} ${id}`;
      return `${name} = ${withUnitText(plainNumber(p.raw_value ?? (p.value as number)), p.unit)}`;
    })
    .join(', ');
}

function withUnitText(v: string, unit: string | undefined): string {
  return unit && unit !== 'dimensionless' ? `${v} ${unit}` : v;
}

/** A number as written in a calculation: "300", "1.2", "1e-24"; ten significant digits at most, so reading noise ("1.0000000000000001e-24") never shows. */
export function plainNumber(v: number): string {
  const n = Number(v.toPrecision(10));
  return Math.abs(n) >= 1e-3 && Math.abs(n) < 1e6 ? String(n) : n.toExponential().replace('e+', 'e');
}

/** Whether a bound stands for "no limit": 0 for a minimum, or a huge stand-in for a maximum. */
function noLimit(v: number, other: number, direction: string): boolean {
  return direction === 'Min' ? v === 0 : other > 0 && v / other >= 1e20;
}

/** The right-hand side of each piece of a formula: "1 - Tc/Th" from "efficiency <= 1 - Tc/Th". */
function rightSides(formula: string): string[] {
  return formula
    .split(/,\s*(?=[A-Za-zΔ]+\s*(?:<=|>=|=))/)
    .map((piece) => {
      const m = piece.match(/(?:<=|>=|=)\s*(.+)$/);
      return (m ? m[1] : piece).trim();
    });
}

/**
 * The calculation with the claim's values put in the formula, and the
 * engine's result when it is a single value: "1 − 300/600 = 0.500".
 * Null when a parameter of the formula has no value.
 */
export function calculation(c: AnalyzedClaim, limit: LimitInfo | null | undefined): string | null {
  if (!limit || !c.result?.bounds) return null;
  const values = new Map<string, string>();
  for (const [id, p] of Object.entries(c.parameters)) {
    if (p.value === null) continue;
    values.set(id, withUnitText(plainNumber(p.raw_value ?? p.value), p.unit));
  }
  const ids = limit.parameters.map((p) => p.id).sort((a, b) => b.length - a.length);
  const pieces = rightSides(limit.formula).map((rhs) => {
    let out = rhs;
    for (const id of ids) {
      const v = values.get(id);
      if (v === undefined) {
        if (new RegExp(`\\b${id}\\b`).test(out)) return null;
        continue;
      }
      out = out.replace(new RegExp(`\\b${id}\\b`, 'g'), v);
    }
    return prettyFormula(out);
  });
  if (pieces.some((p) => p === null)) return null;
  const b = c.result.bounds.permissive;
  if (pieces.length !== 1) return (pieces as string[]).join('; ');
  if (b.b_min === b.b_max) return `${pieces[0]} = ${inClaimUnit(b.b_min, c)}`;
  // A range whose other end means "no limit": the shared formula gives the real end.
  const direction = limit.direction;
  if (noLimit(b.b_min, b.b_max, direction)) return `${pieces[0]} = ${inClaimUnit(b.b_max, c)}`;
  if (direction === 'Max' && noLimit(b.b_max, b.b_min, direction)) return `${pieces[0]} = ${inClaimUnit(b.b_min, c)}`;
  return pieces[0] as string;
}

/** The engine's result for one option of an assumption the text does not state. */
export interface OptionResult {
  assumption: string;
  option: string;
  result: EvaluateResponse;
}

/** The option to show first: one the claim fails under, else the first. */
export function focusOption(options: OptionResult[] | undefined): OptionResult | null {
  if (!options?.length) return null;
  return options.find((o) => o.result.verdict === 'IMPOSSIBLE') ?? options[0];
}

/** The bound that decides the verdict, and the claim over it. */
export interface Decisive {
  bound: number;
  ratio: number | null;
}

/**
 * One bound out of the engine's range (the range spans the law's
 * assumptions): for IMPOSSIBLE the loosest one, which the claim still
 * breaks; otherwise the strictest one, which the claim still meets. A
 * bound that stands for "no limit" (0 for a minimum) is never chosen when
 * another exists. The ratio is the engine's margin at that end; without one
 * it is the claim value over that bound.
 */
export function decisive(c: AnalyzedClaim, r: EvaluateResponse | null | undefined, direction: string): Decisive | null {
  const b = r?.bounds?.permissive;
  if (!b) return null;
  const m = b.margin ?? null;
  const low = { bound: b.b_max, ratio: m?.low ?? null };
  const high = { bound: b.b_min, ratio: m?.high ?? null };
  let pick: { bound: number; ratio: number | null };
  if (b.b_min === b.b_max) pick = low;
  else if (direction === 'Min') pick = r?.verdict === 'IMPOSSIBLE' && b.b_min > 0 ? high : low;
  else pick = r?.verdict === 'IMPOSSIBLE' ? low : high;
  const ratio =
    pick.ratio !== null && Number.isFinite(pick.ratio)
      ? pick.ratio
      : c.value !== null && pick.bound !== 0
        ? c.value / pick.bound
        : null;
  return { bound: pick.bound, ratio };
}

/** The bound of a result in the claim's unit; "no limit" for a lower bound of zero. */
export function boundDisplay(c: AnalyzedClaim, r: EvaluateResponse | null | undefined, direction: string): string {
  const d = decisive(c, r, direction);
  if (!d) return UI.metricNone;
  if (direction === 'Min' && d.bound === 0) return UI.optionNoBound;
  return inClaimUnit(d.bound, c);
}

/** The claim over the decisive bound: "2.50×"; null without one. */
export function ratioDisplay(c: AnalyzedClaim, r: EvaluateResponse | null | undefined, direction: string): string | null {
  const d = decisive(c, r, direction);
  return d?.ratio != null ? `${formatValue(d.ratio)}×` : null;
}

/** The web address of a reference's DOI, or null. */
export function doiUrl(ref: string): string | null {
  const m = ref.match(/doi:\s*(10\.\d{4,9}\/[^\s,;]+)/i);
  return m ? `https://doi.org/${m[1].replace(/[.)]+$/, '')}` : null;
}

/** An assumption of the law that the text does not state, with its readable options. */
export interface MissingAssumption {
  id: string;
  label: string;
  options: { id: string; label: string }[];
}

/** The law's assumptions with more than one option that the claim does not state. */
export function missingAssumptions(c: AnalyzedClaim, limit: LimitInfo | null | undefined): MissingAssumption[] {
  if (!limit) return [];
  return limit.assumptions
    .filter((a) => a.options.length > 1 && !c.constraints[a.id])
    .map((a) => ({
      id: a.id,
      label: limit.assumption_labels[a.id] ?? a.id.replace(/_/g, ' '),
      options: a.options.map((o) => ({ id: o, label: limit.option_labels[o] ?? o.replace(/_/g, ' ') })),
    }));
}

/** "State the reference area: unshrouded rotor, shrouded, total capture area or shrouded, rotor area only." */
export function stateAssumptions(missing: MissingAssumption[]): string {
  return missing.map((a) => UI.settleAssumption(a.label, a.options.map((o) => o.label))).join(' ');
}
