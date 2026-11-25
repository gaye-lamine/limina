import { describe, expect, it } from 'vitest';
import { CurvePoint, CurveResponse, SurfaceResponse } from './analysis.models';
import { comparisonScale, graphGeometry, heatGeometry } from './graph-geometry';
import { PLOT } from './workbench-logic';

const pt = (x: number, lo: number | null, hi: number | null, d: number | null): CurvePoint => ({
  x,
  b_min: lo,
  b_max: hi,
  decisive: d,
});

const bound = (extra: Partial<CurveResponse> = {}): CurveResponse => ({
  limit_id: 'carnot',
  direction: 'Max',
  free_parameter: 'Th',
  range: [300, 400],
  points: [pt(300, null, null, null), pt(350, 0.1, 0.1, 0.1), pt(400, 0.2, 0.2, 0.2)],
  kind: 'bound',
  validity_zones: [[300, 300]],
  ...extra,
});

describe('graphGeometry', () => {
  it('draws the bound, the forbidden region and the claim line of a bound law', () => {
    const g = graphGeometry(bound(), 0.15);
    expect(g).not.toBeNull();
    expect(g?.kind).toBe('bound');
    expect(g?.line.startsWith('M')).toBe(true);
    expect(g?.forbidden).not.toBe('');
    expect(g?.tolerance).toBe('');
    expect(g?.band).toBe('');
    expect(g?.claimY).not.toBeNull();
  });

  it('breaks the line at an undefined point and marks the validity zone', () => {
    const g = graphGeometry(bound({ validity_zones: [[300, 320]] }), null);
    expect(g?.line.match(/M/g)?.length).toBe(1);
    expect(g?.zones).toHaveLength(1);
    expect(g?.zones[0].x).toBeCloseTo(PLOT.left);
    expect(g?.zones[0].width).toBeGreaterThan(0);
    expect(g?.claimY).toBeNull();
  });

  it('draws the uncertainty band between b_min and b_max', () => {
    const band = [pt(300, null, null, null), pt(350, 0.08, 0.12, 0.12), pt(400, 0.18, 0.22, 0.22)];
    const g = graphGeometry(bound({ band }), null);
    expect(g?.band.startsWith('M')).toBe(true);
    expect(g?.band.endsWith('Z')).toBe(true);
  });

  it('draws the prediction and its tolerance band for an equality law, no forbidden region', () => {
    const upper = [pt(1, 0, 0, 1.1), pt(2, 0, 0, 2.2), pt(3, 0, 0, 3.3)];
    const lower = [pt(1, 0, 0, 0.9), pt(2, 0, 0, 1.8), pt(3, 0, 0, 2.7)];
    const g = graphGeometry(
      bound({ kind: 'equality', range: [1, 3], points: upper, lower, validity_zones: [] }),
      2,
    );
    expect(g?.kind).toBe('equality');
    expect(g?.forbidden).toBe('');
    expect(g?.tolerance.endsWith('Z')).toBe(true);
    expect(g?.zones).toEqual([]);
  });

  it('returns null with fewer than two defined points', () => {
    const c = bound({ points: [pt(300, null, null, null), pt(350, 0.1, 0.1, 0.1)] });
    expect(graphGeometry(c, null)).toBeNull();
  });
});

const surface = (values: (number | null)[][]): SurfaceResponse => ({
  limit_id: 'carnot',
  direction: 'Max',
  x_parameter: 'Th',
  y_parameter: 'Tc',
  x: [1, 2],
  y: [1, 2],
  values,
});

describe('heatGeometry', () => {
  it('shades cells by the bound, darker for a larger bound, and leaves undefined cells without level', () => {
    const h = heatGeometry(
      surface([
        [0.1, 0.5],
        [null, 0.9],
      ]),
      null,
    );
    expect(h?.cells).toHaveLength(4);
    const byLevel = h?.cells.filter((c) => c.level !== null) ?? [];
    expect(byLevel).toHaveLength(3);
    expect(h?.cells.find((c) => c.level === null)?.grey).toBeNull();
    const smallest = h?.cells.find((c) => c.level === 0);
    const largest = h?.cells.find((c) => c.level === 1);
    expect((smallest?.grey ?? 0) > (largest?.grey ?? 255)).toBe(true);
    expect(h?.min).toBe(0.1);
    expect(h?.max).toBe(0.9);
  });

  it('draws the claim contour where the bound crosses the claim', () => {
    const crossing = heatGeometry(
      surface([
        [0.1, 0.5],
        [0.2, 0.9],
      ]),
      0.4,
    );
    expect(crossing?.contour).not.toBe('');
    const none = heatGeometry(
      surface([
        [0.1, 0.5],
        [0.2, 0.9],
      ]),
      2,
    );
    expect(none?.contour).toBe('');
  });

  it('is null when no cell is defined', () => {
    expect(
      heatGeometry(
        surface([
          [null, null],
          [null, null],
        ]),
        1,
      ),
    ).toBeNull();
  });
});

describe('comparisonScale', () => {
  it('places the claim right of the bound when it is larger, on a log axis', () => {
    const s = comparisonScale(1.2, 1, 'Max');
    expect(s?.claimBeyond).toBe(true);
    expect((s?.claimX ?? 0) > (s?.boundX ?? 1)).toBe(true);
    expect(s?.hatch.x).toBe(s?.boundX);
  });

  it('hatches below the bound of a Min law', () => {
    const s = comparisonScale(5, 10, 'Min');
    expect(s?.claimBeyond).toBe(true);
    expect(s?.hatch.x).toBe(PLOT.left);
  });

  it('spaces equal ratios equally', () => {
    const a = comparisonScale(10, 1, 'Max');
    const b = comparisonScale(100, 10, 'Max');
    expect((a?.claimX ?? 0) - (a?.boundX ?? 0)).toBeCloseTo((b?.claimX ?? 0) - (b?.boundX ?? 0), 6);
  });

  it('returns null for a value a log axis cannot show', () => {
    expect(comparisonScale(0, 1, 'Max')).toBeNull();
    expect(comparisonScale(1, -2, 'Max')).toBeNull();
  });
});

describe('logarithmic axes', () => {
  // Heisenberg: dx >= hbar / (2 dp), sampled by the engine on dp in [1e-28, 1e-20].
  const hbar2 = 1.054571817e-34 / 2;
  const xs = [1e-28, 1e-26, 1e-24, 1e-22, 1e-20];
  const heisenberg = (spacing: 'log' | 'linear'): CurveResponse => ({
    limit_id: 'heisenberg',
    direction: 'Min',
    free_parameter: 'dp',
    range: [1e-28, 1e-20],
    points: xs.map((x) => pt(x, hbar2 / x, hbar2 / x, hbar2 / x)),
    kind: 'bound',
    validity_zones: [],
    spacing,
  });

  it('uses log axes when the engine sampled logarithmically and the values span decades', () => {
    const g = graphGeometry(heisenberg('log'), 1e-10);
    expect(g?.logX).toBe(true);
    expect(g?.logY).toBe(true);
    // Evenly spaced decades are evenly spaced on the axis.
    const step = (g?.xTicks[1].pos ?? 0) - (g?.xTicks[0].pos ?? 0);
    expect((g?.xTicks[4].pos ?? 0) - (g?.xTicks[3].pos ?? 0)).toBeCloseTo(step, 6);
    // The claim line lies inside the plot.
    expect(g?.claimY).toBeGreaterThan(PLOT.top);
    expect(g?.claimY).toBeLessThan(PLOT.height - PLOT.bottom);
    expect(g?.yTicks[0].label).toMatch(/e/);
  });

  it('keeps linear axes for a linear curve over a narrow range', () => {
    const g = graphGeometry(bound(), 0.15);
    expect(g?.logX).toBe(false);
    expect(g?.logY).toBe(false);
  });

  it('never uses a log axis with a non-positive value', () => {
    const c = heisenberg('log');
    c.points = [pt(1e-28, 0, 0, 0), ...c.points.slice(1)];
    expect(graphGeometry(c, 1e-10)?.logY).toBe(false);
  });
});
