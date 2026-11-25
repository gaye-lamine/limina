/**
 * Pure helpers of the analysis screen: paragraphs, offsets, the plain
 * explanation and the explorer geometry. Every number comes from the API;
 * nothing here decides a verdict.
 */
import { AnalyzedClaim, CurvePoint } from './analysis.models';
import { formatValue } from './chart-geometry';
import { EXPLAIN, WB } from './strings';

/** A paragraph: its start in characters (code points) and its text. */
export interface Paragraph {
  start: number;
  text: string;
}

/** Number of characters (code points), as the API counts offsets. */
export function charCount(s: string): number {
  return Array.from(s).length;
}

/** Blocks of non-blank lines; paragraphs without a digit are dropped. */
export function paragraphs(text: string): Paragraph[] {
  const out: Paragraph[] = [];
  const re = /\S[\s\S]*?(?=\n[ \t\r]*\n|$)/g;
  for (const m of text.matchAll(re)) {
    const body = m[0].trimEnd();
    if (/\d/.test(body)) out.push({ start: charCount(text.slice(0, m.index ?? 0)), text: body });
  }
  return out;
}

/** The claim with every evidence offset moved by `by` characters. */
export function shiftClaim(claim: AnalyzedClaim, by: number): AnalyzedClaim {
  const move = <T extends { start: number; end: number } | null>(e: T): T =>
    e ? ({ ...e, start: e.start + by, end: e.end + by } as T) : e;
  const parameters = Object.fromEntries(
    Object.entries(claim.parameters).map(([k, p]) => [k, { ...p, evidence: move(p.evidence) }]),
  );
  return { ...claim, evidence: move(claim.evidence), parameters };
}

/** The label of the claim tab and of its verdict. */
export function verdictLabel(claim: AnalyzedClaim): string {
  if (claim.status === 'outside_model_domain') return WB.outsideModelDomain;
  return claim.result?.verdict.replace('_', ' ') ?? WB.needsInput;
}

/** The badge of the law. */
export function badge(claim: AnalyzedClaim): string {
  return WB.badge[claim.law?.source ?? 'no_law_identified'] ?? '';
}

/**
 * The explanation, from engine data only: the margin (claim over bound,
 * computed by the engine), the configurations, the missing parameters, or
 * the reason the claim was not evaluated.
 */
export function explain(claim: AnalyzedClaim): string[] {
  const out: string[] = [];
  if (claim.law?.source === 'draft_disputed') out.push(EXPLAIN.disputed);
  if (claim.law?.source === 'draft_agreed') out.push(EXPLAIN.aiDraft);
  const r = claim.result;
  if (!r) {
    if (claim.reason) {
      out.push(
        claim.status === 'outside_model_domain'
          ? EXPLAIN.outsideModelDomain(claim.reason)
          : EXPLAIN.needsInput(claim.reason),
      );
    }
    return out;
  }
  const margin = r.bounds?.permissive.margin ?? null;
  const ratio = margin ? formatValue(margin.low) : null;
  const direction = claimDirection(claim);
  switch (r.verdict) {
    case 'IMPOSSIBLE':
      if (ratio) out.push(direction === 'Min' ? EXPLAIN.minUnder(ratio) : EXPLAIN.maxOver(ratio));
      break;
    case 'COMPATIBLE':
      if (ratio) out.push(direction === 'Min' ? EXPLAIN.minWithin(ratio) : EXPLAIN.maxWithin(ratio));
      break;
    case 'CONDITIONAL':
      out.push(EXPLAIN.conditional);
      break;
    case 'INSUFFICIENT_DATA':
      out.push(EXPLAIN.insufficient(r.missing_parameters.map((m) => m.id).join(', ')));
      break;
  }
  return out;
}

/** Direction of the limit, from the claim's limit metadata cache. */
const DIRECTIONS = new Map<string, 'Max' | 'Min'>();

/** Records the direction of a limit, read from GET /v1/limits or a curve. */
export function rememberDirection(limitId: string, direction: 'Max' | 'Min'): void {
  DIRECTIONS.set(limitId, direction);
}

function claimDirection(claim: AnalyzedClaim): 'Max' | 'Min' {
  return DIRECTIONS.get(claim.limit_id ?? '') ?? 'Max';
}

/** The solver sentence as shown: "It would be consistent only if ...". */
export function solveSentence(apiSentence: string): string {
  const at = apiSentence.indexOf('only if ');
  return at < 0 ? apiSentence : `${EXPLAIN.solvePrefix} ${apiSentence.slice(at + 'only if '.length)}`;
}

/** Geometry of the explorer plot, in viewBox units. */
export interface PlotGeometry {
  bound: string;
  hatch: string;
  claimY: number | null;
  xTicks: { x: number; label: string }[];
  yTicks: { y: number; label: string }[];
}

export const PLOT = { width: 640, height: 300, left: 72, right: 16, top: 16, bottom: 40 } as const;

/**
 * Lines of the plot: the decisive bound, the region inconsistent with the
 * law (above the bound for a Max limit, below for a Min limit), and the
 * claim as a horizontal line. Undefined points break the line.
 */
export function plotGeometry(
  points: CurvePoint[],
  direction: 'Max' | 'Min',
  claim: number | null,
): PlotGeometry | null {
  const defined = points.filter((p) => p.decisive !== null) as (CurvePoint & { decisive: number })[];
  if (defined.length < 2) return null;
  const xs = points.map((p) => p.x);
  const x0 = Math.min(...xs);
  const x1 = Math.max(...xs);
  const ys = defined.map((p) => p.decisive);
  if (claim !== null && Number.isFinite(claim)) ys.push(claim);
  let y0 = Math.min(0, ...ys);
  let y1 = Math.max(...ys) * 1.15;
  if (y1 <= y0) y1 = y0 + 1;
  if (y0 < 0) y0 *= 1.15;
  const w = PLOT.width - PLOT.left - PLOT.right;
  const h = PLOT.height - PLOT.top - PLOT.bottom;
  const sx = (x: number) => PLOT.left + ((x - x0) / (x1 - x0 || 1)) * w;
  const sy = (y: number) => PLOT.top + h - ((y - y0) / (y1 - y0)) * h;
  let bound = '';
  let pen = false;
  for (const p of points) {
    if (p.decisive === null) {
      pen = false;
      continue;
    }
    bound += `${pen ? 'L' : 'M'}${sx(p.x).toFixed(1)},${sy(p.decisive).toFixed(1)} `;
    pen = true;
  }
  const edge = direction === 'Max' ? PLOT.top : PLOT.top + h;
  const first = defined[0];
  const last = defined[defined.length - 1];
  const hatch =
    `M${sx(first.x).toFixed(1)},${edge} ` +
    defined.map((p) => `L${sx(p.x).toFixed(1)},${sy(p.decisive).toFixed(1)}`).join(' ') +
    ` L${sx(last.x).toFixed(1)},${edge} Z`;
  const ticks = (a: number, b: number) => [0, 0.25, 0.5, 0.75, 1].map((t) => a + t * (b - a));
  return {
    bound: bound.trim(),
    hatch,
    claimY: claim !== null && Number.isFinite(claim) ? sy(claim) : null,
    xTicks: ticks(x0, x1).map((x) => ({ x: sx(x), label: formatValue(x) })),
    yTicks: ticks(y0, y1).map((y) => ({ y: sy(y), label: formatValue(y) })),
  };
}
