import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { EvaluateForm } from './evaluate-form';
import { EvaluateRequest } from './api.models';
import { LIMITS } from './testing-fixtures';

const NOT_APPLICABLE = {
  error: { code: 'NotApplicable', message: 'limit betz is not applicable' },
};

describe('Guided recovery', () => {
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

  async function failWithNotApplicable() {
    const claim = el.querySelector('#claim-value') as HTMLInputElement;
    claim.value = '0.5';
    claim.dispatchEvent(new Event('input'));
    (el.querySelector('form') as HTMLFormElement).dispatchEvent(new Event('submit'));
    await settle();
    http
      .expectOne('/v1/evaluate')
      .flush(NOT_APPLICABLE, { status: 422, statusText: 'Unprocessable Entity' });
    await settle();
  }

  const buttons = () =>
    Array.from(el.querySelectorAll('.recovery button')) as HTMLButtonElement[];

  it('offers the two buttons on NotApplicable for Betz', async () => {
    await failWithNotApplicable();
    expect(buttons().map((b) => b.textContent?.trim())).toEqual([
      'Evaluate as unshrouded rotor',
      'Evaluate as shrouded, total capture area',
    ]);
  });

  it('sets the unshrouded rotor constraint and evaluates again', async () => {
    await failWithNotApplicable();
    buttons()[0].click();
    await settle();
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).constraints).toEqual({
      reference_area: ['unshrouded_rotor'],
    });
    req.flush({
      verdict: 'COMPATIBLE',
      limit_id: 'betz',
      engine_version: '0.1.0',
      wording: 'does not exceed the bound',
      bounds: null,
      configurations: [],
      missing_parameters: [],
    });
    await settle();
    expect(el.querySelector('.error-block')).toBeNull();
    expect(el.querySelector('.verdict--compatible')).not.toBeNull();
  });

  it('sets the shrouded total capture area constraint and evaluates again', async () => {
    await failWithNotApplicable();
    buttons()[1].click();
    await settle();
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).constraints).toEqual({
      reference_area: ['shrouded_total_capture_area'],
    });
    req.flush(NOT_APPLICABLE, { status: 422, statusText: 'Unprocessable Entity' });
  });

  it('offers no recovery button for another error code', async () => {
    const claim = el.querySelector('#claim-value') as HTMLInputElement;
    claim.value = '0.5';
    claim.dispatchEvent(new Event('input'));
    (el.querySelector('form') as HTMLFormElement).dispatchEvent(new Event('submit'));
    await settle();
    http
      .expectOne('/v1/evaluate')
      .flush(
        { error: { code: 'InvalidTemperatureOrder', message: 'Th > Tc' } },
        { status: 422, statusText: 'Unprocessable Entity' },
      );
    await settle();
    expect(buttons()).toHaveLength(0);
  });
});
