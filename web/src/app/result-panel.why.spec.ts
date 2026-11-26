import { TestBed } from '@angular/core/testing';
import { ResultPanel, activeAssumptions } from './result-panel';
import { EvaluateResponse, LimitInfo, Verdict } from './api.models';
import { LIMITS, LIMIT_SEVERAL_UNITS } from './testing-fixtures';

const BETZ = LIMITS[0];
const CARNOT = LIMITS[1];
const BOUND = 16 / 27;

function response(verdict: Verdict, ratio: number | null, limit_id = 'betz'): EvaluateResponse {
  const enc = { b_min: BOUND, b_max: BOUND, margin: ratio === null ? null : { low: ratio, high: ratio } };
  return {
    verdict,
    limit_id,
    engine_version: '0.1.0',
    wording: `wording for ${verdict}`,
    bounds: { permissive: enc, restrictive: enc },
    configurations: [],
    missing_parameters: [],
  };
}

interface Inputs {
  result: EvaluateResponse | null;
  claim?: { value: number; unit: string } | null;
  limitInfo?: LimitInfo | null;
  constraints?: Record<string, string[]>;
}

async function render(inputs: Inputs): Promise<HTMLElement> {
  await TestBed.configureTestingModule({ imports: [ResultPanel] }).compileComponents();
  const fixture = TestBed.createComponent(ResultPanel);
  fixture.componentRef.setInput('result', inputs.result);
  fixture.componentRef.setInput('claim', inputs.claim ?? null);
  fixture.componentRef.setInput('limitInfo', inputs.limitInfo ?? null);
  fixture.componentRef.setInput('constraints', inputs.constraints ?? {});
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

const sheetValues = (el: HTMLElement) =>
  Array.from(el.querySelectorAll('.sheet dd')).map((d) => d.textContent?.trim() ?? '');
const sheetTerms = (el: HTMLElement) =>
  Array.from(el.querySelectorAll('.sheet dt')).map((d) => d.textContent?.trim() ?? '');

describe('ResultPanel sticky container', () => {
  it('carries the result-sticky class on its host, in every state', async () => {
    const empty = await render({ result: null });
    expect(empty.classList.contains('result-sticky')).toBe(true);
    TestBed.resetTestingModule();
    const filled = await render({ result: response('COMPATIBLE', 0.759375), claim: { value: 0.45, unit: 'dimensionless' } });
    expect(filled.classList.contains('result-sticky')).toBe(true);
  });
});

describe('ResultPanel numbers', () => {
  it('labels the ratio "Claim / bound" and reads it as "x the bound" below the bound', async () => {
    const el = await render({
      result: response('COMPATIBLE', 0.759375),
      claim: { value: 0.45, unit: 'dimensionless' },
      limitInfo: BETZ,
    });
    expect(sheetTerms(el)).toEqual(['Bound', 'Claim', 'Claim / bound']);
    expect(sheetValues(el)[2]).toBe('0.759 x the bound');
    expect(el.textContent ?? '').not.toMatch(/margin\s*0\.759/i);
  });

  it('reads "1.18 x the bound" when the claim exceeds the bound, in the sheet and the chart text', async () => {
    const el = await render({
      result: response('IMPOSSIBLE', 1.18125),
      claim: { value: 0.7, unit: 'dimensionless' },
      limitInfo: BETZ,
    });
    expect(sheetValues(el)[2]).toBe('1.18 x the bound');
    expect(el.querySelector('.chart-caption')?.textContent).toContain('1.18 x the bound');
  });

  it('hides the unit of a dimensionless quantity', async () => {
    const el = await render({
      result: response('COMPATIBLE', 0.759375),
      claim: { value: 0.45, unit: 'dimensionless' },
      limitInfo: BETZ,
    });
    expect(sheetValues(el).slice(0, 2)).toEqual(['0.593', '0.450']);
    expect(el.querySelector('.sheet')?.textContent).not.toContain('dimensionless');
    expect(el.querySelector('.chart')?.textContent).not.toContain('dimensionless');
  });

  it('shows any other unit', async () => {
    const el = await render({
      result: response('COMPATIBLE', 0.759375, 'power'),
      claim: { value: 0.45, unit: 'W' },
      limitInfo: LIMIT_SEVERAL_UNITS,
    });
    expect(sheetValues(el).slice(0, 2)).toEqual(['0.593 W', '0.450 W']);
  });

  it('names the bound on the chart with the limit title', async () => {
    const el = await render({
      result: response('COMPATIBLE', 0.759375),
      claim: { value: 0.45, unit: 'dimensionless' },
      limitInfo: BETZ,
    });
    expect(el.querySelector('.chart-label-bound')?.textContent?.trim()).toBe('Betz limit 0.593');
  });
});

describe('ResultPanel order', () => {
  it('shows the verdict, the wording, the gauge with its caption, the numbers, then "Why this bound"', async () => {
    const el = await render({
      result: response('IMPOSSIBLE', 1.18125),
      claim: { value: 0.7, unit: 'dimensionless' },
      limitInfo: BETZ,
    });
    const order = Array.from(el.querySelectorAll('.verdict, .wording, app-gauge, .sheet, .why')).map((n) =>
      n.tagName === 'APP-GAUGE' ? 'gauge' : (n.getAttribute('class') ?? '').split(' ')[0],
    );
    expect(order).toEqual(['verdict', 'wording', 'gauge', 'sheet', 'why']);
    expect(el.querySelector('app-gauge .fig-caption')).not.toBeNull();
  });
});

describe('ResultPanel "Why this bound"', () => {
  it('shows the title, formula, statement, active assumptions and the pending reference', async () => {
    const el = await render({
      result: response('IMPOSSIBLE', 1.18125),
      claim: { value: 0.7, unit: 'dimensionless' },
      limitInfo: BETZ,
      constraints: { reference_area: ['unshrouded_rotor'] },
    });
    const why = el.querySelector('.why') as HTMLElement;
    expect(why.querySelector('h3')?.textContent?.trim()).toBe('Why this bound');
    expect(why.querySelector('.why-title')?.textContent?.trim()).toBe('Betz limit');
    expect(why.querySelector('.why-formula')?.textContent?.trim()).toBe('Cp <= 16/27');
    expect(why.querySelector('.why-statement')?.textContent?.trim()).toBe(BETZ.statement);
    expect(why.querySelector('.why-assumptions')?.textContent).toContain('reference_area = unshrouded_rotor');
    expect(why.querySelector('.why-reference')?.textContent?.trim()).toBe('Reference: to be added');
  });

  it('sits under the chart', async () => {
    const el = await render({
      result: response('IMPOSSIBLE', 1.18125),
      claim: { value: 0.7, unit: 'dimensionless' },
      limitInfo: BETZ,
    });
    const order = Array.from(el.querySelectorAll('app-bound-chart, .why')).map((n) => n.tagName.toLowerCase());
    expect(order).toEqual(['app-bound-chart', 'section']);
  });

  it('is shown without a chart when no bound was computed', async () => {
    const result: EvaluateResponse = { ...response('INSUFFICIENT_DATA', null, 'carnot'), bounds: null };
    const el = await render({ result, claim: { value: 0.4, unit: 'dimensionless' }, limitInfo: CARNOT });
    expect(el.querySelector('app-bound-chart')).toBeNull();
    expect(el.querySelector('.why-formula')?.textContent?.trim()).toBe('efficiency <= 1 - Tc/Th');
  });

  it('is absent before any result', async () => {
    const el = await render({ result: null, limitInfo: BETZ });
    expect(el.querySelector('.why')).toBeNull();
  });
});

describe('activeAssumptions', () => {
  it('uses the chosen option, the only option, or "not specified"', () => {
    expect(activeAssumptions(BETZ, { reference_area: ['unshrouded_rotor'] })).toEqual([
      { id: 'reference_area', value: 'unshrouded_rotor' },
    ]);
    expect(activeAssumptions(BETZ, {})).toEqual([{ id: 'reference_area', value: 'not specified' }]);
    expect(activeAssumptions(CARNOT, {})).toEqual([
      { id: 'cycle_model', value: 'ideal_reversible_two_reservoirs' },
      { id: 'conversion_process', value: 'not specified' },
    ]);
  });
});
