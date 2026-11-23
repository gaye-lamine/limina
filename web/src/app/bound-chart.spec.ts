import { TestBed } from '@angular/core/testing';
import { BoundChart } from './bound-chart';

async function render(
  claim: number,
  boundMin: number,
  boundMax = boundMin,
  extra: Record<string, unknown> = {},
): Promise<HTMLElement> {
  await TestBed.configureTestingModule({ imports: [BoundChart] }).compileComponents();
  const fixture = TestBed.createComponent(BoundChart);
  fixture.componentRef.setInput('claim', claim);
  fixture.componentRef.setInput('boundMin', boundMin);
  fixture.componentRef.setInput('boundMax', boundMax);
  for (const [name, value] of Object.entries(extra)) fixture.componentRef.setInput(name, value);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

const attr = (el: Element | null, name: string) => Number(el?.getAttribute(name));

describe('BoundChart', () => {
  it('draws a thick bound line, a thin claim line and a hatch, as inline SVG', async () => {
    const el = await render(0.7, 0.593);
    const svg = el.querySelector('svg');
    expect(svg?.getAttribute('role')).toBe('img');
    expect(el.querySelector('.chart-bound')).not.toBeNull();
    expect(el.querySelector('.chart-claim')).not.toBeNull();
    expect(el.querySelector('.chart-hatch')?.getAttribute('fill')).toBe('url(#bound-hatch)');
    expect(el.querySelector('pattern#bound-hatch')).not.toBeNull();
  });

  it('puts a claim above the bound inside the hatched rectangle', async () => {
    const el = await render(0.7, 0.593);
    const hatch = el.querySelector('.chart-hatch');
    const claimX = attr(el.querySelector('.chart-claim'), 'x1');
    expect(claimX).toBeGreaterThan(attr(hatch, 'x'));
    expect(claimX).toBeLessThanOrEqual(attr(hatch, 'x') + attr(hatch, 'width'));
  });

  it('keeps a claim below the bound outside the hatched rectangle', async () => {
    const el = await render(0.45, 0.593);
    const claimX = attr(el.querySelector('.chart-claim'), 'x1');
    expect(claimX).toBeLessThan(attr(el.querySelector('.chart-hatch'), 'x'));
  });

  it('labels the bound below the axis and the claim above it', async () => {
    const el = await render(0.7, 0.593);
    const bound = el.querySelector('.chart-label-bound');
    const claim = el.querySelector('.chart-label-claim');
    const axisY = attr(el.querySelector('.chart-axis'), 'y1');
    expect(bound?.textContent).toContain('0.593');
    expect(claim?.textContent).toContain('0.700');
    expect(attr(bound, 'y')).toBeGreaterThan(axisY);
    expect(attr(claim, 'y')).toBeLessThan(axisY);
  });

  it('has a text alternative with the same information', async () => {
    const el = await render(0.7, 0.593);
    const desc = el.querySelector('svg desc')?.textContent ?? '';
    const caption = el.querySelector('.chart-caption')?.textContent ?? '';
    for (const text of [desc, caption]) {
      expect(text).toContain('0.700');
      expect(text).toContain('0.593');
      expect(text).toContain('above the bound');
    }
    expect(el.querySelector('svg title')?.textContent).toContain('bound');
  });

  it('says the claim is within the bound when it is below', async () => {
    const el = await render(0.45, 0.593);
    expect(el.querySelector('.chart-caption')?.textContent).toContain('within the bound');
  });

  it('shows a range as a band and a "low to high" label', async () => {
    const el = await render(0.45, 0.4, 0.5);
    expect(el.querySelector('.chart-band')).not.toBeNull();
    expect(el.querySelector('.chart-label-bound')?.textContent).toContain('0.400 to 0.500');
    expect(el.querySelector('.chart-caption')?.textContent).toContain('range');
  });

  it('draws no band for a single bound and uses no color attribute', async () => {
    const el = await render(0.45, 0.593);
    expect(el.querySelector('.chart-band')).toBeNull();
    const markup = el.innerHTML;
    expect(markup).not.toMatch(/(rgb|hsl)a?\(/i);
    expect(markup).not.toMatch(/(red|green|blue|orange|yellow|purple)/i);
  });

  it('labels the bound with the limit name and value, and the claim as "claim"', async () => {
    const el = await render(0.45, 16 / 27, 16 / 27, { boundName: 'Betz limit', unit: 'dimensionless' });
    expect(el.querySelector('.chart-label-bound')?.textContent?.trim()).toBe('Betz limit 0.593');
    expect(el.querySelector('.chart-label-claim')?.textContent?.trim()).toBe('claim 0.450');
  });

  it('shows the unit when the quantity has one', async () => {
    const el = await render(500, 600, 600, { boundName: 'Power limit', unit: 'W' });
    expect(el.querySelector('.chart-label-bound')?.textContent?.trim()).toBe('Power limit 600 W');
    expect(el.querySelector('.chart-label-claim')?.textContent?.trim()).toBe('claim 500 W');
  });

  it('labels the hatched region "beyond the bound" inside it', async () => {
    const el = await render(0.7, 0.593);
    const label = el.querySelector('.chart-beyond') as SVGTextElement;
    const lines = Array.from(label.querySelectorAll('tspan')).map((t) => t.textContent?.trim());
    expect(lines.join(' ')).toBe('beyond the bound');
    const hatch = el.querySelector('.chart-hatch');
    expect(attr(label, 'x')).toBeGreaterThan(attr(hatch, 'x'));
    expect(attr(label, 'x')).toBeLessThanOrEqual(attr(hatch, 'x') + attr(hatch, 'width'));
  });

  it('draws 4 or 5 axis ticks with readable numbers', async () => {
    const el = await render(0.7, 0.593);
    const labels = Array.from(el.querySelectorAll('.chart-tick-label')).map((t) => t.textContent?.trim());
    expect(labels).toEqual(['0', '0.2', '0.4', '0.6', '0.8']);
    expect(el.querySelectorAll('.chart-tick')).toHaveLength(5);
  });

  it('gives the claim / bound ratio in the text alternative', async () => {
    const el = await render(0.7, 0.593, 0.593, { margin: { low: 1.18125, high: 1.18125 } });
    for (const text of [el.querySelector('svg desc')?.textContent, el.querySelector('.chart-caption')?.textContent]) {
      expect(text).toContain('Claim / bound: 1.18 x the bound');
    }
  });

  it('is about 180 units high', async () => {
    const el = await render(0.7, 0.593);
    const [, , width, height] = (el.querySelector('svg')?.getAttribute('viewBox') ?? '').split(' ').map(Number);
    expect(width).toBe(640);
    expect(height).toBeGreaterThanOrEqual(170);
    expect(height).toBeLessThanOrEqual(190);
  });
});