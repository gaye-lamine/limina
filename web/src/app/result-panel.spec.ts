import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ResultPanel } from './result-panel';
import { ApiFailure, EvaluateResponse, Verdict } from './api.models';

function response(verdict: Verdict, extra: Partial<EvaluateResponse> = {}): EvaluateResponse {
  return {
    verdict,
    limit_id: 'carnot',
    engine_version: '0.1.0',
    wording: `wording for ${verdict}`,
    bounds: null,
    configurations: [],
    missing_parameters: [],
    ...extra,
  };
}

async function render(
  result: EvaluateResponse | null,
  failure: ApiFailure | null = null,
): Promise<HTMLElement> {
  await TestBed.configureTestingModule({ imports: [ResultPanel] }).compileComponents();
  const fixture: ComponentFixture<ResultPanel> = TestBed.createComponent(ResultPanel);
  fixture.componentRef.setInput('result', result);
  fixture.componentRef.setInput('failure', failure);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

const cases: [Verdict, string, string][] = [
  ['IMPOSSIBLE', 'IMPOSSIBLE', 'verdict--impossible'],
  ['CONDITIONAL', 'CONDITIONAL', 'verdict--conditional'],
  ['COMPATIBLE', 'COMPATIBLE', 'verdict--compatible'],
  ['INSUFFICIENT_DATA', 'INSUFFICIENT DATA', 'verdict--insufficient-data'],
];

describe('ResultPanel', () => {
  it.each(cases)('%s renders its text label and its own class only', async (verdict, label, cls) => {
    const el = await render(response(verdict));
    const badge = el.querySelector('.verdict') as HTMLElement;
    expect(badge.textContent?.trim()).toBe(label);
    expect(badge.classList.contains(cls)).toBe(true);
    const others = cases.map((c) => c[2]).filter((c) => c !== cls);
    for (const other of others) {
      expect(badge.classList.contains(other)).toBe(false);
    }
  });

  it('shows the wording exactly as received', async () => {
    const wording = 'inconsistent with limit carnot under assumptions x = y';
    const el = await render(response('IMPOSSIBLE', { wording }));
    expect(el.querySelector('.wording')?.textContent).toBe(wording);
  });

  it('shows the engine version and the limit id', async () => {
    const el = await render(response('COMPATIBLE', { engine_version: '9.9.9' }));
    expect(el.textContent).toContain('9.9.9');
    expect(el.textContent).toContain('carnot');
  });

  it('shows bounds and margin when present', async () => {
    const bounds = {
      permissive: { b_min: 0.5, b_max: 0.5, margin: { low: 1.2, high: 1.2 } },
      restrictive: { b_min: 0.5, b_max: 0.5, margin: null },
    };
    const el = await render(response('IMPOSSIBLE', { bounds }));
    const text = el.textContent ?? '';
    expect(text).toContain('0.5');
    expect(text).toContain('1.2');
  });

  it('omits the bounds block when there are no bounds', async () => {
    const el = await render(response('INSUFFICIENT_DATA'));
    expect(el.querySelector('.bounds')).toBeNull();
  });

  it('lists configurations with their Certain or Depends tag', async () => {
    const el = await render(
      response('CONDITIONAL', {
        configurations: [
          { assumptions: { a: 'x' }, support: 'Certain', depends_on: [] },
          { assumptions: { a: 'y' }, support: 'Depends', depends_on: ['Th'] },
        ],
      }),
    );
    const items = Array.from(el.querySelectorAll('.configuration')).map((i) => i.textContent ?? '');
    expect(items).toHaveLength(2);
    expect(items[0]).toContain('a = x');
    expect(items[0]).toContain('Certain');
    expect(items[1]).toContain('Depends');
    expect(items[1]).toContain('Th');
  });

  it('lists missing parameters with NoDomain or HasDomain', async () => {
    const el = await render(
      response('INSUFFICIENT_DATA', {
        missing_parameters: [
          { id: 'Th', status: 'NoDomain' },
          { id: 'Tc', status: 'HasDomain' },
        ],
      }),
    );
    const items = Array.from(el.querySelectorAll('.missing-parameter')).map((i) => i.textContent ?? '');
    expect(items[0]).toContain('Th');
    expect(items[0]).toContain('NoDomain');
    expect(items[1]).toContain('HasDomain');
  });

  it('shows the code and message in a bordered error block', async () => {
    const el = await render(null, { status: 422, code: 'InvalidTemperatureOrder', message: 'Th > Tc > 0' });
    const block = el.querySelector('.error-block') as HTMLElement;
    expect(block.textContent).toContain('InvalidTemperatureOrder');
    expect(block.textContent).toContain('Th > Tc > 0');
    expect(block.textContent).not.toContain('reference area');
    expect(el.querySelector('.verdict')).toBeNull();
  });

  it('tells the user to choose a reference area for NotApplicable', async () => {
    const el = await render(null, { status: 422, code: 'NotApplicable', message: 'not applicable' });
    expect(el.querySelector('.error-block')?.textContent).toContain('reference area');
  });

  it('is an aria-live region', async () => {
    const el = await render(null);
    expect(el.querySelector('[aria-live]')).not.toBeNull();
  });

  it('shows a placeholder when there is nothing to show', async () => {
    const el = await render(null);
    expect(el.textContent).toContain('No result yet.');
  });
});
