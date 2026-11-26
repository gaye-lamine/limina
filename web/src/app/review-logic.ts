/**
 * Pure helpers of the review screen: verdict kinds and labels, the plain
 * sentence, metric texts, highlighted segments of the text, the summary and
 * the Markdown report. Every verdict and number comes from the API; nothing
 * here decides a verdict.
 */
import { AnalyzedClaim } from './analysis.models';
import { ApiFailure, LimitInfo } from './api.models';
import { formatValue } from './chart-geometry';
import { UI, WB } from './strings';
import { badge, explain, verdictLabel } from './workbench-logic';

/** One reviewed item: a claim, or a paragraph that could not be analysed. Paragraphs are 1-based. */
export type Entry =
  | { kind: 'claim'; claim: AnalyzedClaim; paragraph: number }
  | { kind: 'failure'; paragraph: number; text: string; failure: ApiFailure; attempts: number };

/** The visual family of a verdict; it is always shown with its text label. */
export type VerdictKind = 'impossible' | 'conditional' | 'compatible' | 'needs' | 'disputed';

export const VERDICT_KINDS: VerdictKind[] = ['impossible', 'conditional', 'compatible', 'needs', 'disputed'];

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

/** One plain sentence from engine data only. */
export function plainSentence(e: Entry, direction: 'Max' | 'Min' | 'Equal'): string {
  if (e.kind === 'failure') return UI.failedSentence(e.failure.code);
  const c = e.claim;
  if (c.law?.source === 'draft_disputed') return UI.disputedSentence;
  const r = c.result;
  if (!r) return c.reason ? UI.noVerdictBecause(c.reason) : UI.noVerdict;
  const m = r.bounds?.permissive.margin ?? null;
  if ((r.verdict === 'IMPOSSIBLE' || r.verdict === 'COMPATIBLE') && m) {
    const ratio = formatValue(m.low);
    if (direction === 'Min') return r.verdict === 'IMPOSSIBLE' ? UI.belowMinimum(ratio) : UI.aboveMinimum(ratio);
    return r.verdict === 'IMPOSSIBLE' ? UI.aboveMaximum(ratio) : UI.withinMaximum(ratio);
  }
  if (r.verdict === 'INSUFFICIENT_DATA') return UI.missing(r.missing_parameters.map((p) => p.id).join(', '));
  if (r.verdict === 'CONDITIONAL') return UI.conditional(r.wording);
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
  return e.kind === 'claim' ? `${e.claim.quantity} · ${readValueText(e.claim)}` : `${WB.progressParagraph} ${e.paragraph}`;
}

/**
 * The review as a self-contained Markdown document: title, summary, then
 * each claim with its verdict, plain sentence, engine wording, bound, law,
 * source and engine version.
 */
export function reviewMarkdown(entries: Entry[], paragraphs: number, limits: LimitInfo[], date: string): string {
  const s = summary(entries, paragraphs);
  const out = [`# ${UI.reportTitle}`, '', `${UI.reportDate}: ${date}`, '', `${UI.claimsCount(s.claims)}, ${UI.paragraphsCount(s.paragraphs)}.`, ''];
  for (const c of s.counts) out.push(`- ${c.label}: ${c.count}`);
  out.push('');
  entries.forEach((e, i) => {
    const limit = e.kind === 'claim' ? limits.find((l) => l.id === e.claim.limit_id) : undefined;
    const direction = limit?.direction ?? 'Max';
    out.push(`## ${i + 1}. ${entryTitle(e)}`, '');
    out.push(`- ${UI.reportVerdict}: ${verdictText(e)}`);
    out.push(`- ${UI.reportPlain}: ${plainSentence(e, direction)}`);
    if (e.kind === 'failure') {
      out.push(`- ${UI.reportQuote}: "${e.text}"`, `- ${UI.reportError}: ${e.failure.code}: ${e.failure.message}`, '');
      return;
    }
    const c = e.claim;
    if (c.evidence) out.push(`- ${UI.reportQuote}: "${c.evidence.text}"`);
    for (const line of explain(c)) out.push(`- ${line}`);
    if (c.result) out.push(`- ${UI.reportWording}: ${c.result.wording}`);
    out.push(`- ${UI.metricClaim}: ${claimValueText(c)}`, `- ${UI.metricBound}: ${boundText(c)}`);
    const ratio = ratioText(c);
    if (ratio) out.push(`- ${UI.metricRatio}: ${ratio}`);
    if (limit) out.push(`- ${UI.reportLaw}: ${limit.title}, \`${limit.formula}\``);
    else if (c.law?.sentence) out.push(`- ${UI.reportLaw}: ${c.law.sentence}`);
    out.push(`- ${UI.reportStatus}: ${badge(c)}`);
    if (limit?.validity_domain) out.push(`- ${UI.sourceDomain}: ${limit.validity_domain}`);
    const refs = limit?.references ?? [];
    out.push(`- ${UI.reportSource}: ${refs.length ? refs.join('; ') : UI.reportNoSource}`);
    out.push(`- ${UI.reportEngine}: ${c.result?.engine_version ?? UI.reportNotEvaluated}`, '');
  });
  out.push('---', '', UI.reportFooter, '');
  return out.join('\n');
}
