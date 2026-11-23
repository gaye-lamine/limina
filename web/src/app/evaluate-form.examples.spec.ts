import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { EvaluateForm } from './evaluate-form';
import { EvaluateRequest, EvaluateResponse, Verdict } from './api.models';
import { COP_LIMIT, LIMITS } from './testing-fixtures';

function response(verdict: Verdict, limit_id: string): EvaluateResponse {
  return {
    verdict,
    limit_id,
    engine_version: '0.1.0',
    wording: `wording ${verdict}`,
    bounds: null,
    configurations: [],
    missing_parameters: [],
  };
}

describe('Examples', () => {
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

  afterEach(() => http.verify());

  const caseSelect = () => el.querySelector('#case') as HTMLSelectElement;

  /** Opens case `index` from the "Open a case" select. */
  async function open(index: number): Promise<void> {
    caseSelect().value = String(index);
    caseSelect().dispatchEvent(new Event('change'));
    await settle();
  }

  /** The COP requests: the unconstrained one first, then the heating and cooling twins. */
  function copRequests() {
    const reqs = http.match('/v1/evaluate');
    expect(reqs).toHaveLength(3);
    const modeOf = (r: (typeof reqs)[number]) => (r.request.body as EvaluateRequest).constraints['mode']?.[0] ?? '';
    reqs.sort((a, b) => modeOf(a).localeCompare(modeOf(b)));
    const [main, cooling, heating] = reqs;
    return [main, heating, cooling];
  }

  async function click(index: number): Promise<EvaluateRequest> {
    await open(index);
    const req = http.expectOne('/v1/evaluate');
    const body = req.request.body as EvaluateRequest;
    req.flush(response('IMPOSSIBLE', body.limit_id));
    await settle();
    return body;
  }

  it('offers the cases in one labelled "Open a case" select, and no example buttons', () => {
    expect(el.querySelector('label[for="case"]')?.textContent?.trim()).toBe('Open a case');
    const options = Array.from(caseSelect().options).map((o) => o.textContent?.trim());
    expect(options).toEqual([
      'Choose a case',
      'Betz, claim 0.70, unshrouded rotor',
      'Betz, claim 0.45, unshrouded rotor',
      'Carnot, claim 0.60, Th 600 K, Tc 300 K',
      'Carnot, claim 0.40, no temperatures',
      'COP, claim 14.5, Th 300 K, Tc 280 K',
    ]);
    expect(caseSelect().value).toBe('');
    expect(el.querySelector('.example-row')).toBeNull();
    expect(el.querySelector('button.chip')).toBeNull();
  });

  it('has one caption saying the cases are illustrative', () => {
    const notes = Array.from(el.querySelectorAll('.examples-note'));
    expect(notes).toHaveLength(1);
    expect(notes[0].textContent?.trim()).toBe('All examples are illustrative, not from real companies.');
    expect((el.textContent ?? '').match(/illustrative/g)).toHaveLength(1);
  });

  it('keeps the open case selected, and closes it when the user picks another limit', async () => {
    await click(2);
    expect(caseSelect().value).toBe('2');
    const limit = el.querySelector('#limit') as HTMLSelectElement;
    limit.value = 'betz';
    limit.dispatchEvent(new Event('change'));
    await settle();
    expect(caseSelect().value).toBe('');
  });

  it('opens the Carnot case on the bench apparatus', async () => {
    await click(2);
    expect(el.querySelector('app-carnot-apparatus')).not.toBeNull();
    expect(el.querySelector('label[for="claim-value"]')?.textContent?.trim()).toBe('Claimed efficiency');
    expect((el.querySelector('#param-Tc-range') as HTMLInputElement).value).toBe('300');
  });

  it('Betz 0.70 unshrouded rotor: fills the form and evaluates', async () => {
    const body = await click(0);
    expect(body).toEqual({
      limit_id: 'betz',
      claim: { value: 0.7, unit: 'dimensionless' },
      constraints: { reference_area: ['unshrouded_rotor'] },
      parameters: {},
    });
    expect((el.querySelector('#claim-value') as HTMLInputElement).value).toBe('0.70');
    expect((el.querySelector('#assumption-reference_area') as HTMLSelectElement).value).toBe(
      'unshrouded_rotor',
    );
    expect(el.querySelector('.verdict--impossible')).not.toBeNull();
  });

  it('Betz 0.45 unshrouded rotor', async () => {
    const body = await click(1);
    expect(body.claim.value).toBe(0.45);
    expect(body.constraints).toEqual({ reference_area: ['unshrouded_rotor'] });
  });

  it('Carnot 0.60 with Th = 600 K and Tc = 300 K, units from the metadata', async () => {
    const body = await click(2);
    expect(body).toEqual({
      limit_id: 'carnot',
      claim: { value: 0.6, unit: 'dimensionless' },
      constraints: {},
      parameters: { Th: { value: 600, unit: 'K' }, Tc: { value: 300, unit: 'K' } },
    });
    expect((el.querySelector('#param-Th-value') as HTMLInputElement).value).toBe('600');
  });

  it('Carnot 0.40 with no temperatures, even after another example filled them', async () => {
    await click(2);
    const body = await click(3);
    expect(body.parameters).toEqual({});
    expect(body.claim.value).toBe(0.4);
    expect((el.querySelector('#param-Th-value') as HTMLInputElement).value).toBe('');
  });

  it('COP 14.5 with Th = 300 K and Tc = 280 K: CONDITIONAL, dashed style, satisfying configuration listed', async () => {
    expect(caseSelect().options[5].textContent?.trim()).toBe('COP, claim 14.5, Th 300 K, Tc 280 K');
    await open(4);
    const [req, ...twins] = copRequests();
    for (const t of twins) t.flush(response('IMPOSSIBLE', 'cop'));
    expect(req.request.body).toEqual({
      limit_id: 'cop',
      claim: { value: 14.5, unit: 'dimensionless' },
      constraints: {},
      parameters: { Th: { value: 300, unit: 'K' }, Tc: { value: 280, unit: 'K' } },
    });
    const enc = { b_min: 14, b_max: 15, margin: { low: 14.5 / 15, high: 14.5 / 14 } };
    req.flush({
      ...response('CONDITIONAL', 'cop'),
      wording: 'possible only if mode = heating',
      bounds: { permissive: enc, restrictive: enc },
      configurations: [{ assumptions: { mode: 'heating' }, support: 'Certain', depends_on: [] }],
    });
    await settle();
    const verdict = el.querySelector('.verdict') as HTMLElement;
    expect(verdict.textContent?.trim()).toBe('CONDITIONAL');
    expect(verdict.classList.contains('verdict--conditional')).toBe(true);
    expect(el.querySelector('.wording')?.textContent?.trim()).toBe('possible only if mode = heating');
    const section = el.querySelector('.configurations') as HTMLElement;
    expect(section.querySelector('h3')?.textContent?.trim()).toBe('Satisfying configurations');
    const items = Array.from(section.querySelectorAll('.configuration')).map((i) => i.textContent ?? '');
    expect(items).toHaveLength(1);
    expect(items[0]).toContain('mode = heating');
    expect(items[0]).toContain('Certain');
    expect(el.querySelector('app-cop-apparatus')).not.toBeNull();
    expect(el.querySelector('app-bound-chart')).toBeNull();
    const ticks = Array.from(el.querySelectorAll('.gauge-bound-label')).map((t) => t.textContent?.trim());
    expect(ticks).toEqual(['bound 14.0', 'bound 15.0']);
  });

  it('lists the satisfying configurations right after the gauge and the numbers, before "Why this bound"', async () => {
    await open(4);
    const enc = { b_min: 14, b_max: 15, margin: null };
    const [main, ...twins] = copRequests();
    for (const t of twins) t.flush(response('IMPOSSIBLE', 'cop'));
    main.flush({
      ...response('CONDITIONAL', 'cop'),
      bounds: { permissive: enc, restrictive: enc },
      configurations: [{ assumptions: { mode: 'heating' }, support: 'Certain', depends_on: [] }],
    });
    await settle();
    const order = Array.from(el.querySelectorAll('app-gauge, .sheet, .configurations, .why')).map((n) =>
      n.tagName === 'APP-GAUGE' ? 'gauge' : n.className,
    );
    expect(order).toEqual(['gauge', 'sheet', 'configurations', 'why']);
  });

  it('shows the chosen claim in the result sheet', async () => {
    await click(0);
    expect(el.querySelector('.sheet')?.textContent).toContain('0.700');
  });
});
