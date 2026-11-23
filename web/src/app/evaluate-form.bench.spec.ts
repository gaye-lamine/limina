import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { EvaluateForm } from './evaluate-form';
import { EvaluateRequest, EvaluateResponse, Verdict } from './api.models';
import { LIMITS } from './testing-fixtures';

function response(verdict: Verdict, bound: number): EvaluateResponse {
  const enc = { b_min: bound, b_max: bound, margin: null };
  return {
    verdict,
    limit_id: 'carnot',
    engine_version: '0.1.0',
    wording: `wording ${verdict}`,
    bounds: { permissive: enc, restrictive: enc },
    configurations: [],
    missing_parameters: [],
  };
}

describe('EvaluateForm bench', () => {
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
  });

  afterEach(() => {
    vi.useRealTimers();
    http.verify();
  });

  async function openCarnot(): Promise<void> {
    const select = el.querySelector('#case') as HTMLSelectElement;
    select.value = '2';
    select.dispatchEvent(new Event('change'));
    await settle();
    http.expectOne('/v1/evaluate').flush(response('IMPOSSIBLE', 0.5));
    await settle();
  }

  it('puts the bench first and the instrument panel second', () => {
    const children = Array.from(el.children).map((c) => c.tagName.toLowerCase() + (c.className ? '.' + c.className : ''));
    expect(children[0]).toContain('bench');
    expect(children[1]).toContain('app-result-panel');
  });

  it('keeps the plain form and the linear chart for a limit without an apparatus', async () => {
    expect(el.querySelector('app-carnot-apparatus')).toBeNull();
    expect(el.querySelector('label[for="claim-value"]')?.textContent?.trim()).toBe('Claim value');
  });

  it('for Carnot shows the apparatus, the gauge as Fig. 2, and no linear chart', async () => {
    await openCarnot();
    expect(el.querySelector('app-carnot-apparatus')).not.toBeNull();
    expect(el.querySelector('app-gauge')).not.toBeNull();
    expect(el.querySelector('app-gauge .fig-caption')?.textContent).toMatch(/^Fig\. 2:/);
    expect(el.querySelector('app-bound-chart')).toBeNull();
    expect(el.querySelector('.verdict--impossible')).not.toBeNull();
    expect(el.querySelector('.why')).not.toBeNull();
  });

  for (const typed of ['0.60', '0,60']) {
    it(`reads the claimed efficiency "${typed}" as 0.6 and keeps the text as typed`, async () => {
      await openCarnot();
      const claim = el.querySelector('#claim-value') as HTMLInputElement;
      claim.value = typed;
      claim.dispatchEvent(new Event('input'));
      (el.querySelector('form') as HTMLFormElement).dispatchEvent(new Event('submit'));
      await settle();
      const req = http.expectOne('/v1/evaluate');
      expect((req.request.body as EvaluateRequest).claim.value).toBe(0.6);
      req.flush(response('IMPOSSIBLE', 0.5));
      await settle();
      expect((el.querySelector('#claim-value') as HTMLInputElement).value).toBe(typed);
    });
  }

  it('reads a temperature typed with a decimal comma', async () => {
    await openCarnot();
    const th = el.querySelector('#param-Th-value') as HTMLInputElement;
    th.value = '600,5';
    th.dispatchEvent(new Event('input'));
    (el.querySelector('form') as HTMLFormElement).dispatchEvent(new Event('submit'));
    await settle();
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).parameters['Th']).toEqual({ value: 600.5, unit: 'K' });
    req.flush(response('IMPOSSIBLE', 0.5));
    await settle();
    expect((el.querySelector('#param-Th-value') as HTMLInputElement).value).toBe('600,5');
  });

  it('re-evaluates after a debounce when a thermometer moves, and sends the new temperature', async () => {
    await openCarnot();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const range = el.querySelector('#param-Th-range') as HTMLInputElement;
    range.value = '900';
    range.dispatchEvent(new Event('input'));
    await vi.advanceTimersByTimeAsync(0);
    http.expectNone('/v1/evaluate');
    await vi.advanceTimersByTimeAsync(300);
    const req = http.expectOne('/v1/evaluate');
    const body = req.request.body as EvaluateRequest;
    expect(body.parameters).toEqual({ Th: { value: 900, unit: 'K' }, Tc: { value: 300, unit: 'K' } });
    req.flush(response('COMPATIBLE', 1 - 300 / 900));
    vi.useRealTimers();
    await settle();
    expect(el.querySelector('.verdict--compatible')).not.toBeNull();
  });
});
