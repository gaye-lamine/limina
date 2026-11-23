import { TestBed } from '@angular/core/testing';
import {
  COP_FONT,
  COP_FONT_COMPACT,
  COP_MIN_WIDTH,
  COP_MIN_WIDTH_COMPACT,
  COP_VIEW_WIDTH,
  CopFigure,
} from './cop-figure';
import { MAX_ARROW, MIN_ARROW } from './cop-geometry';

interface Inputs {
  th?: string;
  tc?: string;
  claim?: string;
  mode: 'heating' | 'cooling';
  compact?: boolean;
}

async function render(i: Inputs): Promise<HTMLElement> {
  await TestBed.configureTestingModule({ imports: [CopFigure] }).compileComponents();
  const fixture = TestBed.createComponent(CopFigure);
  fixture.componentRef.setInput('th', i.th ?? '300');
  fixture.componentRef.setInput('tc', i.tc ?? '280');
  fixture.componentRef.setInput('claim', i.claim ?? '14.5');
  fixture.componentRef.setInput('mode', i.mode);
  fixture.componentRef.setInput('compact', i.compact ?? false);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

/** Shaft width of a vertical arrow polygon, or of a horizontal one. */
function width(points: string | null | undefined, axis: 'x' | 'y'): number {
  const pts = (points ?? '').split(' ').map((p) => p.split(',').map(Number));
  return axis === 'x' ? Math.abs(pts[1][0] - pts[0][0]) : Math.abs(pts[6][1] - pts[0][1]);
}

const pointsOf = (el: HTMLElement, selector: string) => el.querySelector(selector)?.getAttribute('points');

describe('CopFigure', () => {
  it('puts the hot reservoir on top, the machine in the middle, the cold reservoir at the bottom', async () => {
    const el = await render({ mode: 'heating' });
    const [hot, cold] = Array.from(el.querySelectorAll('.reservoir')).map((r) => Number(r.getAttribute('y')));
    const machine = Number(el.querySelector('.engine')?.getAttribute('cy'));
    expect(hot).toBeLessThan(machine);
    expect(cold).toBeGreaterThan(machine);
    expect(el.querySelector('svg')?.getAttribute('role')).toBe('img');
  });

  it('draws W entering from the side, Qc coming up from the cold reservoir and Qh going up to the hot one', async () => {
    const el = await render({ mode: 'heating' });
    const w = (pointsOf(el, '.cop-work') ?? '').split(' ').map((p) => p.split(',').map(Number));
    expect(w[3][0]).toBeGreaterThan(w[0][0]);
    const [qh, qc] = Array.from(el.querySelectorAll('polygon.flow:not(.cop-work)')).map((p) =>
      (p.getAttribute('points') ?? '').split(' ').map((q) => q.split(',').map(Number)),
    );
    // The tip (fifth point) is above the start: both heat arrows point up.
    expect(qh[4][1]).toBeLessThan(qh[0][1]);
    expect(qc[4][1]).toBeLessThan(qc[0][1]);
    const machine = Number(el.querySelector('.engine')?.getAttribute('cy'));
    expect(qh[4][1]).toBeLessThan(machine);
    expect(qc[0][1]).toBeGreaterThan(machine);
  });

  it('heating: the useful arrow is Qh at the bound 15, with the claimed 14.5 dashed and thinner', async () => {
    const el = await render({ mode: 'heating' });
    const useful = pointsOf(el, 'polygon.cop-useful');
    const other = pointsOf(el, 'polygon.cop-other');
    const claimed = pointsOf(el, 'polygon.cop-claimed');
    expect(width(useful, 'x')).toBeCloseTo(MAX_ARROW, 1);
    expect(width(other, 'x') / width(useful, 'x')).toBeCloseTo(14 / 15, 2);
    expect(width(claimed, 'x')).toBeLessThan(width(useful, 'x'));
    expect(el.querySelector('.cop-useful-label')?.textContent?.replace(/\s+/g, '')).toBe('Qh≤15.0allowed');
    expect(el.querySelector('.cop-claimed-label')?.textContent?.replace(/\s+/g, '')).toBe('Qh=14.5claimed');
    expect(el.querySelector('.cop-other-label')?.textContent?.replace(/\s+/g, '')).toBe('Qc=14.0');
  });

  it('cooling: the useful arrow is Qc at the bound 14, with the claimed 14.5 dashed and wider', async () => {
    const el = await render({ mode: 'cooling' });
    const useful = pointsOf(el, 'polygon.cop-useful');
    const claimed = pointsOf(el, 'polygon.cop-claimed');
    expect(width(claimed, 'x')).toBeGreaterThan(width(useful, 'x'));
    expect(el.querySelector('.cop-useful-label')?.textContent?.replace(/\s+/g, '')).toBe('Qc≤14.0allowed');
    expect(el.querySelector('.cop-other-label')?.textContent?.replace(/\s+/g, '')).toBe('Qh=15.0');
  });

  it('keeps every arrow between the minimum and the maximum width; W = 1 is drawn at least 2 units wide', async () => {
    const el = await render({ mode: 'heating' });
    for (const p of Array.from(el.querySelectorAll('polygon.flow, polygon.flow-claimed'))) {
      const pts = p.getAttribute('points');
      const axis = p.classList.contains('cop-work') ? 'y' : 'x';
      expect(width(pts, axis)).toBeGreaterThanOrEqual(MIN_ARROW - 0.01);
      expect(width(pts, axis)).toBeLessThanOrEqual(MAX_ARROW + 0.01);
    }
    expect(el.querySelector('svg')?.textContent).toContain('= 1');
  });

  it('states the numbers in its text alternative', async () => {
    const el = await render({ mode: 'cooling' });
    const desc = el.querySelector('svg desc')?.textContent ?? '';
    expect(desc).toContain('W = 1');
    expect(desc).toContain('Qc at most 14.0');
    expect(desc).toContain('Qh 15.0');
    expect(desc).toContain('Claimed useful heat 14.5');
    expect(desc).toContain('above the allowed value');
  });

  it('renders no text below 12 px at its smallest drawn width, full size and compact', async () => {
    for (const compact of [false, true]) {
      TestBed.resetTestingModule();
      const el = await render({ mode: 'heating', compact });
      const minWidth = compact ? COP_MIN_WIDTH_COMPACT : COP_MIN_WIDTH;
      const svg = el.querySelector('svg') as SVGElement;
      expect(svg.style.minWidth).toBe(`${minWidth}px`);
      const scale = minWidth / COP_VIEW_WIDTH;
      for (const t of Array.from(el.querySelectorAll('svg text, svg tspan[font-size]'))) {
        expect(Number(t.getAttribute('font-size')) * scale, t.textContent ?? '').toBeGreaterThanOrEqual(12);
      }
      const font = compact ? COP_FONT_COMPACT : COP_FONT;
      expect(Math.min(...Object.values(font)) * scale).toBeGreaterThanOrEqual(12);
    }
  });

  it('asks for valid temperatures instead of drawing flows', async () => {
    const el = await render({ mode: 'heating', th: '280', tc: '300' });
    expect(el.querySelector('polygon.flow')).toBeNull();
    expect(el.querySelector('svg desc')?.textContent).toContain('Set Th above Tc');
  });
});
