import { AnalyzedClaim } from './analysis.models';
import { LIMITS } from './testing-fixtures';
import {
  Entry,
  filtered,
  noVerdict,
  plainSentence,
  readNumber,
  readingSentences,
  sentenceRanges,
  shortLabel,
  valueRange,
  ratioText,
  reviewMarkdown,
  segments,
  step,
  summary,
  verdictKind,
  verdictText,
} from './review-logic';
import { claimOf } from './workbench-logic.spec';

function entry(verdict: 'IMPOSSIBLE' | 'CONDITIONAL' | 'COMPATIBLE' | 'INSUFFICIENT_DATA', over: Partial<AnalyzedClaim> = {}): Entry {
  const base = claimOf();
  const result = { ...base.result!, verdict, wording: verdict === 'CONDITIONAL' ? 'possible only if mode = heating' : base.result!.wording };
  if (verdict === 'INSUFFICIENT_DATA') result.missing_parameters = [{ id: 'Tc', status: 'NoDomain' }];
  return { kind: 'claim', claim: { ...base, result, ...over }, paragraph: 1 };
}
const noLaw: Entry = {
  kind: 'claim',
  claim: claimOf({ limit_id: null, status: 'needs_input', result: null, reason: 'no established law identified for this quantity' }),
  paragraph: 2,
};
const failure: Entry = { kind: 'failure', paragraph: 3, text: 'x 1', failure: { status: 502, code: 'ExtractionFailed', message: 'm' }, attempts: 3 };
const disputed = entry('IMPOSSIBLE', { law: { source: 'draft_disputed', limit_id: null, sentence: null, attempts: [], comparison: null } });

describe('verdict kinds and labels', () => {
  it('maps each verdict to its family and keeps the text label', () => {
    expect(['IMPOSSIBLE', 'CONDITIONAL', 'COMPATIBLE', 'INSUFFICIENT_DATA'].map((v) => verdictKind(entry(v as 'IMPOSSIBLE')))).toEqual([
      'impossible',
      'conditional',
      'compatible',
      'needs',
    ]);
    expect([noLaw, failure, disputed].map(verdictKind)).toEqual(['needs', 'needs', 'disputed']);
    expect([noLaw, failure, disputed].map(verdictText)).toEqual(['NEEDS INPUT', 'NOT ANALYSED', 'DISPUTED']);
    expect(verdictText(entry('INSUFFICIENT_DATA'))).toBe('INSUFFICIENT DATA');
  });

  it('writes one plain sentence from engine data', () => {
    expect(plainSentence(entry('IMPOSSIBLE'), 'Max')).toBe('The claim is 1.20× what the law allows.');
    expect(plainSentence(entry('IMPOSSIBLE'), 'Min')).toBe('The claim is 1.20× the minimum the law requires.');
    expect(plainSentence(entry('COMPATIBLE'), 'Max')).toBe('The claim is 1.20× what the law allows, within the bound.');
    expect(plainSentence(entry('CONDITIONAL'), 'Max')).toBe('Possible only if mode = heating.');
    expect(plainSentence(entry('INSUFFICIENT_DATA'), 'Max')).toBe('The engine needs a value the text does not give: Tc.');
    expect(plainSentence(noLaw, 'Max')).toBe('No verdict: no established law identified for this quantity.');
    expect(plainSentence(failure, 'Max')).toBe('The paragraph could not be read (ExtractionFailed).');
    expect(ratioText(claimOf())).toBe('1.20×');
    expect(ratioText(noLaw.kind === 'claim' ? noLaw.claim : claimOf())).toBeNull();
  });
});

describe('segments of the text', () => {
  it('cuts the text at each claim evidence, from code-point offsets', () => {
    const text = 'Ω engine 60% and 40%.';
    const a = claimOf({ evidence: { text: '60%', start: 9, end: 12 } });
    const b = claimOf({ evidence: { text: '40%', start: 17, end: 20 } });
    const s = segments(text, [
      { index: 0, claim: a },
      { index: 1, claim: b },
    ]);
    expect(s).toEqual([
      { text: 'Ω engine ', claim: null },
      { text: '60%', claim: 0 },
      { text: ' and ', claim: null },
      { text: '40%', claim: 1 },
      { text: '.', claim: null },
    ]);
  });

  it('ignores evidence outside the paragraph and keeps the first of two overlapping claims', () => {
    const text = 'abc 12 def';
    const s = segments(text, [
      { index: 0, claim: claimOf({ evidence: { text: '12', start: 4, end: 6 } }) },
      { index: 1, claim: claimOf({ evidence: { text: '2 d', start: 5, end: 8 } }) },
      { index: 2, claim: claimOf({ evidence: { text: 'x', start: 40, end: 41 } }) },
    ]);
    expect(s.filter((g) => g.claim !== null).map((g) => g.claim)).toEqual([0]);
  });
});

describe('short labels, sentences and values', () => {
  it('labels a claim with the short name of its law and its value, never cut', () => {
    expect(shortLabel(entry('IMPOSSIBLE'))).toBe('efficiency 60.0 %');
    const noLawEntry: Entry = { kind: 'claim', claim: claimOf({ limit_id: null, quantity: 'position-momentum uncertainty product', raw_value: 5, raw_unit: 'kWh/kg' }), paragraph: 1 };
    expect(shortLabel(noLawEntry)).toBe('position momentum 5.00 kWh/kg');
    expect(shortLabel(failure)).toBe('Paragraph 3');
  });

  it('reads numbers as written, powers of ten included', () => {
    expect(readNumber('3.6e8')).toBe(3.6e8);
    expect(readNumber('1 × 10⁻²⁴')).toBe(1e-24);
    expect(readNumber('1,4')).toBe(1.4);
    expect(Number.isNaN(readNumber('abc'))).toBe(true);
  });

  it('cuts a paragraph into sentences and finds the value inside the evidence', () => {
    const text = 'The probe cruises at 3.6e8 m/s. It weighs 2 kg.';
    expect(sentenceRanges(text)).toEqual([
      { start: 0, end: 31 },
      { start: 32, end: 47 },
    ]);
    const c = claimOf({ raw_value: 3.6e8, evidence: { text: 'cruises at 3.6e8 m/s', start: 10, end: 30 } });
    expect(valueRange(text, c)).toEqual({ start: 21, end: 26 });
    const s = readingSentences(text, [{ index: 0, claim: c }]);
    expect(s[0].claim).toBe(0);
    expect(s[0].pieces).toEqual([
      { text: 'The probe cruises at ', claim: null },
      { text: '3.6e8', claim: 0 },
      { text: ' m/s.', claim: null },
    ]);
    expect(s[1].claim).toBeNull();
  });
});

describe('the card of a claim without a verdict', () => {
  it('is null for a claim with a verdict', () => {
    expect(noVerdict(entry('IMPOSSIBLE'), LIMITS)).toBeNull();
  });

  it('lists the missing quantities for insufficient data', () => {
    const card = noVerdict(entry('INSUFFICIENT_DATA'), LIMITS)!;
    expect(card.kind).toBe('needs');
    expect(card.missing.map((m) => m.id)).toEqual(['Tc']);
    expect(card.missing[0].unit).toBe('K');
    expect(card.settle).toContain('Enter it below');
  });

  it('gives both candidates and their divergence for a disputed law', () => {
    const e: Entry = {
      kind: 'claim',
      claim: claimOf({
        limit_id: null,
        result: null,
        law: {
          source: 'draft_disputed',
          limit_id: null,
          sentence: 's',
          comparison: null,
          attempts: [
            { model: 'a', ok: true, attempts: 1, errors: [], definition: { id: 'a', title: 'a', direction: 'max', formula: 'c' }, bound_at_claim: 100 },
            { model: 'b', ok: false, attempts: 3, errors: ['dimension mismatch'], definition: null, bound_at_claim: null },
          ],
        },
      }),
      paragraph: 1,
    };
    const card = noVerdict(e, LIMITS)!;
    expect(card.kind).toBe('disputed');
    expect(card.candidates.map((k) => [k.model, k.formula, k.bound])).toEqual([
      ['a', 'c', 100],
      ['b', 'dimension mismatch', null],
    ]);
    expect(card.divergence).toBeNull();
  });

  it('names the sources searched for a claim with no law', () => {
    const card = noVerdict(noLaw, LIMITS)!;
    expect(card.kind).toBe('nolaw');
    expect(card.sources[0]).toContain(`${LIMITS.length} laws`);
    expect(card.settle).toContain('added to the library');
  });
});

describe('summary, filters and navigation', () => {
  const entries = [entry('IMPOSSIBLE'), entry('CONDITIONAL'), noLaw, entry('IMPOSSIBLE'), disputed];

  it('counts claims and each verdict family', () => {
    const s = summary(entries, 4);
    expect(s.claims).toBe(5);
    expect(s.counts.map((c) => [c.kind, c.label, c.count])).toEqual([
      ['impossible', 'Impossible', 2],
      ['conditional', 'Conditional', 1],
      ['needs', 'Needs input', 1],
      ['disputed', 'Disputed', 1],
    ]);
  });

  it('filters by family and steps through the visible claims, wrapping', () => {
    expect(filtered(entries, null)).toEqual([0, 1, 2, 3, 4]);
    const v = filtered(entries, 'impossible');
    expect(v).toEqual([0, 3]);
    expect(step(v, 0, 1)).toBe(3);
    expect(step(v, 3, 1)).toBe(0);
    expect(step(v, 0, -1)).toBe(3);
    expect(step(v, 2, 1)).toBe(0);
    expect(step([], 2, 1)).toBe(2);
  });
});

describe('Markdown report', () => {
  it('is self-contained: title, date, summary, each claim with verdict, sentence, bound, source, engine', () => {
    const md = reviewMarkdown([entry('IMPOSSIBLE'), noLaw, failure], 3, LIMITS, '2026-11-25 10:00 UTC');
    expect(md.startsWith('# Limina review\n')).toBe(true);
    expect(md).toContain('Date: 2026-11-25 10:00 UTC');
    expect(md).toContain('2 claims, 3 paragraphs.');
    expect(md).toContain('- Impossible: 1');
    expect(md).toContain('- Verdict: IMPOSSIBLE');
    expect(md).toContain('- In plain words: The claim is 1.20× what the law allows.');
    expect(md).toContain('- Bound: 0.500');
    expect(md).toContain('- Ratio: 1.20×');
    expect(md).toContain('- Engine version: 0.1.0');
    expect(md).toContain('- Engine version: not evaluated');
    expect(md).toContain('- Error: ExtractionFailed: m');
    expect(md).toMatch(/- Source: \S/);
    expect(md.trimEnd().endsWith('Generated by Limina; AI-written laws are not validated')).toBe(true);
  });
});
