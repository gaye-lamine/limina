import { dialAngle, gaugeGeometry, placeBoundLabels, polar, sectorPath } from './gauge-geometry';

describe('dialAngle', () => {
  it('maps 0 to the left end (180) and the maximum to the right end (0)', () => {
    expect(dialAngle(0, 2)).toBe(180);
    expect(dialAngle(2, 2)).toBe(0);
    expect(dialAngle(1, 2)).toBe(90);
  });

  it('draws values outside the dial at its ends', () => {
    expect(dialAngle(-1, 2)).toBe(180);
    expect(dialAngle(5, 2)).toBe(0);
  });
});

describe('gaugeGeometry', () => {
  it('scales the dial to about 1.25 times the largest of claim and bounds', () => {
    expect(gaugeGeometry(0.7, [0.5]).max).toBeCloseTo(0.875, 12);
    expect(gaugeGeometry(14.5, [14, 15]).max).toBeCloseTo(18.75, 12);
  });

  it('puts the needle at the claim angle', () => {
    const g = gaugeGeometry(0.7, [0.5]);
    expect(g.needleAngle).toBeCloseTo(180 - (180 * 0.7) / 0.875, 9);
  });

  it('draws one tick per distinct bound, ascending, labelled with its value', () => {
    const g = gaugeGeometry(14.5, [15, 14, 14, 15]);
    expect(g.boundTicks.map((t) => t.value)).toEqual([14, 15]);
    expect(g.boundTicks.map((t) => t.label)).toEqual(['14.0', '15.0']);
    expect(g.boundTicks[0].angle).toBeCloseTo(dialAngle(14, g.max), 12);
    expect(g.boundTicks[1].angle).toBeCloseTo(dialAngle(15, g.max), 12);
  });

  it('places a claim between two bounds between their ticks', () => {
    const g = gaugeGeometry(14.5, [14, 15]);
    const [low, high] = g.boundTicks;
    expect(g.needleAngle).toBeLessThan(low.angle);
    expect(g.needleAngle).toBeGreaterThan(high.angle);
    expect(g.claimBeyond).toBe(false);
  });

  it('starts the hatched zone at the largest bound', () => {
    const g = gaugeGeometry(14.5, [14, 15]);
    expect(g.hatchStart).toBe(g.boundTicks[1].angle);
    expect(gaugeGeometry(0.7, [0.5]).claimBeyond).toBe(true);
    const beyond = gaugeGeometry(0.7, [0.5]);
    expect(beyond.needleAngle).toBeLessThan(beyond.hatchStart);
  });

  it('graduates the scale from 0 with 4 or 5 marks inside the dial', () => {
    for (const [claim, bounds] of [
      [0.7, [0.5]],
      [14.5, [14, 15]],
      [0.3, [16 / 27]],
    ] as [number, number[]][]) {
      const g = gaugeGeometry(claim, bounds);
      expect(g.scaleTicks.length).toBeGreaterThanOrEqual(4);
      expect(g.scaleTicks.length).toBeLessThanOrEqual(5);
      expect(g.scaleTicks[0]).toEqual({ value: 0, label: '0', angle: 180 });
      for (const t of g.scaleTicks) expect(t.angle).toBeGreaterThanOrEqual(0);
    }
  });

  it('survives an all-zero input', () => {
    const g = gaugeGeometry(0, [0]);
    expect(g.max).toBeGreaterThan(0);
    expect(Number.isFinite(g.needleAngle)).toBe(true);
  });
});

describe('placeBoundLabels', () => {
  const frame = { cx: 220, cy: 160, outer: 136, radius: 142, size: 16 };

  it('anchors labels right of the top at their start, left of it at their end, and centres the top one', () => {
    const p = placeBoundLabels([45, 90, 135], frame);
    expect(p.map((q) => q.anchor)).toEqual(['start', 'middle', 'end']);
    expect(p[0].x).toBeGreaterThan(frame.cx);
    expect(p[2].x).toBeLessThan(frame.cx);
  });

  it('pushes a label down when it is closer than one line to the previous one on its side', () => {
    const g = gaugeGeometry(14.5, [14, 15]);
    const p = placeBoundLabels(
      g.boundTicks.map((t) => t.angle),
      frame,
    );
    expect(p[1].y - p[0].y).toBeGreaterThanOrEqual(frame.size * 1.15 - 1e-9);
  });

  it('moves a pushed label out until its box clears the dial', () => {
    for (const p of placeBoundLabels([40, 38, 36], frame)) {
      const top = p.y - 0.75 * frame.size;
      const bottom = p.y + 0.25 * frame.size;
      const dy = Math.min(Math.abs(frame.cy - top), Math.abs(frame.cy - bottom));
      expect(p.x - frame.cx).toBeGreaterThan(Math.sqrt(frame.outer ** 2 - dy ** 2));
    }
  });
});

describe('polar and sectorPath', () => {
  it('puts angle 90 straight above the centre', () => {
    const p = polar(160, 168, 100, 90);
    expect(p.x).toBeCloseTo(160, 9);
    expect(p.y).toBeCloseTo(68, 9);
  });

  it('builds a closed annular sector path', () => {
    const d = sectorPath(160, 168, 128, 100, 180, 0);
    expect(d.startsWith('M 32.00 168.00 A 128 128')).toBe(true);
    expect(d.endsWith('Z')).toBe(true);
  });
});
