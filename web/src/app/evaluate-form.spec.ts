import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { EvaluateForm } from './evaluate-form';
import { EvaluateRequest, EvaluateResponse } from './api.models';
import { LIMITS } from './testing-fixtures';

const IMPOSSIBLE: EvaluateResponse = {
  verdict: 'IMPOSSIBLE',
  limit_id: 'betz',
  engine_version: '0.1.0',
  wording: 'inconsistent with limit betz under assumptions reference_area = unshrouded_rotor',
  bounds: null,
  configurations: [],
  missing_parameters: [],
};

describe('EvaluateForm', () => {
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

  afterEach(() => http.verify());

  const input = (id: string) => el.querySelector(`#${id}`) as HTMLInputElement;
  const select = (id: string) => el.querySelector(`#${id}`) as HTMLSelectElement;

  async function type(id: string, value: string) {
    const field = input(id);
    field.value = value;
    field.dispatchEvent(new Event('input'));
    await settle();
  }

  async function choose(id: string, value: string) {
    const field = select(id);
    field.value = value;
    field.dispatchEvent(new Event('change'));
    await settle();
  }

  async function submit() {
    (el.querySelector('form') as HTMLFormElement).dispatchEvent(new Event('submit'));
    await settle();
  }

  it('shows one selector per assumption for Betz and no parameter fields', () => {
    expect(select('assumption-reference_area')).not.toBeNull();
    const options = Array.from(select('assumption-reference_area').options).map((o) => o.value);
    expect(options).toEqual(['', 'unshrouded_rotor', 'shrouded_rotor_area_only']);
    expect(el.querySelector('#param-Th-value')).toBeNull();
  });

  it('adapts to Carnot: parameter fields with K prefilled, no assumption selector', async () => {
    await choose('limit', 'carnot');
    expect(select('assumption-reference_area')).toBeNull();
    expect(input('param-Th-value')).not.toBeNull();
    expect(input('param-Th-unit').value).toBe('K');
    expect(el.querySelector('#claim-unit')).toBeNull();
  });

  it('sends the claim, the chosen constraint and no parameters for Betz', async () => {
    await type('claim-value', '0.7');
    await choose('assumption-reference_area', 'unshrouded_rotor');
    await submit();
    const req = http.expectOne('/v1/evaluate');
    const body = req.request.body as EvaluateRequest;
    expect(body).toEqual({
      limit_id: 'betz',
      claim: { value: 0.7, unit: 'dimensionless' },
      constraints: { reference_area: ['unshrouded_rotor'] },
      parameters: {},
    });
    req.flush(IMPOSSIBLE);
    await settle();
    expect(el.querySelector('.verdict--impossible')?.textContent).toContain('IMPOSSIBLE');
    expect(el.querySelector('.wording')?.textContent).toBe(IMPOSSIBLE.wording);
  });

  it('omits an unspecified assumption, the uncertainty and empty parameters', async () => {
    await choose('limit', 'carnot');
    await type('claim-value', '0.6');
    await type('param-Th-value', '600');
    await submit();
    const req = http.expectOne('/v1/evaluate');
    expect(req.request.body).toEqual({
      limit_id: 'carnot',
      claim: { value: 0.6, unit: 'dimensionless' },
      constraints: {},
      parameters: { Th: { value: 600, unit: 'K' } },
    });
    req.flush({ ...IMPOSSIBLE, verdict: 'INSUFFICIENT_DATA', missing_parameters: [{ id: 'Tc', status: 'NoDomain' }] });
    await settle();
    expect(el.querySelector('.verdict--insufficient-data')).not.toBeNull();
  });

  it('sends the uncertainty when given', async () => {
    await type('claim-value', '0.3');
    await type('claim-uncertainty', '0.01');
    await submit();
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).claim).toEqual({
      value: 0.3,
      unit: 'dimensionless',
      uncertainty: 0.01,
    });
    req.flush(IMPOSSIBLE);
  });

  it('does not call the API when the claim value is empty or not a number', async () => {
    await submit();
    expect(el.querySelector('.field-error')?.textContent).toContain('required');
    await type('claim-value', 'abc');
    await submit();
    expect(el.querySelector('.field-error')?.textContent).toContain('finite number');
    http.expectNone('/v1/evaluate');
  });

  it('shows the NotApplicable error with the reference area hint', async () => {
    await type('claim-value', '0.5');
    await submit();
    http
      .expectOne('/v1/evaluate')
      .flush(
        { error: { code: 'NotApplicable', message: 'limit betz is not applicable' } },
        { status: 422, statusText: 'Unprocessable Entity' },
      );
    await settle();
    const block = el.querySelector('.error-block') as HTMLElement;
    expect(block.textContent).toContain('NotApplicable');
    expect(block.textContent).toContain('reference area');
  });
});

