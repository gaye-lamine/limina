import { SAMPLES, evaluateRequest, sampleClaims, sampleOf } from './samples';
import { claimOf } from './workbench-logic.spec';

describe('built-in samples', () => {
  it('has three illustrative samples whose evidence is found in their text', () => {
    expect(SAMPLES).toHaveLength(3);
    for (const s of SAMPLES) {
      expect(sampleOf(s.text)).toBe(s);
      for (const c of s.claims) {
        expect(s.text).toContain(c.evidence);
        for (const p of Object.values(c.parameters)) expect(s.text).toContain(p.evidence);
      }
    }
    expect(sampleOf('other')).toBeNull();
  });

  it('builds the evaluate request with constraints as lists and no request for a claim without law', () => {
    const wind = SAMPLES[2];
    expect(evaluateRequest(wind.claims[0])).toEqual({
      limit_id: 'betz',
      claim: { value: 0.7, unit: 'dimensionless' },
      constraints: { reference_area: ['unshrouded_rotor'] },
      parameters: {},
    });
    expect(evaluateRequest(SAMPLES[0].claims[5])).toBeNull();
  });

  it('turns engine results into claims with located offsets and a trace without language model', () => {
    const six = SAMPLES[0];
    const result = claimOf().result;
    const claims = sampleClaims(six, six.claims.map((c) => (c.limit_id ? result : null)));
    const chars = Array.from(six.text);
    for (const c of claims) {
      const e = c.evidence!;
      expect(chars.slice(e.start, e.end).join('')).toBe(e.text);
    }
    expect(claims[1].parameters['dp'].evidence?.text).toBe('1e-24 kg*m/s');
    expect(claims[3].parameters['Th'].evidence?.start).toBeGreaterThan(claims[3].evidence!.start);
    expect(claims[0].trace[0]).toEqual({ step: 'extraction', status: 'skipped', detail: expect.stringContaining('no AI was needed') });
    expect(claims[5].status).toBe('needs_input');
    expect(claims[5].reason).toBe('no known law covers this quantity');
  });
});
