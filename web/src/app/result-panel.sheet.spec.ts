import { TestBed } from '@angular/core/testing';
import { ResultPanel } from './result-panel';
import { ApiFailure, EvaluateResponse, Verdict } from './api.models';
import { STRINGS } from './strings';

function response(verdict: Verdict, extra: Partial<EvaluateResponse> = {}): EvaluateResponse {
  return {
    verdict,
    limit_id: 'betz',
    engine_version: '0.1.0',
    wording: `wording for ${verdict}`,
    bounds: null,
    configurations: [],
    missing_parameters: [],
    ...extra,
  };
}

const BOUND = 16 / 27;
const SAME = { b_min: BOUND, b_max: BOUND, margin: { low: 1.18, high: 1.18 } };

async function render(
  result: EvaluateResponse | null,
  failure: ApiFailure | null = null,
  claim: { value: number; unit: string } | null = null,
): Promise<HTMLElement> {
  await TestBed.configureTestingModule({ imports: [ResultPanel] }).compileComponents();
  const fixture = TestBed.createComponent(ResultPanel);
  fixture.componentRef.setInput('result', result);
  fixture.componentRef.setInput('failure', failure);
  fixture.componentRef.setInput('claim', claim);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

describe('ResultPanel opening state', () => {
  it('shows the legend of the four verdicts, each in its own style with its meaning', async () => {
    const el = await render(null);
    const items = Array.from(el.querySelectorAll('.legend-item'));
    expect(items).toHaveLength(4);
    const expected: [string, Verdict][] = [
      ['verdict--impossible', 'IMPOSSIBLE'],
      ['verdict--conditional', 'CONDITIONAL'],
      ['verdict--compatible', 'COMPATIBLE'],
      ['verdict--insufficient-data', 'INSUFFICIENT_DATA'],
    ];
    expected.forEach(([cls, verdict], i) => {
      const label = items[i].querySelector('.legend-label') as HTMLElement;
      expect(label.classList.contains(cls)).toBe(true);
      expect(label.textContent?.trim()).toBe(STRINGS.verdictLabel[verdict]);
      expect(items[i].querySelector('.legend-meaning')?.textContent?.trim()).toBe(
        STRINGS.legendMeaning[verdict],
      );
    });
  });

  it('shows no error block and no verdict sheet', async () => {
    const el = await render(null);
    expect(el.querySelector('.error-block')).toBeNull();
    expect(el.querySelector('[role="alert"]')).toBeNull();
    expect(el.querySelector('.verdict')).toBeNull();
  });

  it('hides the legend once there is a result or a failure', async () => {
    const withResult = await render(response('COMPATIBLE'));
    expect(withResult.querySelector('.legend')).toBeNull();
    TestBed.resetTestingModule();
    const withFailure = await render(null, { status: 422, code: 'X', message: 'm' });
    expect(withFailure.querySelector('.legend')).toBeNull();
    expect(withFailure.querySelector('.error-block')).not.toBeNull();
  });
});

describe('ResultPanel verdict sheet', () => {
  const bounds = { permissive: SAME, restrictive: SAME };

  it('shows bound, claim and margin in tabular monospace under the wording', async () => {
    const el = await render(response('IMPOSSIBLE', { bounds }), null, { value: 0.7, unit: 'dimensionless' });
    const sheet = el.querySelector('.sheet') as HTMLElement;
    const values = Array.from(sheet.querySelectorAll('dd')).map((d) => d.textContent?.trim());
    expect(values[0]).toContain('0.593');
    expect(values[1]).toContain('0.700');
    expect(values[2]).toContain('1.18');
    expect(sheet.querySelectorAll('dd.mono')).toHaveLength(3);
    const order = Array.from(el.querySelectorAll('.verdict, .wording, .sheet')).map((n) => n.className);
    expect(order[0]).toContain('verdict');
    expect(order[1]).toContain('wording');
    expect(order[2]).toContain('sheet');
  });

  it('draws the chart with the claim and the bound', async () => {
    const el = await render(response('IMPOSSIBLE', { bounds }), null, { value: 0.7, unit: 'dimensionless' });
    expect(el.querySelector('app-bound-chart svg')).not.toBeNull();
    expect(el.querySelector('.chart-label-bound')?.textContent).toContain('0.593');
    expect(el.querySelector('.chart-label-claim')?.textContent).toContain('0.700');
  });

  it('draws the chart for a range using the smallest and the largest bound', async () => {
    const range = {
      permissive: { b_min: 0.4, b_max: 0.5, margin: null },
      restrictive: { b_min: 0.45, b_max: 0.5, margin: null },
    };
    const el = await render(response('CONDITIONAL', { bounds: range }), null, { value: 0.45, unit: 'dimensionless' });
    expect(el.querySelector('.chart-band')).not.toBeNull();
    expect(el.querySelector('.chart-label-bound')?.textContent).toContain('0.400 to 0.500');
  });

  it('draws no chart and lists the missing parameters when there is no bound', async () => {
    const el = await render(
      response('INSUFFICIENT_DATA', { missing_parameters: [{ id: 'Th', status: 'NoDomain' }] }),
      null,
      { value: 0.4, unit: 'dimensionless' },
    );
    expect(el.querySelector('app-bound-chart')).toBeNull();
    const note = el.querySelector('.no-bound')?.textContent ?? '';
    expect(note).toContain(STRINGS.noBound);
    expect(note).toContain('Th');
  });
});
