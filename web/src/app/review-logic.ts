/**
 * Pure helpers of the review screen: verdict kinds and labels, the plain
 * sentence, metric texts, highlighted segments of the text, the summary and
 * the Markdown report. Every verdict and number comes from the API; nothing
 * here decides a verdict.
 */
import { AnalyzedClaim, LawAttempt } from './analysis.models';
import { decisive, MissingAssumption, missingAssumptions, stateAssumptions } from './review-format';
import { ApiFailure, LimitInfo } from './api.models';
import { formatValue } from './chart-geometry';
import { UI, WB } from './strings';
import { verdictLabel } from './workbench-logic';

/** One reviewed item: a claim, or a paragraph that could not be analysed. Paragraphs are 1-based. */
export type Entry =
  | { kind: 'claim'; claim: AnalyzedClaim; paragraph: number }
  | { kind: 'failure'; paragraph: number; text: string; failure: ApiFailure; attempts: number };

/** The visual family of a verdict; it is always shown with its text label. */
export type VerdictKind = 'impossible' | 'conditional' | 'compatible' | 'insufficient' | 'needs' | 'disputed';

export const VERDICT_KINDS: VerdictKind[] = ['impossible', 'conditional', 'compatible', 'insufficient', 'needs', 'disputed'];

export function verdictKind(e: Entry): VerdictKind {
  if (e.kind === 'failure') return 'needs';
  if (e.claim.law?.source === 'draft_disputed') return 'disputed';
  switch (e.claim.result?.verdict) {
    case 'IMPOSSIBLE':
      return 'impossible';
    case 'CONDITIONAL':
      return 'conditional';
    case 'COMPATIBLE':
      return 'compatible';
    case 'INSUFFICIENT_DATA':
      return 'insufficient';
    default:
      return 'needs';
  }
}

/** The label of the verdict pill: the engine verdict, or why there is none. */
export function verdictText(e: Entry): string {
  if (e.kind === 'failure') return UI.notAnalysed;
  if (e.claim.law?.source === 'draft_disputed') return UI.disputed;
  return verdictLabel(e.claim);
}

/** Label of a verdict family in the summary strip. */
export function kindLabel(k: VerdictKind): string {
  return UI.kind[k];
}

/** The claim over the bound, as computed by the engine; null without a margin. */
export function ratioText(c: AnalyzedClaim): string | null {
  const m = c.result?.bounds?.permissive.margin ?? null;
  if (!m) return null;
  return m.low === m.high ? `${formatValue(m.low)}×` : `${formatValue(m.low)}–${formatValue(m.high)}×`;
}

/** A ratio in words: "2.5", "1.49"; at most three significant digits. */
function ratioWords(r: number): string {
  return String(Number(r.toPrecision(r >= 10 ? 2 : 3)));
}

/** How far a claim is from a bound, in one plain sentence. */
export function ratioSentence(ratio: number, direction: 'Max' | 'Min' | 'Equal', impossible: boolean): string {
  if (direction === 'Min') {
    if (ratio >= 1) return UI.aboveMinimum(ratioWords(ratio));
    if (ratio < 0.5) return UI.belowMinimumTimes(ratioWords(1 / ratio));
    return UI.belowMinimumPercent(String(Math.round((1 - ratio) * 100)));
  }
  if (impossible || ratio > 1) return UI.aboveMaximum(ratioWords(ratio));
  return UI.withinMaximum(String(Number((ratio * 100).toPrecision(3))));
}

/** "super-resolution techniques or near-field methods": the options under which a CONDITIONAL claim holds. */
export function conditionalOptions(c: AnalyzedClaim, limit: LimitInfo | null | undefined): string | null {
  const configs = c.result?.configurations ?? [];
  const labels = [
    ...new Set(
      configs.flatMap((k) =>
        Object.entries(k.assumptions)
          .filter(([a]) => !c.constraints[a])
          .map(([, o]) => limit?.option_labels[o] ?? o.replace(/_/g, ' ')),
      ),
    ),
  ];
  return labels.length ? labels.join(` ${UI.or} `) : null;
}

/** Names of the missing values, as people say them. */
function missingNames(c: AnalyzedClaim, limit: LimitInfo | null | undefined): string {
  return (c.result?.missing_parameters ?? []).map((p) => limit?.parameter_labels[p.id] ?? p.id).join(', ');
}

/** One plain sentence from engine data only. */
export function plainSentence(e: Entry, direction: 'Max' | 'Min' | 'Equal', limit?: LimitInfo | null): string {
  if (e.kind === 'failure') return UI.failedSentence();
  const c = e.claim;
  if (c.law?.source === 'draft_disputed') return UI.disputedSentence;
  const r = c.result;
  if (!r) return c.reason ? UI.noVerdictBecause(c.reason) : UI.noVerdict;
  if (r.verdict === 'IMPOSSIBLE' || r.verdict === 'COMPATIBLE') {
    const d = decisive(c, r, direction);
    if (d?.ratio != null) return ratioSentence(d.ratio, direction, r.verdict === 'IMPOSSIBLE');
    return r.verdict === 'IMPOSSIBLE' ? UI.impossibleSentence : UI.compatibleSentence;
  }
  if (r.verdict === 'INSUFFICIENT_DATA') return UI.missing(missingNames(c, limit));
  if (r.verdict === 'CONDITIONAL') {
    const options = conditionalOptions(c, limit);
    return options ? UI.conditionalWith(options) : UI.conditional(r.wording);
  }
  return r.wording;
}

/** The bound in the claim's canonical unit; '-' when the engine gave none. */
export function boundText(c: AnalyzedClaim): string {
  const b = c.result?.bounds?.permissive;
  if (!b) return '-';
  const v = b.b_min === b.b_max ? formatValue(b.b_min) : `${formatValue(b.b_min)} – ${formatValue(b.b_max)}`;
  return withUnit(v, c.unit);
}

/** The claim value in the canonical unit when converted, else as read. */
export function claimValueText(c: AnalyzedClaim): string {
  return c.value !== null ? withUnit(formatValue(c.value), c.unit) : withUnit(formatValue(c.raw_value), c.raw_unit);
}

/** The claim as read in the text: "3.60e+8 m/s". */
export function readValueText(c: AnalyzedClaim): string {
  return withUnit(formatValue(c.raw_value), c.raw_unit);
}

function withUnit(v: string, unit: string): string {
  return unit && unit !== 'dimensionless' ? `${v} ${unit}` : v;
}

/** A piece of the text: plain, or the evidence of claim `claim` (index in the entries). */
export interface Segment {
  text: string;
  claim: number | null;
}

/** UTF-16 index of code point `cp` of `s`. */
function cpIndex(s: string, cp: number): number {
  return Array.from(s).slice(0, Math.max(0, cp)).join('').length;
}

/**
 * The text cut at the evidence of each claim, from the API's character
 * offsets (code points). Overlapping evidence: the first claim wins.
 */
export function segments(text: string, claims: { index: number; claim: AnalyzedClaim }[]): Segment[] {
  const n = Array.from(text).length;
  const marks: { from: number; to: number; claim: number }[] = [];
  for (const { index, claim } of claims) {
    const e = claim.evidence;
    if (!e) continue;
    const a = Math.max(0, Math.min(n, e.start));
    const b = Math.max(0, Math.min(n, e.end));
    if (b <= a) continue;
    const from = cpIndex(text, a);
    const to = cpIndex(text, b);
    if (!marks.some((k) => from < k.to && to > k.from)) marks.push({ from, to, claim: index });
  }
  marks.sort((x, y) => x.from - y.from);
  const out: Segment[] = [];
  let at = 0;
  for (const k of marks) {
    if (k.from > at) out.push({ text: text.slice(at, k.from), claim: null });
    out.push({ text: text.slice(k.from, k.to), claim: k.claim });
    at = k.to;
  }
  if (at < text.length) out.push({ text: text.slice(at), claim: null });
  return out;
}

/** A short readable label of a claim: the law's short quantity name (or the first words of the quantity) and the value. */
export function shortLabel(e: Entry): string {
  if (e.kind === 'failure') return `${WB.progressParagraph} ${e.paragraph}`;
  const c = e.claim;
  const short = c.limit_id ? UI.short[c.limit_id] : undefined;
  const name = short ?? c.quantity.split(/[\s-]+/).filter(Boolean).slice(0, 2).join(' ');
  return `${name} ${readValueText(c)}`;
}

/** Sentence ranges of a text, in code points: up to and including ". ", "! ", "? ", a line break, or the end. */
export function sentenceRanges(text: string): { start: number; end: number }[] {
  const cp = Array.from(text);
  const out: { start: number; end: number }[] = [];
  let start = 0;
  for (let i = 0; i < cp.length; i++) {
    const endHere = (/[.!?]/.test(cp[i]) && (i + 1 === cp.length || /\s/.test(cp[i + 1]))) || cp[i] === '\n';
    if (endHere) {
      out.push({ start, end: i + 1 });
      start = i + 1;
      while (start < cp.length && /\s/.test(cp[start])) start++;
      i = start - 1;
    }
  }
  if (start < cp.length) out.push({ start, end: cp.length });
  return out.filter((r) => r.end > r.start);
}

const SUPER: Record<string, string> = { '⁰': '0', '¹': '1', '²': '2', '³': '3', '⁴': '4', '⁵': '5', '⁶': '6', '⁷': '7', '⁸': '8', '⁹': '9', '⁻': '-', '⁺': '+' };
const NUM = /[-+−]?\d+(?:[.,]\d+)?(?:\s*(?:[eE][-+−]?\d+|[×x*·]\s*10(?:\^?[-+−]?\d+|[⁻⁺]?[⁰¹²³⁴⁵⁶⁷⁸⁹]+)))?/g;

/** The value of a number as written: "3.6e8", "1 × 10⁻²⁴", "1,4". */
export function readNumber(s: string): number {
  const t = s.replace(/−/g, '-').replace(/\s/g, '').replace(',', '.');
  const m = t.match(/^([-+]?\d+(?:\.\d+)?)(?:[eE]([-+]?\d+)|[×x*·]10\^?([-+]?\d+|[⁻⁺]?[⁰¹²³⁴⁵⁶⁷⁸⁹]+))?$/);
  if (!m) return NaN;
  const exp = m[2] ?? (m[3] ? Array.from(m[3]).map((c) => SUPER[c] ?? c).join('') : '0');
  return Number(`${m[1]}e${exp}`);
}

/**
 * Where the claim's value is written, in code points of `text`: the number
 * inside the claim's evidence whose value is the value the model read.
 * Falls back to the whole evidence when no number matches.
 */
export function valueRange(text: string, c: AnalyzedClaim): { start: number; end: number } | null {
  const e = c.evidence;
  if (!e) return null;
  const cp = Array.from(text);
  const ev = cp.slice(e.start, e.end).join('');
  for (const m of ev.matchAll(NUM)) {
    const v = readNumber(m[0]);
    if (Number.isFinite(v) && Math.abs(v - c.raw_value) <= 1e-9 * Math.max(Math.abs(v), Math.abs(c.raw_value))) {
      const at = Array.from(ev.slice(0, m.index ?? 0)).length;
      return { start: e.start + at, end: e.start + at + Array.from(m[0]).length };
    }
  }
  return { start: e.start, end: e.end };
}

/** A piece of a sentence: plain, or the value of claim `claim` (underlined, with its badge). */
export interface Piece {
  text: string;
  claim: number | null;
}

/** A sentence of the reading panel: tinted for the first claim it holds, with the value pieces. */
export interface ReadingSentence {
  claim: number | null;
  claims: number[];
  pieces: Piece[];
}

/**
 * The paragraph cut into sentences. A sentence that holds a claim's
 * evidence is tinted with that claim; inside it, each claim's value is its
 * own piece. Offsets are code points of the paragraph (claims already
 * shifted to the paragraph).
 */
export function readingSentences(text: string, claims: { index: number; claim: AnalyzedClaim }[]): ReadingSentence[] {
  const cp = Array.from(text);
  return sentenceRanges(text).map((r) => {
    const mine = claims.filter(({ claim }) => claim.evidence && claim.evidence.start < r.end && claim.evidence.end > r.start);
    const values = mine
      .map(({ index, claim }) => ({ index, range: valueRange(text, claim) }))
      .filter((v): v is { index: number; range: { start: number; end: number } } => !!v.range)
      .map((v) => ({ index: v.index, start: Math.max(v.range.start, r.start), end: Math.min(v.range.end, r.end) }))
      .filter((v) => v.end > v.start)
      .sort((a, b) => a.start - b.start);
    const pieces: Piece[] = [];
    let at = r.start;
    for (const v of values) {
      if (v.start < at) continue;
      if (v.start > at) pieces.push({ text: cp.slice(at, v.start).join(''), claim: null });
      pieces.push({ text: cp.slice(v.start, v.end).join(''), claim: v.index });
      at = v.end;
    }
    if (at < r.end) pieces.push({ text: cp.slice(at, r.end).join(''), claim: null });
    return { claim: mine[0]?.index ?? null, claims: mine.map((m) => m.index), pieces };
  });
}

/** One candidate law of a disputed claim. */
export interface Candidate {
  model: string;
  formula: string;
  bound: number | null;
  note: string;
}

/** A missing quantity the user can supply. */
export interface MissingInput {
  id: string;
  label: string;
  unit: string;
}

/** The card shown instead of metrics for a claim without a verdict. */
export interface NoVerdict {
  kind: 'disputed' | 'needs' | 'nolaw' | 'failed';
  happened: string;
  tried: string[];
  settle: string;
  candidates: Candidate[];
  divergence: number | null;
  missing: MissingInput[];
  /** Assumptions of the law the text does not state, to choose from. */
  assumptions: MissingAssumption[];
  /** "What would settle it" for the printed review (no on-screen instruction). */
  settlePrint: string;
  sources: string[];
}

function formulaOf(a: LawAttempt): string {
  const d = a.definition;
  if (!d) return UI.noDefinition;
  if (d.formula) return d.formula;
  const per = (d.assumptions ?? []).flatMap((s) => (s.options ?? []).filter((o) => o.formula).map((o) => `${o.id ?? o.name}: ${o.formula}`));
  return per.length ? per.join('; ') : UI.noDefinition;
}

/**
 * A step of the trace in plain words, without internal ids, model names or
 * error codes: "Looked for the matching law of physics in our library: no result".
 */
export function plainStep(t: { step: string; status: string }): string {
  const authoring = /^authoring\b/.test(t.step);
  const name = authoring ? UI.stepNames['authoring'] : (UI.stepNames[t.step] ?? t.step.replace(/_/g, ' '));
  return `${name}: ${UI.stepStatus[t.status] ?? t.status}`;
}

/** One step of the check, for the timeline: a plain name and its status. */
export interface StepItem {
  name: string;
  status: 'done' | 'failed' | 'skipped';
  label: string;
}

/** The steps of an entry for the timeline, in plain words. */
export function stepItems(e: Entry): StepItem[] {
  const trace = e.kind === 'claim' ? e.claim.trace : [{ step: 'extraction', status: 'failed' as const, detail: '' }];
  let version = 0;
  return trace.map((t) => {
    const authoring = /^authoring\b/.test(t.step);
    const base = authoring ? UI.stepNames['authoring'] : (UI.stepNames[t.step] ?? t.step.replace(/_/g, ' '));
    const name = authoring ? `${base} (${UI.version(version++)})` : base;
    return { name, status: t.status, label: UI.stepStatus[t.status] ?? t.status };
  });
}

/** Steps of the trace, one plain line each. */
export function plainSteps(c: AnalyzedClaim): string[] {
  let version = 0;
  return c.trace.map((t) => {
    const line = plainStep(t);
    return /^authoring\b/.test(t.step) ? line.replace(UI.stepNames['authoring'], `${UI.stepNames['authoring']} (${UI.version(version++)})`) : line;
  });
}

function triedSteps(c: AnalyzedClaim): string[] {
  return plainSteps(c);
}

/** What the card says for a claim with no engine verdict; null when there is a verdict. */
export function noVerdict(e: Entry, limits: LimitInfo[]): NoVerdict | null {
  if (e.kind === 'failure') {
    return {
      kind: 'failed',
      happened: UI.failedSentence(),
      tried: [plainStep({ step: 'extraction', status: 'failed' })],
      settle: UI.settleFailed,
      settlePrint: UI.settleFailed,
      candidates: [],
      divergence: null,
      missing: [],
      assumptions: [],
      sources: [],
    };
  }
  const c = e.claim;
  const r = c.result;
  if (r && r.verdict !== 'INSUFFICIENT_DATA') return null;
  const base = {
    tried: triedSteps(c),
    candidates: [] as Candidate[],
    divergence: null as number | null,
    missing: [] as MissingInput[],
    assumptions: [] as MissingAssumption[],
    sources: [] as string[],
  };
  if (c.law?.source === 'draft_disputed') {
    const candidates = c.law.attempts.map((a, n) => ({
      model: UI.version(n),
      formula: formulaOf(a),
      bound: a.bound_at_claim ?? null,
      note: a.bound_note ?? (a.ok ? '' : UI.noDefinition),
    }));
    const [x, y] = candidates.map((k) => k.bound);
    const divergence =
      x !== null && x !== undefined && y !== null && y !== undefined && Math.max(Math.abs(x), Math.abs(y)) > 0
        ? Math.abs(x - y) / Math.max(Math.abs(x), Math.abs(y))
        : null;
    return { ...base, kind: 'disputed', happened: UI.disputedSentence, settle: UI.settleDisputed, settlePrint: UI.settleDisputed, candidates, divergence };
  }
  const limit = limits.find((l) => l.id === c.limit_id);
  if (limit) {
    const ids = r?.missing_parameters.map((m) => m.id) ?? limit.parameters.filter((p) => c.parameters[p.id]?.value == null).map((p) => p.id);
    const assumptions = r ? [] : missingAssumptions(c, limit);
    const missing = ids.map((id) => {
      const spec = limit.parameters.find((p) => p.id === id);
      return { id, label: limit.parameter_labels[id] ?? id, unit: spec?.units[0] ?? '' };
    });
    return {
      ...base,
      kind: 'needs',
      happened: c.reason ? UI.noVerdictBecause(c.reason) : UI.missing(ids.join(', ')),
      settle: missing.length ? UI.settleNeeds(missing.map((m) => m.label).join(', ')) : assumptions.length ? stateAssumptions(assumptions) : UI.settleUnit,
      settlePrint: missing.length ? UI.settleNeedsPrint(missing.map((m) => m.label).join(', ')) : assumptions.length ? stateAssumptions(assumptions) : UI.settleUnit,
      missing,
      assumptions,
    };
  }
  const law = c.trace.find((t) => t.step === 'law');
  const sources = [UI.sourceLibrary(limits.length), law ? `${UI.sourceModel}: ${UI.sourceNone}` : UI.sourceNoModel, UI.sourceNoWeb];
  return {
    ...base,
    kind: 'nolaw',
    happened: c.reason ? UI.noVerdictBecause(c.reason) : UI.noVerdict,
    settle: UI.settleNoLaw(c.quantity, c.raw_unit),
    settlePrint: UI.settleNoLaw(c.quantity, c.raw_unit),
    sources,
  };
}

/** Summary strip: number of claims and of entries in each verdict family. */
export interface Summary {
  claims: number;
  paragraphs: number;
  counts: { kind: VerdictKind; label: string; count: number }[];
}

export function summary(entries: Entry[], paragraphs: number): Summary {
  const counts = VERDICT_KINDS.map((kind) => ({
    kind,
    label: kindLabel(kind),
    count: entries.filter((e) => verdictKind(e) === kind).length,
  })).filter((c) => c.count > 0);
  return { claims: entries.filter((e) => e.kind === 'claim').length, paragraphs, counts };
}

/** Indices of the entries shown under a filter; all of them without one. */
export function filtered(entries: Entry[], kind: VerdictKind | null): number[] {
  return entries.map((e, i) => ({ e, i })).filter(({ e }) => kind === null || verdictKind(e) === kind).map(({ i }) => i);
}

/** The next index in `visible` after (step 1) or before (step -1) `current`, wrapping. */
export function step(visible: number[], current: number, by: 1 | -1): number {
  if (visible.length === 0) return current;
  const at = visible.indexOf(current);
  if (at < 0) return visible[0];
  return visible[(at + by + visible.length) % visible.length];
}

/** Title of an entry in lists: "speed of a massive body · 3.60e+8 m/s". */
export function entryTitle(e: Entry): string {
  return e.kind === 'claim' ? `${quantityName(e.claim)} · ${readValueText(e.claim)}` : `${WB.progressParagraph} ${e.paragraph}`;
}

/** The claim's quantity in plain words: the law's name for it, else the reading's. */
export function quantityName(c: AnalyzedClaim): string {
  return (c.limit_id ? UI.quantityName[c.limit_id] : undefined) ?? c.quantity;
}
