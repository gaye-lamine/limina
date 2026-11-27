import { AnalyzedClaim, AuthorResponse } from '../analysis.models';
import { LAB } from '../strings';
import { claimOf } from '../testing-fixtures';
import {
  CASCADE_MAX,
  CascadeMember,
  addMember,
  canJoin,
  claimAtStress,
  claimsAtLoad,
  firstBreaking,
  maxStress,
  removeMember,
  requestsAtLoad,
  stressOf,
} from './cascade';
import { attemptsIn, canForge, forgeDone, forgeFailure, forgeRunning, samplesOf } from './forge';
import { CASCADE_KEY, cascadeOf, readCascade } from './inspect';
import { LabResult } from './lab-model';

function member(limitId: string, direction: 'Max' | 'Min', claim: number, bound: number | null): CascadeMember {
  return { limitId, title: limitId, direction, bound, request: { limit_id: limitId, claim: { value: claim, unit: 'dimensionless' }, constraints: {}, parameters: {} } };
}
const verdict = (kind: LabResult['kind']): LabResult => ({ kind, label: kind.toUpperCase(), wording: '', bound: null, ratio: null });

describe('laboratory: system stress test', () => {
  it('measures stress both ways: above 1 is past the limit', () => {
    expect(stressOf(0.6, 0.5, 'Max')).toBeCloseTo(1.2, 12);
    expect(stressOf(2, 4, 'Min')).toBe(2);
    expect(stressOf(1, null, 'Max')).toBeNull();
    expect(claimAtStress(1.2, 0.5, 'Max')).toBeCloseTo(0.6, 12);
    expect(claimAtStress(2, 4, 'Min')).toBe(2);
  });
  it('puts the most constrained law exactly at its limit at 100 % load, the others below in proportion', () => {
    const list = [member('a', 'Max', 0.4, 0.5), member('b', 'Max', 10, 40), member('c', 'Min', 8, 4)];
    expect(maxStress(list)).toBeCloseTo(0.8, 12);
    const at100 = claimsAtLoad(list, 100);
    expect(stressOf(at100[0], 0.5, 'Max')).toBeCloseTo(1, 12);
    expect(stressOf(at100[1], 40, 'Max')).toBeCloseTo(0.3125, 12);
    expect(stressOf(at100[2], 4, 'Min')).toBeCloseTo(0.625, 12);
    const at150 = claimsAtLoad(list, 150);
    expect(stressOf(at150[0], 0.5, 'Max')).toBeCloseTo(1.5, 12);
    // A minimum is loaded by lowering its claim.
    expect(at150[2]).toBeLessThan(at100[2]);
    expect(requestsAtLoad(list, 150)[1].claim.value).toBeCloseTo(at150[1], 12);
    // Without a known limit the claim stays.
    expect(claimsAtLoad([member('x', 'Max', 3, null)], 120)).toEqual([3]);
  });
  it('names as first breaking limit the most constrained law the engine finds IMPOSSIBLE', () => {
    const list = [member('a', 'Max', 0.4, 0.5), member('b', 'Max', 10, 40), member('c', 'Min', 8, 4)];
    expect(firstBreaking(list, [verdict('compatible'), verdict('compatible'), verdict('compatible')])).toBeNull();
    expect(firstBreaking(list, [verdict('impossible'), verdict('compatible'), verdict('compatible')])).toBe('a');
    expect(firstBreaking(list, [verdict('impossible'), verdict('impossible'), verdict('impossible')])).toBe('a');
    // Only the engine's verdict counts: a draft held at CONDITIONAL never breaks.
    expect(firstBreaking(list, [verdict('conditional'), verdict('impossible'), null])).toBe('b');
  });
  it('holds two to four bound laws, once each', () => {
    let list: CascadeMember[] = [];
    for (const id of ['a', 'b', 'a', 'c', 'd', 'e']) list = addMember(list, member(id, 'Max', 1, 1));
    expect(list.map((m) => m.limitId)).toEqual(['a', 'b', 'c', 'd']);
    expect(list.length).toBe(CASCADE_MAX);
    expect(removeMember(list, 'b').map((m) => m.limitId)).toEqual(['a', 'c', 'd']);
    expect(canJoin({ direction: 'Equal' })).toBe(false);
    expect(canJoin({ direction: 'Min' })).toBe(true);
  });
  it('sends the analysed claims of a text, one per law, four at most, and reads them back', () => {
    const claims: AnalyzedClaim[] = ['carnot', 'betz', 'carnot', 'cop', 'eddington_luminosity', 'speed_limit'].map((id) => claimOf({ limit_id: id }));
    const list = cascadeOf(claims);
    expect(list.map((c) => c.limit_id)).toEqual(['carnot', 'betz', 'cop', 'eddington_luminosity']);
    expect(readCascade({ [CASCADE_KEY]: list })).toEqual(list);
    expect(readCascade({ [CASCADE_KEY]: [{ limit_id: 3 }] })).toEqual([]);
    expect(readCascade(null)).toEqual([]);
    expect(cascadeOf([claimOf({ value: null })])).toEqual([]);
  });
});

describe('laboratory: forge a new law', () => {
  const answer: AuthorResponse = {
    limit_id: 'magnetic_pressure',
    status: 'draft',
    attempts: 2,
    report: {
      limit_id: 'magnetic_pressure',
      validated: false,
      dimension: 'passed',
      formulas: [
        { configuration: 'a = x', formula: 'B^2/(2 mu0)', samples: 125, finite: true, non_negative: true },
        { configuration: 'a = y', formula: 'B^2/(2 mu0)', samples: 75, finite: true, non_negative: true },
      ],
      validity: [{ condition: 'B > 0', held: 190, failed: 10 }],
    },
  };
  it('accepts a sentence that is not blank and not too long', () => {
    expect(canForge('  ')).toBe(false);
    expect(canForge('The lift of a wing cannot exceed ...')).toBe(true);
    expect(canForge('x'.repeat(2001))).toBe(false);
  });
  it('shows the progress in plain words, then the checks the answer reports', () => {
    expect(forgeRunning().every((s) => s.state === 'running')).toBe(true);
    expect(samplesOf(answer)).toBe(200);
    const done = forgeDone(answer);
    expect(done.map((s) => s.text)).toEqual([LAB.forge.stepWritten(2), LAB.forge.stepUnitsOk, LAB.forge.stepGrid(200), LAB.forge.stepConditions(10)]);
    expect(done.every((s) => s.state === 'done')).toBe(true);
    const refusedUnits = forgeDone({ ...answer, report: { ...answer.report, dimension: 'm does not match s' } });
    expect(refusedUnits[1]).toEqual({ text: LAB.forge.stepUnitsNo, state: 'failed' });
  });
  it('explains a refusal without technical words, keeping the number of tries', () => {
    const msg = 'no valid definition after 3 attempts; last error: the definition does not match the schema: unknown variant';
    expect(attemptsIn(msg)).toBe(3);
    const text = forgeFailure({ status: 422, code: 'LimitNotAuthored', message: msg });
    expect(text).toBe(LAB.forge.notAuthored(3));
    expect(forgeFailure({ status: 429, code: 'RateLimited', message: 'too many requests from this address; try again in 60 minutes' })).toBe(LAB.forge.limited);
    for (const code of ['LimitNotAuthored', 'ExtractionUnavailable', 'RateLimited', 'ExtractionProviderError', 'EmptyText', 'TextTooLong', 'NetworkError', 'Other']) {
      const t = forgeFailure({ status: 422, code, message: msg });
      expect(t).not.toMatch(/(?<!heat )\b(engine|extraction|pipeline|api|server|json|schema|variant|llm|model|rust)\b/i);
    }
  });
});
