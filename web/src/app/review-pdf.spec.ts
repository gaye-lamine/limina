import { Entry } from './review-logic';
import { PRINT, reviewDocument } from './review-pdf';
import { LIMITS } from './testing-fixtures';
import { claimOf } from './workbench-logic.spec';
import { contrast } from './design-tokens.spec';

const hex = (h: string) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255);

const impossible: Entry = { kind: 'claim', claim: claimOf(), paragraph: 1 };
const noLaw: Entry = {
  kind: 'claim',
  claim: claimOf({ limit_id: null, quantity: 'specific energy', status: 'needs_input', result: null, reason: 'no established law identified for this quantity' }),
  paragraph: 2,
};

describe('the PDF review', () => {
  const doc = reviewDocument([impossible, noLaw], 2, LIMITS, '2026-11-26 10:00 UTC', '0.1.0');
  const text = JSON.stringify(doc.content);

  it('has a title, the summary, how to read it and one block per claim', () => {
    expect(text).toContain('Physics review');
    expect(text).toContain('2 claims checked in 2 paragraphs.');
    expect(text).toContain('How to read this review');
    expect(text).toContain('1. efficiency · 60.0 %');
    expect(text).toContain('The claim is 1.2 times what the law allows.');
    expect(text).toContain('2. specific energy · 60.0 %');
    expect(text).toContain('Our library of 2 laws of physics: none applies to this quantity');
    const footer = (doc.footer as (p: number, n: number) => unknown)(1, 3);
    expect(JSON.stringify(footer)).toContain('1 / 3');
    // Established laws only: no doubt cast on them.
    expect(JSON.stringify(footer)).not.toContain('AI-written');
  });

  it('shows the values used, the calculation, real symbols and DOI links', () => {
    const withParams: Entry = {
      kind: 'claim',
      claim: claimOf({
        parameters: {
          Th: { value: 600, raw_value: 600, unit: 'K', evidence: null, flags: [] },
          Tc: { value: 300, raw_value: 300, unit: 'K', evidence: null, flags: [] },
        },
      }),
      paragraph: 1,
    };
    const limits = LIMITS.map((l) => (l.id === 'carnot' ? { ...l, references: ['Landauer, R., Irreversibility, doi:10.1147/rd.53.0183'] } : l));
    const d = JSON.stringify(reviewDocument([withParams], 1, limits, 'd', '0.1.0').content);
    expect(d).toContain('Th = 600 K');
    expect(d).toContain('1 − 300 K/600 K = 50.0 %');
    expect(d).toContain('≤');
    expect(d).not.toContain('<=');
    expect(d).toContain('"link":"https://doi.org/10.1147/rd.53.0183"');
    expect(d).toContain('the Carnot limit');
  });

  it('defines every status it uses and gives no on-screen instruction', () => {
    const insufficient: Entry = {
      kind: 'claim',
      claim: claimOf({ result: { ...claimOf().result!, verdict: 'INSUFFICIENT_DATA', bounds: null, missing_parameters: [{ id: 'Tc', status: 'NoDomain' }] } }),
      paragraph: 1,
    };
    const d = JSON.stringify(reviewDocument([insufficient, noLaw], 2, LIMITS, 'd', '0.1.0').content);
    expect(d).toContain('INSUFFICIENT DATA');
    expect(d).toContain('NEEDS INPUT');
    expect(d).toContain('The claim could not be checked yet');
    expect(d).not.toMatch(/below|click|press/i);
  });

  it('mentions AI-written laws only when the review uses one', () => {
    const draft: Entry = {
      kind: 'claim',
      claim: claimOf({ law: { source: 'draft_agreed', limit_id: 'draft-x', sentence: 's', attempts: [], comparison: null } }),
      paragraph: 1,
    };
    const footer = reviewDocument([draft], 1, LIMITS, 'd', '0.1.0').footer as (p: number, n: number) => unknown;
    expect(JSON.stringify(footer(1, 1))).toContain('AI-written laws are not validated');
  });

  it('shows no software vocabulary', () => {
    expect(text).not.toMatch(/(?<!heat )\b(engine|extraction|pipeline|api|server|json|markdown|ExtractionFailed)\b/i);
  });

  it('prints verdict colors readable on white (WCAG AA) and on their tint', () => {
    for (const [kind, c] of Object.entries(PRINT)) {
      expect(contrast(hex(c.ink), hex('#ffffff')), `${kind} on white`).toBeGreaterThanOrEqual(4.5);
      expect(contrast(hex(c.ink), hex(c.tint)), `${kind} on its tint`).toBeGreaterThanOrEqual(4.5);
    }
  });
});
