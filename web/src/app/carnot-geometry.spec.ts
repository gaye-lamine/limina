import { UNIT_WIDTH, carnotArrows, carnotFlows } from './carnot-geometry';

describe('carnotFlows', () => {
  it('600 K and 300 K give a bound of 0.5 and a minimum rejected heat of 0.5', () => {
    const f = carnotFlows(600, 300, null);
    expect(f).not.toBeNull();
    expect(f?.qh).toBe(1);
    expect(f?.boundWork).toBe(0.5);
    expect(f?.minReject).toBe(0.5);
    expect(f?.claimWork).toBeNull();
    expect(f?.claimReject).toBeNull();
    expect(f?.exceedsBound).toBe(false);
  });

  it('claim 0.60 exceeds the bound and leaves less than the minimum rejected heat', () => {
    const f = carnotFlows(600, 300, 0.6);
    expect(f?.exceedsBound).toBe(true);
    expect(f?.claimWork).toBe(0.6);
    expect(f?.claimReject).toBeCloseTo(0.4, 12);
    expect(f?.claimReject).toBeLessThan(f?.minReject ?? 0);
  });

  it('claim 0.60 at 600 K and 300 K gives a claimed rejected heat of 0.40', () => {
    expect(carnotFlows(600, 300, 0.6)?.claimReject).toBeCloseTo(0.4, 12);
  });

  it('claim 0.40 does not exceed the bound', () => {
    const f = carnotFlows(600, 300, 0.4);
    expect(f?.exceedsBound).toBe(false);
    expect(f?.claimReject).toBeCloseTo(0.6, 12);
    expect(f?.claimReject).toBeGreaterThan(f?.minReject ?? 1);
  });

  it('a claim exactly at the bound does not exceed it', () => {
    expect(carnotFlows(600, 300, 0.5)?.exceedsBound).toBe(false);
  });

  it('work and rejected heat add up to Qh', () => {
    for (const [th, tc] of [
      [600, 300],
      [500, 250],
      [1000, 1],
      [301, 300],
    ]) {
      const f = carnotFlows(th, tc, null);
      expect((f?.boundWork ?? 0) + (f?.minReject ?? 0)).toBeCloseTo(1, 12);
    }
  });

  it('returns null for missing or invalid temperatures', () => {
    expect(carnotFlows(null, 300, 0.5)).toBeNull();
    expect(carnotFlows(600, null, 0.5)).toBeNull();
    expect(carnotFlows(300, 300, 0.5)).toBeNull();
    expect(carnotFlows(300, 600, 0.5)).toBeNull();
    expect(carnotFlows(600, 0, 0.5)).toBeNull();
    expect(carnotFlows(Number.NaN, 300, 0.5)).toBeNull();
  });
});

describe('carnotArrows', () => {
  it('draws widths proportional to the flows', () => {
    const a = carnotArrows(carnotFlows(600, 300, 0.6) ?? fail());
    expect(a.qh).toBe(UNIT_WIDTH);
    expect(a.work).toBeCloseTo(0.5 * UNIT_WIDTH, 9);
    expect(a.reject).toBeCloseTo(0.5 * UNIT_WIDTH, 9);
    expect(a.claimWork).toBeCloseTo(0.6 * UNIT_WIDTH, 9);
    expect(a.claimReject).toBeCloseTo(0.4 * UNIT_WIDTH, 9);
  });

  it('makes the claimed work arrow thicker than the allowed one only when the claim exceeds the bound', () => {
    const above = carnotArrows(carnotFlows(600, 300, 0.6) ?? fail());
    const below = carnotArrows(carnotFlows(600, 300, 0.4) ?? fail());
    expect(above.claimWork).toBeGreaterThan(above.work);
    expect(below.claimWork).toBeLessThan(below.work);
  });

  it('draws nothing for a missing, negative or overspent claim', () => {
    expect(carnotArrows(carnotFlows(600, 300, null) ?? fail()).claimWork).toBe(0);
    expect(carnotArrows(carnotFlows(600, 300, -1) ?? fail()).claimWork).toBe(0);
    expect(carnotArrows(carnotFlows(600, 300, 1.5) ?? fail()).claimReject).toBe(0);
    expect(carnotArrows(carnotFlows(600, 300, 5) ?? fail()).claimWork).toBe(2 * UNIT_WIDTH);
  });
});

function fail(): never {
  throw new Error('expected valid flows');
}
