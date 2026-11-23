import { ComponentFixture, TestBed } from '@angular/core/testing';
import { FIGURE_MAX_HEIGHT, MIN_FIGURE_SCALE } from './bench-figure';
import { CarnotApparatus, FONT, MIN_RENDER_WIDTH, VIEW_HEIGHT, VIEW_WIDTH } from './carnot-apparatus';

async function render(th: string, tc: string, claim: string): Promise<ComponentFixture<CarnotApparatus>> {
  await TestBed.configureTestingModule({ imports: [CarnotApparatus] }).compileComponents();
  const fixture = TestBed.createComponent(CarnotApparatus);
  fixture.componentRef.setInput('th', th);
  fixture.componentRef.setInput('tc', tc);
  fixture.componentRef.setInput('claim', claim);
  await fixture.whenStable();
  return fixture;
}

/** Half-width of an arrow polygon: distance from its centre line to the first shaft corner. */
function shaftWidth(points: string | null | undefined, axis: 'x' | 'y'): number {
  const pts = (points ?? '').split(' ').map((p) => p.split(',').map(Number));
  return axis === 'x' ? Math.abs(pts[1][0] - pts[0][0]) : Math.abs(pts[6][1] - pts[0][1]);
}

describe('CarnotApparatus', () => {
  it('draws the hot reservoir on top, the engine, the cold reservoir at the bottom, as a labelled figure', async () => {
    const el = (await render('600', '300', '0.4')).nativeElement as HTMLElement;
    const svg = el.querySelector('svg.apparatus-figure') as SVGElement;
    expect(svg.getAttribute('role')).toBe('img');
    const reservoirs = Array.from(svg.querySelectorAll('.reservoir')).map((r) => Number(r.getAttribute('y')));
    expect(reservoirs).toHaveLength(2);
    expect(reservoirs[0]).toBeLessThan(Number(svg.querySelector('.engine')?.getAttribute('cy')));
    expect(reservoirs[1]).toBeGreaterThan(Number(svg.querySelector('.engine')?.getAttribute('cy')));
    expect(el.querySelector('.fig-caption')?.textContent).toMatch(/^Fig\. 1:/);
  });

  it('puts the thermometers and the claimed efficiency on the figure, labelled and keyboard accessible', async () => {
    const el = (await render('600', '300', '0.4')).nativeElement as HTMLElement;
    for (const id of ['Th', 'Tc']) {
      const number = el.querySelector(`#param-${id}-value`) as HTMLInputElement;
      const range = el.querySelector(`#param-${id}-range`) as HTMLInputElement;
      expect(number.type).toBe('text');
      expect(number.getAttribute('inputmode')).toBe('decimal');
      expect(number.classList.contains('decimal')).toBe(true);
      expect(range.type).toBe('range');
      expect(el.querySelector(`label[for="param-${id}-value"]`)?.textContent?.replace(/\s+/g, '')).toBe(id);
      expect(range.getAttribute('aria-label')).toContain(id);
    }
    const claim = el.querySelector('#claim-value') as HTMLInputElement;
    expect(claim.type).toBe('text');
    expect(claim.getAttribute('inputmode')).toBe('decimal');
    expect(claim.classList.contains('decimal')).toBe(true);
    expect(el.querySelector('label[for="claim-value"]')?.textContent?.trim()).toBe('Claimed efficiency');
  });

  it('emits the new values from the thermometers and the claim field', async () => {
    const fixture = await render('600', '300', '0.4');
    const el = fixture.nativeElement as HTMLElement;
    const got: string[] = [];
    fixture.componentInstance.thChange.subscribe((v) => got.push(`Th=${v}`));
    fixture.componentInstance.tcChange.subscribe((v) => got.push(`Tc=${v}`));
    fixture.componentInstance.claimChange.subscribe((v) => got.push(`claim=${v}`));
    const range = el.querySelector('#param-Th-range') as HTMLInputElement;
    range.value = '700';
    range.dispatchEvent(new Event('input'));
    const tc = el.querySelector('#param-Tc-value') as HTMLInputElement;
    tc.value = '350';
    tc.dispatchEvent(new Event('input'));
    const claim = el.querySelector('#claim-value') as HTMLInputElement;
    claim.value = '0.55';
    claim.dispatchEvent(new Event('input'));
    expect(got).toEqual(['Th=700', 'Tc=350', 'claim=0.55']);
  });

  it('for a claim above the bound, draws a thicker dashed work arrow and the callout', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const flows = Array.from(el.querySelectorAll('polygon.flow'));
    const allowedWork = flows[1].getAttribute('points');
    const claimed = el.querySelector('polygon.work-claimed');
    expect(shaftWidth(claimed?.getAttribute('points'), 'y')).toBeGreaterThan(shaftWidth(allowedWork, 'y'));
    const lines = Array.from(el.querySelectorAll('.callout tspan')).map((t) => t.textContent?.trim());
    expect(lines.join(' ')).toBe('rejected heat would fall below the minimum Tc/Th');
  });

  it('for a claim within the bound, draws a thinner dashed work arrow and no callout', async () => {
    const el = (await render('600', '300', '0.4')).nativeElement as HTMLElement;
    const allowedWork = el.querySelectorAll('polygon.flow')[1].getAttribute('points');
    const claimed = el.querySelector('polygon.work-claimed')?.getAttribute('points');
    expect(shaftWidth(claimed, 'y')).toBeLessThan(shaftWidth(allowedWork, 'y'));
    expect(el.querySelector('.callout')).toBeNull();
  });

  it('draws arrow widths in proportion: Qh = 1, W = 1 - Tc/Th, Qc = Tc/Th', async () => {
    const el = (await render('600', '200', '')).nativeElement as HTMLElement;
    const [qh, work, reject] = Array.from(el.querySelectorAll('polygon.flow')).map((p) => p.getAttribute('points'));
    const unit = shaftWidth(qh, 'x');
    // Polygon points are written to 0.01 units.
    expect(shaftWidth(work, 'y') / unit).toBeCloseTo(2 / 3, 3);
    expect(shaftWidth(reject, 'x') / unit).toBeCloseTo(1 / 3, 3);
    expect(el.querySelector('polygon.flow-claimed')).toBeNull();
  });

  it('animates the flow direction with dashed strokes', async () => {
    const el = (await render('600', '300', '0.4')).nativeElement as HTMLElement;
    expect(el.querySelectorAll('.flow-dash')).toHaveLength(3);
  });

  it('states the numbers in its text alternative', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const desc = el.querySelector('svg desc')?.textContent ?? '';
    expect(desc).toContain('Qh = 1');
    expect(desc).toContain('at most 0.500');
    expect(desc).toContain('Qc 0.500');
    expect(desc).toContain('Claimed work 0.600');
    expect(desc).toContain('leaving rejected heat 0.400');
    expect(desc).toContain('below the minimum Tc/Th');
  });

  it('asks for valid temperatures instead of drawing flows', async () => {
    const el = (await render('300', '600', '0.4')).nativeElement as HTMLElement;
    expect(el.querySelector('polygon.flow')).toBeNull();
    expect(el.querySelector('svg desc')?.textContent).toContain('Set Th above Tc');
    expect(el.querySelector('.apparatus-note')?.textContent).toContain('Set Th above Tc');
  });

  it('draws the claimed rejected heat next to the minimum, thinner when the claim exceeds the bound, with its value', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const minimum = el.querySelector('polygon.reject-minimum')?.getAttribute('points');
    const claimed = el.querySelector('polygon.reject-claimed')?.getAttribute('points');
    expect(claimed).toBeTruthy();
    expect(shaftWidth(claimed, 'x')).toBeLessThan(shaftWidth(minimum, 'x'));
    // Side by side: the claimed arrow lies entirely right of the minimum one.
    const xs = (p: string | null | undefined) => (p ?? '').split(' ').map((q) => Number(q.split(',')[0]));
    expect(Math.min(...xs(claimed))).toBeGreaterThan(Math.max(...xs(minimum)));
    const label = el.querySelector('.reject-claimed-label')?.textContent?.replace(/\s+/g, '');
    expect(label).toBe('Qc=0.400');
    const texts = Array.from(el.querySelectorAll('svg text')).map((t) => t.textContent?.trim());
    expect(texts).toContain('claimed');
  });

  it('draws the claimed rejected heat wider than the minimum for a claim within the bound', async () => {
    const el = (await render('600', '300', '0.4')).nativeElement as HTMLElement;
    const minimum = el.querySelector('polygon.reject-minimum')?.getAttribute('points');
    const claimed = el.querySelector('polygon.reject-claimed')?.getAttribute('points');
    expect(shaftWidth(claimed, 'x')).toBeGreaterThan(shaftWidth(minimum, 'x'));
    expect(el.querySelector('.reject-claimed-label')?.textContent?.replace(/\s+/g, '')).toBe('Qc=0.600');
  });

  it('labels each reservoir once, inside its block', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const text = el.textContent ?? '';
    expect(text.match(/hot reservoir/g)).toHaveLength(1);
    expect(text.match(/cold reservoir/g)).toHaveLength(1);
    expect(el.querySelector('.thermometer')?.textContent).not.toContain('reservoir');
  });

  it('renders no text below 12 px at its smallest drawn width', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const scale = MIN_RENDER_WIDTH / VIEW_WIDTH;
    const texts = Array.from(el.querySelectorAll('svg.apparatus-figure text'));
    expect(texts.length).toBeGreaterThan(10);
    for (const t of texts) {
      const size = Number(t.getAttribute('font-size'));
      expect(size * scale, t.textContent ?? '').toBeGreaterThanOrEqual(12);
      for (const span of Array.from(t.querySelectorAll('tspan[font-size]'))) {
        expect(Number(span.getAttribute('font-size')) * scale).toBeGreaterThanOrEqual(12);
      }
    }
    expect(Math.min(...Object.values(FONT)) * scale).toBeGreaterThanOrEqual(12);
  });

  it('gives the callout its own room: larger text, right of the claimed arrow, above the cold reservoir', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const callout = el.querySelector('.callout-text') as SVGTextElement;
    expect(Number(callout.getAttribute('font-size'))).toBe(FONT.label);
    const spans = Array.from(callout.querySelectorAll('tspan'));
    const top = Number(callout.getAttribute('y'));
    const lastBaseline = top + spans.slice(1).reduce((s, sp) => s + Number(sp.getAttribute('dy')), 0);
    const coldTop = Number(el.querySelectorAll('.reservoir')[1].getAttribute('y'));
    expect(lastBaseline + 4).toBeLessThan(coldTop);
    const claimedLabelY = Number(el.querySelector('.reject-claimed-label')?.getAttribute('y'));
    expect(top - FONT.label).toBeGreaterThan(claimedLabelY + 20);
    // Longest line, at 0.6 em per monospace character, stays inside the frame.
    const longest = Math.max(...spans.map((s) => (s.textContent ?? '').length));
    expect(Number(callout.getAttribute('x')) + longest * 0.6 * FONT.label).toBeLessThanOrEqual(VIEW_WIDTH);
  });

  it('limits the figure to about 60vh, keeps its aspect ratio and its minimum size for 12 px text', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const css = Array.from(document.querySelectorAll('style'))
      .map((s) => s.textContent ?? '')
      .join('\n');
    expect(FIGURE_MAX_HEIGHT).toContain('60vh');
    expect(css).toMatch(/\.apparatus-figure[^{]*\{[^}]*max-height:\s*min\(60vh, calc\(100vh - 26rem\)\)/);
    expect(css).toMatch(/\.apparatus-figure[^{]*\{[^}]*height:\s*auto/);
    const svg = el.querySelector('svg.apparatus-figure') as SVGElement;
    expect(svg.getAttribute('viewBox')).toBe(`0 0 ${VIEW_WIDTH} ${VIEW_HEIGHT}`);
    expect(svg.style.minWidth).toBe(`${MIN_RENDER_WIDTH}px`);
    expect(Number(svg.style.minHeight.replace('px', '')) / VIEW_HEIGHT).toBeGreaterThanOrEqual(MIN_FIGURE_SCALE);
    expect(css).toMatch(/\.apparatus-scroll[^{]*\{[^}]*overflow-x:\s*auto/);
  });

  it('puts the thermometers and the claim in one compact row directly under the figure', async () => {
    const el = (await render('600', '300', '0.6')).nativeElement as HTMLElement;
    const controls = el.querySelector('.apparatus-controls') as HTMLElement;
    expect(controls.previousElementSibling?.classList.contains('apparatus-scroll')).toBe(true);
    expect(Array.from(controls.children).map((c) => c.className)).toEqual([
      'apparatus-th',
      'apparatus-tc',
      'apparatus-claim',
    ]);
    const css = Array.from(document.querySelectorAll('style'))
      .map((s) => s.textContent ?? '')
      .join('\n');
    expect(css).toMatch(/\.apparatus-controls[^{]*\{[^}]*display:\s*grid/);
    expect(css).toMatch(/\.apparatus-controls[^{]*\{[^}]*grid-template-columns:\s*repeat\(auto-fit/);
  });

  it('accepts a decimal comma and keeps the text as typed', async () => {
    const el = (await render('600', '300', '0,60')).nativeElement as HTMLElement;
    expect((el.querySelector('#claim-value') as HTMLInputElement).value).toBe('0,60');
    expect(el.querySelector('.callout')).not.toBeNull();
    expect(el.querySelector('svg desc')?.textContent).toContain('Claimed work 0.600');
  });
});
