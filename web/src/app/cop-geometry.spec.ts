import { MAX_ARROW, MIN_ARROW, copArrows, copFlows } from './cop-geometry';

describe('copFlows', () => {
  it('300 K and 280 K give a heating bound of 15 and a cooling bound of 14', () => {
    expect(copFlows(300, 280, null, 'heating')?.bound).toBe(15);
    expect(copFlows(300, 280, null, 'cooling')?.bound).toBe(14);
  });

  it('claim 14.5 exceeds the cooling bound and not the heating bound', () => {
    expect(copFlows(300, 280, 14.5, 'cooling')?.exceedsBound).toBe(true);
    expect(copFlows(300, 280, 14.5, 'heating')?.exceedsBound).toBe(false);
  });

  it('heating bound minus cooling bound equals 1', () => {
    for (const [th, tc] of [
      [300, 280],
      [320, 280],
      [300, 250],
      [1000, 1],
    ]) {
      const h = copFlows(th, tc, null, 'heating')?.bound ?? 0;
      const c = copFlows(th, tc, null, 'cooling')?.bound ?? 0;
      expect(h - c).toBeCloseTo(1, 9);
    }
  });

  it('normalizes W to 1 and keeps the balance Qh = Qc + W', () => {
    const heating = copFlows(300, 280, 14.5, 'heating');
    expect(heating?.w).toBe(1);
    expect(heating?.usefulAllowed).toBe(15);
    expect(heating?.otherHeatAllowed).toBe(14);
    expect((heating?.usefulAllowed ?? 0) - (heating?.otherHeatAllowed ?? 0)).toBe(1);
    const cooling = copFlows(300, 280, 14.5, 'cooling');
    expect(cooling?.usefulAllowed).toBe(14);
    expect(cooling?.otherHeatAllowed).toBe(15);
    expect((cooling?.otherHeatAllowed ?? 0) - (cooling?.usefulAllowed ?? 0)).toBe(1);
  });

  it('gives the claimed useful heat as claim x W', () => {
    expect(copFlows(300, 280, 14.5, 'heating')?.usefulClaimed).toBe(14.5);
    expect(copFlows(300, 280, null, 'heating')?.usefulClaimed).toBeNull();
  });

  it('returns null for missing or invalid temperatures', () => {
    expect(copFlows(null, 280, 1, 'heating')).toBeNull();
    expect(copFlows(300, null, 1, 'cooling')).toBeNull();
    expect(copFlows(280, 280, 1, 'heating')).toBeNull();
    expect(copFlows(280, 300, 1, 'heating')).toBeNull();
    expect(copFlows(300, 0, 1, 'cooling')).toBeNull();
  });
});

describe('copArrows', () => {
  it('draws widths proportional to energy, the largest at the maximum width', () => {
    const a = copArrows(copFlows(300, 280, 14.5, 'heating') ?? fail());
    expect(a.useful).toBe(MAX_ARROW);
    expect(a.other / a.useful).toBeCloseTo(14 / 15, 9);
    expect(a.claimed / a.useful).toBeCloseTo(14.5 / 15, 9);
    expect(a.w / a.useful).toBeCloseTo(1 / 15, 9);
  });

  it('never draws an arrow thinner than the minimum width', () => {
    const a = copArrows(copFlows(1000, 999, 2, 'heating') ?? fail());
    expect(a.w).toBe(MIN_ARROW);
    expect(a.useful).toBe(MAX_ARROW);
  });

  it('draws the claimed arrow wider than the allowed one only when the claim exceeds the bound', () => {
    const cooling = copArrows(copFlows(300, 280, 14.5, 'cooling') ?? fail());
    const heating = copArrows(copFlows(300, 280, 14.5, 'heating') ?? fail());
    expect(cooling.claimed).toBeGreaterThan(cooling.useful);
    expect(heating.claimed).toBeLessThan(heating.useful);
  });

  it('draws no claimed arrow without a claim', () => {
    expect(copArrows(copFlows(300, 280, null, 'heating') ?? fail()).claimed).toBe(0);
  });
});

function fail(): never {
  throw new Error('expected valid flows');
}
