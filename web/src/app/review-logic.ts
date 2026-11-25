/**
 * Pure helpers of the stamped review: entries, stamps, underlined numbers,
 * the scoreboard and the Markdown report. Every verdict and number comes
 * from the API; nothing here decides a verdict.
 */
import { AnalyzedClaim } from './analysis.models';
import { ApiFailure, LimitInfo } from './api.models';
import { formatValue } from './chart-geometry';
import { REVIEW, WB } from './strings';
import { badge, explain, verdictLabel } from './workbench-logic';

/** One reviewed item: a claim, or a paragraph the pipeline could not analyse (paragraph is 1-based). */
export type Entry =
  | { kind: 'claim'; claim: AnalyzedClaim; paragraph: number }
  | { kind: 'failure'; paragraph: number; text: string; failure: ApiFailure; attempts: number };

/** Visual style of a stamp; the label is always shown with it. */
export type StampKind = 'impossible' | 'conditional' | 'compatible' | 'neutral';

/** The stamp style: by engine verdict; no verdict, disputed laws and failures are neutral. */
export function stampKind(e: Entry): StampKind {
  if (e.kind === 'failure' || e.claim.law?.source === 'draft_disputed') return 'neutral';
  switch (e.claim.result?.verdict) {
    case 'IMPOSSIBLE':
      return 'impossible';
    case 'CONDITIONAL':
      return 'conditional';
    case 'COMPATIBLE':
      return 'compatible';
    default:
      return 'neutral';
  }
}

/** The stamp text: the engine verdict, or why there is none. */
export function stampLabel(e: Entry): string {
  if (e.kind === 'failure') return REVIEW.notAnalysed;
  if (e.claim.law?.source === 'draft_disputed') return REVIEW.disputed;
  return verdictLabel(e.claim);
}

/** Deterministic rotation of a stamp, in degrees, within [-3, +2]. */
export function stampRotation(seed: number): number {
  const h = Math.imul((Math.trunc(seed) + 1) ^ 0x9e3779b9, 2654435761) >>> 0;
  return -3 + (h % 501) / 100;
}

/** The seed of the stamp of an entry: its position and its evidence offset. */
export function stampSeed(e: Entry, index: number): number {
  const start = e.kind === 'claim' ? (e.claim.evidence?.start ?? 0) : e.paragraph;
  return index * 7919 + start;
}

/** One plain sentence under the stamp, from engine data only. */
export function stampSentence(e: Entry, direction: 'Max' | 'Min' | 'Equal'): string {
  if (e.kind === 'failure') return REVIEW.failedSentence;
  const c = e.claim;
  if (c.law?.source === 'draft_disputed') return REVIEW.disputedSentence;
  const r = c.result;
  if (!r) return c.reason ?? REVIEW.noVerdict;
  const margin = r.bounds?.permissive.margin ?? null;
  if ((r.verdict === 'IMPOSSIBLE' || r.verdict === 'COMPATIBLE') && margin) {
    const ratio = formatValue(margin.low);
    return direction === 'Min' ? REVIEW.timesMinimum(ratio) : REVIEW.timesAllowed(ratio);
  }
  if (r.verdict === 'INSUFFICIENT_DATA') return REVIEW.missing(r.missing_parameters.map((m) => m.id).join(', '));
  return r.wording;
}

/** A piece of a paragraph: plain text, or a traceable number of claim `claim`. */
export interface Segment {
  text: string;
  claim: number | null;
}

const NUMBER = /[-+−]?\d+(?:[.,]\d+)?(?:\s*(?:[eE][-+−]?\d+|[×x*]\s*10(?:\^?[-+−]?\d+|[⁻⁺]?[⁰¹²³⁴⁵⁶⁷⁸⁹]+)))?/g;

/** UTF-16 index of code point `cp` of `s`. */
function cpIndex(s: string, cp: number): number {
  return Array.from(s).slice(0, Math.max(0, cp)).join('').length;
}

/** Evidence ranges of a claim, in code points of the whole text. */
function ranges(c: AnalyzedClaim): { start: number; end: number }[] {
  const out = c.evidence ? [c.evidence] : [];
  for (const p of Object.values(c.parameters)) if (p.evidence) out.push(p.evidence);
  return out;
}

/**
 * The paragraph cut into segments: inside each evidence range given by
 * the API, every number is underlined for its claim. Ranges outside the
 * paragraph are ignored; the first claim wins on overlap.
 */
export function segments(text: string, start: number, claims: { index: number; claim: AnalyzedClaim }[]): Segment[] {
  const length = Array.from(text).length;
  const marks: { from: number; to: number; claim: number }[] = [];
  for (const { index, claim } of claims) {
    for (const r of ranges(claim)) {
      const a = Math.max(0, r.start - start);
      const b = Math.min(length, r.end - start);
      if (b <= a) continue;
      const from = cpIndex(text, a);
      const slice = text.slice(from, cpIndex(text, b));
      for (const m of slice.matchAll(NUMBER)) {
        const s = from + (m.index ?? 0);
        const t = s + m[0].length;
        if (!marks.some((k) => s < k.to && t > k.from)) marks.push({ from: s, to: t, claim: index });
      }
    }
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

/** The scoreboard: counts and, per stamp label, how many entries carry it. */
export interface Score {
  paragraphs: number;
  claims: number;
  counts: { label: string; kind: StampKind; count: number }[];
}

const ORDER = ['IMPOSSIBLE', 'CONDITIONAL', 'COMPATIBLE', 'INSUFFICIENT DATA'];

export function score(entries: Entry[], paragraphs: number): Score {
  const counts: Score['counts'] = [];
  for (const e of entries) {
    const label = stampLabel(e);
    const row = counts.find((c) => c.label === label);
    if (row) row.count += 1;
    else counts.push({ label, kind: stampKind(e), count: 1 });
  }
  const rank = (l: string) => (ORDER.indexOf(l) < 0 ? ORDER.length : ORDER.indexOf(l));
  counts.sort((a, b) => rank(a.label) - rank(b.label));
  return { paragraphs, claims: entries.filter((e) => e.kind === 'claim').length, counts };
}

/** The scoreboard as one line: "6 paragraphs, 8 claims". */
export function scoreLine(s: Score): string {
  return `${REVIEW.paragraphs(s.paragraphs)}, ${REVIEW.claims(s.claims)}`;
}

/** The bound of the engine, in the claim's canonical unit; '-' when there is none. */
export function boundText(c: AnalyzedClaim): string {
  const b = c.result?.bounds?.permissive;
  if (!b) return '-';
  const v = b.b_min === b.b_max ? formatValue(b.b_min) : `${formatValue(b.b_min)} to ${formatValue(b.b_max)}`;
  return c.unit && c.unit !== 'dimensionless' ? `${v} ${c.unit}` : v;
}

/** The claim as read: "speed: 3.6e8 m/s". */
export function claimText(c: AnalyzedClaim): string {
  const unit = c.raw_unit === 'dimensionless' ? '' : c.raw_unit;
  return `${c.quantity}: ${formatValue(c.raw_value)} ${unit}`.trim();
}

/**
 * The review as a self-contained Markdown document: title, scoreboard,
 * each claim with its verdict sentence, bound, source and engine version.
 */
export function reviewMarkdown(entries: Entry[], paragraphs: number, limits: LimitInfo[]): string {
  const s = score(entries, paragraphs);
  const out = [`# ${REVIEW.reportTitle}`, '', `${scoreLine(s)}.`, ''];
  for (const c of s.counts) out.push(`- ${c.label}: ${c.count}`);
  out.push('');
  entries.forEach((e, i) => {
    const direction = e.kind === 'claim' ? (limits.find((l) => l.id === e.claim.limit_id)?.direction ?? 'Max') : 'Max';
    if (e.kind === 'failure') {
      out.push(`## ${i + 1}. ${WB.progressParagraph} ${e.paragraph}`, '');
      out.push(`- ${REVIEW.reportVerdict}: ${stampLabel(e)}`, `- ${REVIEW.reportText}: "${e.text}"`);
      out.push(`- ${REVIEW.reportPlain}: ${stampSentence(e, direction)} (${e.failure.code})`, '');
      return;
    }
    const c = e.claim;
    const limit = limits.find((l) => l.id === c.limit_id);
    out.push(`## ${i + 1}. ${claimText(c)}`, '');
    out.push(`- ${REVIEW.reportVerdict}: ${stampLabel(e)} (${badge(c)})`);
    if (c.evidence) out.push(`- ${REVIEW.reportText}: "${c.evidence.text}"`);
    out.push(`- ${REVIEW.reportPlain}: ${stampSentence(e, direction)}`);
    for (const line of explain(c)) out.push(`- ${line}`);
    if (c.result) out.push(`- ${REVIEW.reportWording}: ${c.result.wording}`);
    out.push(`- ${REVIEW.reportBound}: ${boundText(c)}`);
    if (limit) out.push(`- ${REVIEW.reportLaw}: ${limit.title}, ${limit.formula}`);
    else if (c.law?.sentence) out.push(`- ${REVIEW.reportLaw}: ${c.law.sentence}`);
    const refs = limit?.references ?? [];
    out.push(`- ${REVIEW.reportSource}: ${refs.length ? refs.join('; ') : REVIEW.reportNoSource}`);
    out.push(`- ${REVIEW.reportEngine}: ${c.result?.engine_version ?? REVIEW.reportNotEvaluated}`, '');
  });
  out.push('---', '', REVIEW.reportFooter, '');
  return out.join('\n');
}
