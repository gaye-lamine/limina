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
  if (!limit) return wording;
  return wording.replace(new RegExp(`\\blimit ${limit.id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`), `the ${limit.title}`);
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

/** The parameters the engine used: "Th = 600 K, Tc = 300 K". */
export function valuesUsed(c: AnalyzedClaim): string {
  return Object.entries(c.parameters)
    .filter(([, p]) => p.value !== null)
    .map(([id, p]) => `${id} = ${formatValue(p.raw_value ?? (p.value as number))}${p.unit && p.unit !== 'dimensionless' ? ` ${p.unit}` : ''}`)
    .join(', ');
}

/** A number as written in a calculation: "300", "1.2", "1e-24". */
function plainNumber(v: number): string {
  return Math.abs(v) >= 1e-3 && Math.abs(v) < 1e6 ? String(Number(v.toPrecision(6))) : v.toExponential().replace('e+', 'e');
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
    // Numbers only: their units are listed on the "values used" line.
    values.set(id, plainNumber(p.raw_value ?? p.value));
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
  const single = b.b_min === b.b_max && pieces.length === 1;
  return single ? `${pieces[0]} = ${inClaimUnit(b.b_min, c)}` : (pieces as string[]).join('; ');
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

/** The bound of a result in the claim's unit; "no limit" for a lower bound of zero. */
export function boundDisplay(c: AnalyzedClaim, r: EvaluateResponse | null | undefined, direction: string): string {
  const b = r?.bounds?.permissive;
  if (!b) return UI.metricNone;
  if (direction === 'Min' && b.b_max === 0) return UI.optionNoBound;
  return b.b_min === b.b_max ? inClaimUnit(b.b_min, c) : `${inClaimUnit(b.b_min, c)} – ${inClaimUnit(b.b_max, c)}`;
}

/** The claim over the bound, as computed by the engine; null without a margin. */
export function ratioDisplay(r: EvaluateResponse | null | undefined): string | null {
  const m = r?.bounds?.permissive.margin ?? null;
  if (!m) return null;
  return m.low === m.high ? `${formatValue(m.low)}×` : `${formatValue(m.low)}–${formatValue(m.high)}×`;
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
