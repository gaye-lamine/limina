import { axisTicks, boundScale, ratioText, summarizeBounds, tickStep, withUnit } from './chart-geometry';
import { Bounds } from './api.models';

const PLOT = { plotLeft: 40, plotWidth: 560 };
const scale = (claim: number, boundMin: number, boundMax = boundMin) =>
  boundScale({ claim, boundMin, boundMax, ...PLOT });

describe('boundScale', () => {
  it('puts a claim above the bound inside the hatched region', () => {
    const s = scale(0.7, 0.593);
    expect(s.claimInHatch).toBe(true);
    expect(s.claimX).toBeGreaterThan(s.hatch.x);
    expect(s.claimX).toBeLessThanOrEqual(s.hatch.x + s.hatch.width);
  });

  it('keeps a claim below the bound outside the hatched region', () => {
    const s = scale(0.45, 0.593);
    expect(s.claimInHatch).toBe(false);
    expect(s.claimX).toBeLessThan(s.hatch.x);
  });

  it('does not hatch a claim equal to the bound', () => {
    expect(scale(0.5, 0.5).claimInHatch).toBe(false);
  });

  it('spans the axis from 0 to 1.25 times the larger of claim and bound', () => {
    expect(scale(0.7, 0.593).axisMax).toBeCloseTo(0.875, 12);
    expect(scale(0.45, 0.593).axisMax).toBeCloseTo(0.74125, 12);
  });

  it('maps 0 to the left of the plot and the axis maximum to its right', () => {
    const s = scale(0.7, 0.593);
    expect(s.axisStartX).toBe(40);
    expect(s.axisEndX).toBe(600);
    expect(scale(0, 0.5).claimX).toBe(40);
  });

  it('starts the hatch at the bound and ends it at the right of the axis', () => {
    const s = scale(0.7, 0.593);
    expect(s.hatch.x).toBe(s.boundX);
    expect(s.hatch.x + s.hatch.width).toBeCloseTo(600, 9);
    expect(s.boundX).toBeCloseTo(40 + (560 * 0.593) / 0.875, 9);
  });

  it('is not a range when min equals max', () => {
    expect(scale(0.4, 0.5).isRange).toBe(false);
  });

  it('hatches from the largest bound and keeps the smaller one for the band', () => {
    const s = scale(0.55, 0.4, 0.5);
    expect(s.isRange).toBe(true);
    expect(s.boundMinX).toBeLessThan(s.boundX);
    expect(s.hatch.x).toBe(s.boundX);
    expect(s.claimInHatch).toBe(true);
  });

  it('does not hatch a claim between the smallest and the largest bound', () => {
    expect(scale(0.45, 0.4, 0.5).claimInHatch).toBe(false);
  });

  it('clamps negative values to the left of the plot and survives an all-zero input', () => {
    expect(scale(-1, 0.5).claimX).toBe(40);
    const zero = scale(0, 0);
    expect(zero.axisMax).toBeGreaterThan(0);
    expect(Number.isFinite(zero.boundX)).toBe(true);
  });
});

describe('summarizeBounds', () => {
  const enc = (b_min: number, b_max: number, margin: { low: number; high: number } | null) => ({
    b_min,
    b_max,
    margin,
  });

  it('returns null without bounds', () => {
    expect(summarizeBounds(null)).toBeNull();
  });

  it('returns one value and the margin when both enclosures agree', () => {
    const b: Bounds = {
      permissive: enc(0.593, 0.593, { low: 1.18, high: 1.18 }),
      restrictive: enc(0.593, 0.593, { low: 1.18, high: 1.18 }),
    };
    expect(summarizeBounds(b)).toEqual({ low: 0.593, high: 0.593, margin: { low: 1.18, high: 1.18 } });
  });

  it('spans the smallest b_min and the largest b_max, and hulls the margins', () => {
    const b: Bounds = {
      permissive: enc(0.4, 0.6, { low: 0.9, high: 1.2 }),
      restrictive: enc(0.45, 0.5, null),
    };
    expect(summarizeBounds(b)).toEqual({ low: 0.4, high: 0.6, margin: { low: 0.9, high: 1.2 } });
  });

  it('has no margin when neither enclosure has one', () => {
    const b: Bounds = { permissive: enc(1, 1, null), restrictive: enc(1, 1, null) };
    expect(summarizeBounds(b)?.margin).toBeNull();
  });
});

describe('axisTicks', () => {
  const cases: [number, number][] = [
    [0.7, 0.593],
    [0.45, 0.593],
    [0.6, 0.5],
    [0.4, 0.5],
    [0, 0],
    [3, 0.5],
    [450, 600],
    [0.0012, 0.001],
  ];

  it.each(cases)('claim %d, bound %d: 4 or 5 evenly spaced ticks from 0, inside the axis', (claim, bound) => {
    const s = scale(claim, bound);
    const ticks = axisTicks(s);
    expect(ticks.length).toBeGreaterThanOrEqual(4);
    expect(ticks.length).toBeLessThanOrEqual(5);
    expect(ticks[0].value).toBe(0);
    expect(ticks[0].x).toBe(s.axisStartX);
    const step = ticks[1].value - ticks[0].value;
    ticks.forEach((t, i) => {
      expect(t.value).toBeCloseTo(i * step, 9);
      expect(t.value).toBeLessThanOrEqual(s.axisMax + 1e-12);
      expect(t.x).toBeLessThanOrEqual(s.axisEndX + 1e-9);
      if (i > 0) expect(t.x).toBeGreaterThan(ticks[i - 1].x);
    });
  });

  it('uses readable numbers without float noise', () => {
    const labels = axisTicks(scale(0.7, 0.593)).map((t) => t.label);
    expect(labels).toEqual(['0', '0.2', '0.4', '0.6', '0.8']);
  });

  it('picks a round step', () => {
    expect(tickStep(0.875)).toBe(0.2);
    expect(tickStep(1)).toBe(0.25);
  });
});

describe('withUnit', () => {
  it('omits the unit of a dimensionless quantity', () => {
    expect(withUnit('0.593', 'dimensionless')).toBe('0.593');
    expect(withUnit('0.593', '')).toBe('0.593');
  });

  it('keeps any other unit', () => {
    expect(withUnit('600', 'K')).toBe('600 K');
  });
});

describe('ratioText', () => {
  it('reads "x the bound" below and above the bound', () => {
    expect(ratioText({ low: 0.759375, high: 0.759375 })).toBe('0.759 x the bound');
    expect(ratioText({ low: 1.18125, high: 1.18125 })).toBe('1.18 x the bound');
  });

  it('shows a range of ratios', () => {
    expect(ratioText({ low: 0.9, high: 1.2 })).toBe('0.900 to 1.20 x the bound');
  });
});
