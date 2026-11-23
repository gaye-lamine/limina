import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EvaluateForm } from './evaluate-form';
import { EvaluateRequest, EvaluateResponse } from './api.models';
import { LIMITS } from './testing-fixtures';

const enclosure = (b: number) => ({ b_min: b, b_max: b, margin: null });

function response(verdict: EvaluateResponse['verdict'], bound: number): EvaluateResponse {
  return {
    verdict,
    limit_id: 'betz',
    engine_version: '0.1.0',
    wording: `wording ${verdict}`,
    bounds: { permissive: enclosure(bound), restrictive: enclosure(bound) },
    configurations: [],
    missing_parameters: [],
  };
}

describe('Live updates', () => {
  let http: HttpTestingController;
  let el: HTMLElement;
  let settle: () => Promise<void>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EvaluateForm],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(EvaluateForm);
    fixture.componentRef.setInput('limits', LIMITS);
    el = fixture.nativeElement as HTMLElement;
    settle = () => fixture.whenStable();
    await settle();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
  });

  afterEach(() => {
    vi.useRealTimers();
    http.verify();
  });

  async function type(id: string, value: string) {
    const field = el.querySelector(`#${id}`) as HTMLInputElement;
    field.value = value;
    field.dispatchEvent(new Event('input'));
  }

  async function firstEvaluation() {
    await type('claim-value', '0.7');
    const select = el.querySelector('#assumption-reference_area') as HTMLSelectElement;
    select.value = 'unshrouded_rotor';
    select.dispatchEvent(new Event('change'));
    await vi.advanceTimersByTimeAsync(1000);
    http.expectNone('/v1/evaluate');
    (el.querySelector('form') as HTMLFormElement).dispatchEvent(new Event('submit'));
    http.expectOne('/v1/evaluate').flush(response('IMPOSSIBLE', 0.593));
    await vi.advanceTimersByTimeAsync(0);
  }

  it('does not evaluate on typing before the first evaluation', async () => {
    await type('claim-value', '0.5');
    await vi.advanceTimersByTimeAsync(2000);
    http.expectNone('/v1/evaluate');
  });

  it('re-evaluates after a debounce when the claim value changes, and updates verdict and chart', async () => {
    await firstEvaluation();
    expect(el.querySelector('.verdict--impossible')).not.toBeNull();
    expect(el.querySelector('.chart-label-claim')?.textContent).toContain('0.700');

    await type('claim-value', '0.4');
    await vi.advanceTimersByTimeAsync(100);
    http.expectNone('/v1/evaluate');
    await vi.advanceTimersByTimeAsync(1000);
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).claim.value).toBe(0.4);
    req.flush(response('COMPATIBLE', 0.593));
    await vi.advanceTimersByTimeAsync(0);
    expect(el.querySelector('.verdict--compatible')).not.toBeNull();
    expect(el.querySelector('.verdict--impossible')).toBeNull();
    expect(el.querySelector('.chart-label-claim')?.textContent).toContain('0.400');
  });

  it('sends one request for a burst of keystrokes', async () => {
    await firstEvaluation();
    for (const v of ['0', '0.', '0.3', '0.35']) {
      await type('claim-value', v);
      await vi.advanceTimersByTimeAsync(50);
    }
    await vi.advanceTimersByTimeAsync(1000);
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).claim.value).toBe(0.35);
    req.flush(response('COMPATIBLE', 0.593));
  });

  it('re-evaluates when an assumption changes', async () => {
    await firstEvaluation();
    const select = el.querySelector('#assumption-reference_area') as HTMLSelectElement;
    select.value = 'shrouded_rotor_area_only';
    select.dispatchEvent(new Event('change'));
    await vi.advanceTimersByTimeAsync(1000);
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).constraints).toEqual({
      reference_area: ['shrouded_rotor_area_only'],
    });
    req.flush(response('CONDITIONAL', 0.8));
    await vi.advanceTimersByTimeAsync(0);
    expect(el.querySelector('.verdict--conditional')).not.toBeNull();
  });

  it('stays silent when the claim becomes empty or invalid', async () => {
    await firstEvaluation();
    await type('claim-value', '');
    await vi.advanceTimersByTimeAsync(1000);
    http.expectNone('/v1/evaluate');
    expect(el.querySelector('.field-error')).toBeNull();
  });
});
