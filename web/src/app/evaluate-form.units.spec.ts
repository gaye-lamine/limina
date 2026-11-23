import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { EvaluateForm } from './evaluate-form';
import { EvaluateRequest } from './api.models';
import { LIMIT_SEVERAL_UNITS, LIMITS } from './testing-fixtures';

describe('EvaluateForm units come from the metadata', () => {
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
    fixture.componentRef.setInput('limits', [...LIMITS, LIMIT_SEVERAL_UNITS]);
    el = fixture.nativeElement as HTMLElement;
    settle = () => fixture.whenStable();
    await settle();
  });

  async function choose(id: string, value: string) {
    const field = el.querySelector(`#${id}`) as HTMLSelectElement;
    field.value = value;
    field.dispatchEvent(new Event('change'));
    await settle();
  }

  it('hides the claim unit when the quantity is dimensionless', async () => {
    expect(el.querySelector('#claim-unit')).toBeNull();
    expect(el.textContent ?? '').not.toContain('dimensionless');
    await choose('limit', 'carnot');
    expect(el.querySelector('#claim-unit')).toBeNull();
  });

  it('shows the claim unit of the limit as read-only text when it has one', async () => {
    await choose('limit', 'power');
    const power = el.querySelector('#claim-unit') as HTMLInputElement;
    expect(power.readOnly).toBe(true);
    expect(power.value).toBe('W');
  });

  it('shows a single parameter unit as read-only text, not a select', async () => {
    await choose('limit', 'carnot');
    const unit = el.querySelector('#param-Th-unit') as HTMLInputElement;
    expect(unit.tagName).toBe('INPUT');
    expect(unit.readOnly).toBe(true);
    expect(unit.value).toBe('K');
  });

  it('offers a select when a parameter allows several units, and sends the chosen one', async () => {
    await choose('limit', 'power');
    const unit = el.querySelector('#param-P-unit') as HTMLSelectElement;
    expect(unit.tagName).toBe('SELECT');
    expect(Array.from(unit.options).map((o) => o.value)).toEqual(['W', 'kW', 'MW']);
    const claim = el.querySelector('#claim-value') as HTMLInputElement;
    claim.value = '3';
    claim.dispatchEvent(new Event('input'));
    const value = el.querySelector('#param-P-value') as HTMLInputElement;
    value.value = '2';
    value.dispatchEvent(new Event('input'));
    await choose('param-P-unit', 'kW');
    (el.querySelector('form') as HTMLFormElement).dispatchEvent(new Event('submit'));
    await settle();
    const req = http.expectOne('/v1/evaluate');
    expect((req.request.body as EvaluateRequest).claim.unit).toBe('W');
    expect((req.request.body as EvaluateRequest).parameters).toEqual({ P: { value: 2, unit: 'kW' } });
    req.flush({}, { status: 500, statusText: 'x' });
  });
});
