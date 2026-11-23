import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, TestRequest, provideHttpClientTesting } from '@angular/common/http/testing';
import { EvaluateForm } from './evaluate-form';
import { EvaluateRequest, EvaluateResponse, Verdict } from './api.models';
import { COP_LIMIT, LIMITS } from './testing-fixtures';

function response(verdict: Verdict, extra: Partial<EvaluateResponse> = {}): EvaluateResponse {
  return {
    verdict,
    limit_id: 'cop',
    engine_version: '0.1.0',
    wording: `wording ${verdict}`,
    bounds: null,
    configurations: [],
    missing_parameters: [],
    ...extra,
  };
}

const modeOf = (r: TestRequest) => (r.request.body as EvaluateRequest).constraints['mode']?.[0] ?? '';

describe('EvaluateForm with the COP apparatus', () => {
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
    fixture.componentRef.setInput('limits', [...LIMITS, COP_LIMIT]);
    el = fixture.nativeElement as HTMLElement;
    settle = () => fixture.whenStable();
    await settle();
  });

  afterEach(() => {
    vi.useRealTimers();
    http.verify();
  });

  async function openCop(): Promise<TestRequest[]> {
    const select = el.querySelector('#case') as HTMLSelectElement;
    select.value = '4';
    select.dispatchEvent(new Event('change'));
    await settle();
    return http.match('/v1/evaluate');
  }

  it('with the mode not stated, asks the engine three times: unconstrained, heating and cooling', async () => {
    const reqs = await openCop();
    expect(reqs.map(modeOf).sort()).toEqual(['', 'cooling', 'heating']);
    for (const r of reqs) {
      const body = r.request.body as EvaluateRequest;
      expect(body.claim.value).toBe(14.5);
      expect(body.parameters).toEqual({ Th: { value: 300, unit: 'K' }, Tc: { value: 280, unit: 'K' } });
    }
    for (const r of reqs) r.flush(response('COMPATIBLE'));
  });

  it('shows each mode verdict from the engine under its figure, the sentence, and CONDITIONAL in the panel', async () => {
    const reqs = await openCop();
    const enc = { b_min: 14, b_max: 15, margin: null };
    for (const r of reqs) {
      const mode = modeOf(r);
      if (mode === 'heating') r.flush(response('COMPATIBLE'));
      else if (mode === 'cooling') r.flush(response('IMPOSSIBLE'));
      else
        r.flush(
          response('CONDITIONAL', {
            wording: 'possible only if mode = heating',
            bounds: { permissive: enc, restrictive: enc },
            configurations: [{ assumptions: { mode: 'heating' }, support: 'Certain', depends_on: [] }],
          }),
        );
    }
    await settle();
    const words = Array.from(el.querySelectorAll('.twin')).map((t) => t.querySelector('.twin-verdict')?.textContent?.trim());
    expect(words).toEqual(['COMPATIBLE', 'IMPOSSIBLE']);
    expect(el.querySelector('.twins-sentence')?.textContent?.trim()).toBe('Possible only in heating mode.');
    const verdict = el.querySelector('app-result-panel .verdict') as HTMLElement;
    expect(verdict.textContent?.trim()).toBe('CONDITIONAL');
    expect(verdict.classList.contains('verdict--conditional')).toBe(true);
    expect(el.querySelector('.configuration')?.textContent).toContain('mode = heating');
  });

  it('never shows a twin verdict the engine did not return', async () => {
    const reqs = await openCop();
    for (const r of reqs) {
      if (modeOf(r) === 'heating') r.error(new ProgressEvent('error'));
      else r.flush(response('IMPOSSIBLE'));
    }
    await settle();
    const words = Array.from(el.querySelectorAll('.twin')).map((t) => t.querySelector('.twin-verdict')?.textContent?.trim());
    expect(words).toEqual(['evaluating', 'IMPOSSIBLE']);
    expect(el.querySelector('.twins-sentence')).toBeNull();
  });

  it('sends the mode set on the figure as the constraint, and then asks only once', async () => {
    for (const r of await openCop()) r.flush(response('CONDITIONAL'));
    await settle();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    const heating = el.querySelector('#cop-mode-heating') as HTMLInputElement;
    heating.checked = true;
    heating.dispatchEvent(new Event('change'));
    await vi.advanceTimersByTimeAsync(300);
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).constraints).toEqual({ mode: ['heating'] });
    req.flush(response('COMPATIBLE'));
    vi.useRealTimers();
    await settle();
    expect(el.querySelector('.twins')).toBeNull();
    expect(el.querySelectorAll('app-cop-figure')).toHaveLength(1);
  });

  it('does not repeat the mode in the settings, since it is set on the figure', async () => {
    for (const r of await openCop()) r.flush(response('CONDITIONAL'));
    await settle();
    expect(el.querySelector('#assumption-mode')).toBeNull();
    expect(el.querySelector('label[for="claim-value"]')?.textContent?.trim()).toBe('Claimed COP');
  });
});
