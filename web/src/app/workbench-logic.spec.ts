import { AnalyzedClaim } from './analysis.models';
import {
  badge,
  charCount,
  explain,
  paragraphs,
  plotGeometry,
  rememberDirection,
  shiftClaim,
  solveSentence,
  verdictLabel,
} from './workbench-logic';

export function claimOf(over: Partial<AnalyzedClaim> = {}): AnalyzedClaim {
  return {
    limit_id: 'carnot',
    quantity: 'efficiency',
    value: 0.6,
    unit: 'dimensionless',
    raw_value: 60,
    raw_unit: '%',
    evidence: { text: '60%', start: 3, end: 6 },
    constraints: {},
    parameters: { Th: { value: 600, raw_value: 600, unit: 'K', evidence: { text: '600 K', start: 10, end: 15 }, flags: [] } },
    notes: [],
    flags: [],
    status: 'evaluated',
    result: {
      verdict: 'IMPOSSIBLE',
      limit_id: 'carnot',
      engine_version: '0.1.0',
      wording: 'inconsistent with limit carnot under assumptions cycle model = ideal',
      bounds: {
        permissive: { b_min: 0.5, b_max: 0.5, margin: { low: 1.2, high: 1.2 } },
        restrictive: { b_min: 0.5, b_max: 0.5, margin: { low: 1.2, high: 1.2 } },
      },
      configurations: [],
      missing_parameters: [],
    },
    reason: null,
    code: null,
    trace: [],
    law: { source: 'built_in', limit_id: 'carnot', sentence: null, attempts: [], comparison: null },
    ...over,
  };
}

describe('workbench logic', () => {
  it('splits paragraphs, keeps those with a digit and counts offsets in characters', () => {
    const text = 'Intro without numbers.\n\nÉté: COP 3.\n \nTurbine 0.45';
    const p = paragraphs(text);
    expect(p.map((x) => x.text)).toEqual(['Été: COP 3.', 'Turbine 0.45']);
    expect(p[0].start).toBe(24);
    expect(charCount('😀a')).toBe(2);
    const emoji = paragraphs('😀\n\nx 1');
    expect(emoji[0].start).toBe(3);
  });

  it('shifts every evidence offset of a claim', () => {
    const c = shiftClaim(claimOf(), 100);
    expect(c.evidence).toEqual({ text: '60%', start: 103, end: 106 });
    expect(c.parameters['Th'].evidence?.start).toBe(110);
  });

  it('explains from the engine margin, never from its own arithmetic', () => {
    rememberDirection('carnot', 'Max');
    expect(explain(claimOf())).toEqual(['The claim is 1.20 times what the law allows.']);
    rememberDirection('draft-landauer', 'Min');
    const landauer = claimOf({
      limit_id: 'draft-landauer',
      law: { source: 'draft_agreed', limit_id: 'draft-landauer', sentence: 'law', attempts: [], comparison: null },
    });
    landauer.result!.bounds!.permissive.margin = { low: 0.0348, high: 0.0348 };
    expect(explain(landauer)).toEqual([
      'The law was written by two language models that agree on it; nobody has reviewed it.',
      'The claim is 0.0348 of the minimum the law requires.',
    ]);
    const disputed = claimOf({
      result: null,
      status: 'needs_input',
      reason: 'the two models wrote different laws; no verdict is given',
      law: { source: 'draft_disputed', limit_id: null, sentence: 'law', attempts: [], comparison: null },
    });
    expect(explain(disputed)[0]).toContain('Two models wrote this law differently');
    expect(verdictLabel(disputed)).toBe('NEEDS INPUT');
    expect(badge(disputed)).toBe('disputed between models');
    expect(badge(claimOf())).toBe('built-in law');
  });

  it('rephrases the solver sentence without changing its numbers', () => {
    expect(
      solveSentence('Not inconsistent with limit carnot only if Th is at least 750 K (other parameters fixed at Tc = 300 K)'),
    ).toBe('It would be consistent only if Th is at least 750 K (other parameters fixed at Tc = 300 K)');
  });

  it('hatches above the bound for a Max limit and below for a Min limit, and breaks on undefined points', () => {
    const points = [
      { x: 300, b_min: null, b_max: null, decisive: null },
      { x: 600, b_min: 0.5, b_max: 0.5, decisive: 0.5 },
      { x: 900, b_min: 0.67, b_max: 0.67, decisive: 0.67 },
    ];
    const max = plotGeometry(points, 'Max', 0.6)!;
    expect(max.bound.startsWith('M')).toBe(true);
    expect(max.bound.split('M')).toHaveLength(2);
    expect(max.hatch).toContain(',16 ');
    expect(max.claimY).not.toBeNull();
    const min = plotGeometry(points, 'Min', null)!;
    expect(min.hatch).toContain(',260 ');
    expect(min.claimY).toBeNull();
    expect(plotGeometry([points[0]], 'Max', 1)).toBeNull();
  });
});
