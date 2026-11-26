import { TestBed } from '@angular/core/testing';
import {
  CLAIM_LABEL_Y,
  CX,
  CY,
  GAUGE_FONT,
  GAUGE_MIN_RENDER_WIDTH,
  GAUGE_VIEW_HEIGHT,
  GAUGE_VIEW_WIDTH,
  Gauge,
  TICK_OUTER,
} from './gauge';

async function render(claim: number, bounds: number[], figure = 0): Promise<HTMLElement> {
  await TestBed.configureTestingModule({ imports: [Gauge] }).compileComponents();
  const fixture = TestBed.createComponent(Gauge);
  fixture.componentRef.setInput('claim', claim);
  fixture.componentRef.setInput('bounds', bounds);
  fixture.componentRef.setInput('figure', figure);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

interface Box {
  left: number;
  right: number;
  top: number;
  bottom: number;
}

/** Estimated box of an SVG text, at 0.6 em per monospace character. */
function boxOf(t: Element, size: number): Box {
  const x = Number(t.getAttribute('x'));
  const baseline = Number(t.getAttribute('y'));
  const width = (t.textContent ?? '').trim().length * 0.6 * size;
  const anchor = t.getAttribute('text-anchor') ?? 'start';
  const left = anchor === 'start' ? x : anchor === 'end' ? x - width : x - width / 2;
  return { left, right: left + width, top: baseline - 0.75 * size, bottom: baseline + 0.25 * size };
}

const apart = (a: Box, b: Box) => a.right < b.left || b.right < a.left || a.bottom < b.top || b.bottom < a.top;

describe('Gauge', () => {
  it('labels each configuration bound tick "bound value" and the needle "claim value"', async () => {
    const el = await render(14.5, [14, 15, 14, 15]);
    expect(el.querySelectorAll('.gauge-bound-tick')).toHaveLength(2);
    const labels = Array.from(el.querySelectorAll('.gauge-bound-label')).map((t) => t.textContent?.trim());
    expect(labels).toEqual(['bound 14.0', 'bound 15.0']);
    expect(el.querySelector('.gauge-needle')).not.toBeNull();
    expect(el.querySelector('.gauge-claim-label')?.textContent?.trim()).toBe('claim 14.5');
    TestBed.resetTestingModule();
    const carnot = await render(0.6, [0.5]);
    expect(carnot.querySelector('.gauge-bound-label')?.textContent?.trim()).toBe('bound 0.500');
    expect(carnot.querySelector('.gauge-claim-label')?.textContent?.trim()).toBe('claim 0.600');
  });

  it('hatches the zone beyond the largest bound on a light grey ground, drawn over the dial', async () => {
    const el = await render(0.7, [0.5]);
    const hatch = el.querySelector('.gauge-hatch') as SVGPathElement;
    expect(hatch.getAttribute('fill')).toBe('url(#gauge-hatch)');
    const pattern = el.querySelector('pattern#gauge-hatch') as SVGPatternElement;
    expect(pattern.querySelector('rect.gauge-hatch-ground')).not.toBeNull();
    expect(pattern.querySelector('line.gauge-hatch-line')).not.toBeNull();
    expect(pattern.getAttribute('patternTransform')).toBe('rotate(45)');
    const order = Array.from(el.querySelectorAll('.gauge-dial, .gauge-hatch')).map((n) => n.getAttribute('class'));
    expect(order).toEqual(['gauge-dial', 'gauge-hatch']);
  });

  it('draws bound labels at 14 px or more at its smallest width, set on the element', async () => {
    const el = await render(14.5, [14, 15]);
    const svg = el.querySelector('svg') as SVGElement;
    expect(svg.style.minWidth).toBe(`${GAUGE_MIN_RENDER_WIDTH}px`);
    const scale = GAUGE_MIN_RENDER_WIDTH / GAUGE_VIEW_WIDTH;
    for (const t of Array.from(el.querySelectorAll('.gauge-bound-label'))) {
      expect(Number(t.getAttribute('font-size')) * scale).toBeGreaterThanOrEqual(14);
    }
  });

  it('keeps bound labels outside the arc, inside the frame, apart from each other and from the claim label', async () => {
    for (const [claim, bounds] of [
      [14.5, [14, 15]],
      [0.6, [0.5]],
      [0.7, [0.5]],
      [0.3, [16 / 27]],
      [0.6, [0.5, 0.55]],
      [10, [14, 15]],
    ] as [number, number[]][]) {
      TestBed.resetTestingModule();
      const el = await render(claim, bounds);
      const boxes = Array.from(el.querySelectorAll('.gauge-bound-label')).map((t) => boxOf(t, GAUGE_FONT.bound));
      const claimBox = boxOf(el.querySelector('.gauge-claim-label') as Element, GAUGE_FONT.claim);
      expect(claimBox.bottom).toBeLessThanOrEqual(GAUGE_VIEW_HEIGHT);
      expect(CLAIM_LABEL_Y).toBeGreaterThan(CY);
      for (const b of boxes) {
        // The box clears the dial: its point nearest the centre is beyond the tick ends.
        const nx = Math.min(Math.max(CX, b.left), b.right);
        const ny = Math.min(Math.max(CY, b.top), b.bottom);
        expect(Math.hypot(nx - CX, ny - CY), `${bounds}`).toBeGreaterThan(TICK_OUTER);
        expect(b.left).toBeGreaterThanOrEqual(0);
        expect(b.right).toBeLessThanOrEqual(GAUGE_VIEW_WIDTH);
        expect(b.top).toBeGreaterThanOrEqual(0);
        expect(apart(b, claimBox)).toBe(true);
      }
      for (let i = 0; i < boxes.length; i++) {
        for (let j = i + 1; j < boxes.length; j++) {
          expect(apart(boxes[i], boxes[j]), `${bounds}`).toBe(true);
        }
      }
    }
  });

  it('graduates the scale with readable numbers inside the dial', async () => {
    const el = await render(0.7, [0.5]);
    const labels = Array.from(el.querySelectorAll('.gauge-scale-label')).map((t) => t.textContent?.trim());
    // Only the two ends are written, under the dial, so the needle crosses no label.
    expect(labels).toEqual(['0', '0.8']);
  });

  it('has a text alternative with the claim, the bounds and the position', async () => {
    const el = await render(0.7, [0.5]);
    for (const text of [el.querySelector('svg desc')?.textContent, el.querySelector('.gauge-alt')?.textContent]) {
      expect(text).toContain('claim 0.700');
      expect(text).toContain('bounds 0.500');
      expect(text).toContain('beyond the limit, in the shaded zone');
    }
    expect(el.querySelector('svg')?.getAttribute('role')).toBe('img');
  });

  it('says when the claim is not beyond the largest bound, and numbers its caption', async () => {
    const el = await render(14.5, [14, 15], 2);
    expect(el.querySelector('.gauge-alt')?.textContent).toContain('within the limit');
    expect(el.querySelector('.fig-caption')?.textContent).toMatch(/^Fig\. 2:/);
  });
});
