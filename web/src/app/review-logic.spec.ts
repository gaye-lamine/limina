import { AnalyzedClaim } from './analysis.models';
import { LIMITS } from './testing-fixtures';
import {
  Entry,
  reviewMarkdown,
  score,
  scoreLine,
  segments,
  stampKind,
  stampLabel,
  stampRotation,
  stampSentence,
} from './review-logic';
import { claimOf } from './workbench-logic.spec';

function withVerdict(verdict: 'IMPOSSIBLE' | 'CONDITIONAL' | 'COMPATIBLE' | 'INSUFFICIENT_DATA', over: Partial<AnalyzedClaim> = {}): Entry {
  const base = claimOf();
  const result = base.result ? { ...base.result, verdict, wording: verdict === 'CONDITIONAL' ? 'possible only if mode = heating' : base.result.wording } : null;
  if (result && verdict === 'INSUFFICIENT_DATA') result.missing_parameters = [{ id: 'Tc', status: 'NoDomain' }];
  return { kind: 'claim', claim: { ...base, result, ...over }, paragraph: 1 };
}

const needsInput: Entry = {
  kind: 'claim',
  claim: claimOf({ limit_id: null, status: 'needs_input', result: null, reason: 'no established law identified for this quantity' }),
  paragraph: 2,
};
const failure: Entry = {
  kind: 'failure',
  paragraph: 3,
  text: 'Turbine 0.7',
  failure: { status: 502, code: 'ExtractionFailed', message: 'no JSON' },
  attempts: 3,
};

describe('stamps', () => {
  it('has one style and label per verdict', () => {
    expect(stampKind(withVerdict('IMPOSSIBLE'))).toBe('impossible');
    expect(stampKind(withVerdict('CONDITIONAL'))).toBe('conditional');
    expect(stampKind(withVerdict('COMPATIBLE'))).toBe('compatible');
    expect(stampKind(withVerdict('INSUFFICIENT_DATA'))).toBe('neutral');
    expect(stampKind(needsInput)).toBe('neutral');
    expect(stampKind(failure)).toBe('neutral');
    expect(stampLabel(withVerdict('INSUFFICIENT_DATA'))).toBe('INSUFFICIENT DATA');
    expect(stampLabel(needsInput)).toBe('NEEDS INPUT');
    expect(stampLabel(failure)).toBe('NOT ANALYSED');
    const disputed = withVerdict('IMPOSSIBLE', { law: { source: 'draft_disputed', limit_id: null, sentence: null, attempts: [], comparison: null } });
    expect(stampKind(disputed)).toBe('neutral');
    expect(stampLabel(disputed)).toBe('DISPUTED');
  });

  it('rotates each stamp deterministically between -3 and +2 degrees', () => {
    for (let seed = 0; seed < 2000; seed += 7) {
      const r = stampRotation(seed);
      expect(r).toBeGreaterThanOrEqual(-3);
      expect(r).toBeLessThanOrEqual(2);
      expect(stampRotation(seed)).toBe(r);
    }
    expect(new Set([0, 1, 2, 3, 4].map(stampRotation)).size).toBeGreaterThan(1);
  });

  it('writes one plain sentence from engine data', () => {
    expect(stampSentence(withVerdict('IMPOSSIBLE'), 'Max')).toBe('1.20 x what the law allows');
    expect(stampSentence(withVerdict('IMPOSSIBLE'), 'Min')).toBe('1.20 x the minimum the law requires');
    expect(stampSentence(withVerdict('CONDITIONAL'), 'Max')).toBe('possible only if mode = heating');
    expect(stampSentence(withVerdict('INSUFFICIENT_DATA'), 'Max')).toBe('missing value: Tc');
    expect(stampSentence(needsInput, 'Max')).toBe('no established law identified for this quantity');
    expect(stampSentence(failure, 'Max')).toBe('the text could not be read');
  });
});

describe('segments', () => {
  it('underlines every number inside the API evidence ranges, shifted by the paragraph start', () => {
    const text = 'The cell stores 5 kWh of energy in a mass of 1 kg.';
    const claim = claimOf({ evidence: { text, start: 100, end: 100 + text.length }, parameters: {} });
    const s = segments(text, 100, [{ index: 4, claim }]);
    expect(s.filter((g) => g.claim === 4).map((g) => g.text)).toEqual(['5', '1']);
    expect(s.map((g) => g.text).join('')).toBe(text);
  });

  it('reads powers of ten as one number and ignores ranges of other paragraphs', () => {
    const text = 'momentum known to within 1 × 10⁻²⁴ kg*m/s at 300 K';
    const claim = claimOf({
      evidence: { text: '1 × 10⁻²⁴ kg*m/s', start: 25, end: 41 },
      parameters: { T: { value: 300, raw_value: 300, unit: 'K', evidence: { text: '300 K', start: 500, end: 505 }, flags: [] } },
    });
    const s = segments(text, 0, [{ index: 0, claim }]);
    expect(s.filter((g) => g.claim === 0).map((g) => g.text)).toEqual(['1 × 10⁻²⁴']);
  });
});

describe('scoreboard and report', () => {
  const entries = [withVerdict('IMPOSSIBLE'), withVerdict('IMPOSSIBLE'), withVerdict('CONDITIONAL'), needsInput, failure];

  it('counts paragraphs, claims and each stamp label', () => {
    const s = score(entries, 6);
    expect(scoreLine(s)).toBe('6 paragraphs, 4 claims');
    expect(s.counts.map((c) => [c.label, c.count, c.kind])).toEqual([
      ['IMPOSSIBLE', 2, 'impossible'],
      ['CONDITIONAL', 1, 'conditional'],
      ['NEEDS INPUT', 1, 'neutral'],
      ['NOT ANALYSED', 1, 'neutral'],
    ]);
    expect(scoreLine(score([withVerdict('COMPATIBLE')], 1))).toBe('1 paragraph, 1 claim');
  });

  it('builds a self-contained Markdown review', () => {
    const md = reviewMarkdown(entries, 6, LIMITS);
    expect(md.startsWith('# Limina review\n')).toBe(true);
    expect(md).toContain('6 paragraphs, 4 claims.');
    expect(md).toContain('- IMPOSSIBLE: 2');
    expect(md).toMatch(/## 1\. efficiency: 60(\.0+)? %/);
    expect(md).toContain('- Verdict: IMPOSSIBLE (built-in law)');
    expect(md).toContain('- In plain words: 1.20 x what the law allows');
    expect(md).toContain('- Bound: 0.5');
    expect(md).toContain('- Engine version: 0.1.0');
    expect(md).toContain('- Engine version: not evaluated');
    expect(md).toMatch(/- Source: \S/);
    expect(md).toContain('## 5. Paragraph 3');
    expect(md.trimEnd().endsWith('Generated by Limina; AI-written laws are not validated')).toBe(true);
  });
});
