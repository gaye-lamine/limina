import { TestBed } from '@angular/core/testing';
import { LimitsList } from './limits-list';
import { COP_LIMIT, LIMITS } from './testing-fixtures';
import { ApiFailure, LimitInfo } from './api.models';

async function render(limits: LimitInfo[], failure: ApiFailure | null = null): Promise<HTMLElement> {
  await TestBed.configureTestingModule({ imports: [LimitsList] }).compileComponents();
  const fixture = TestBed.createComponent(LimitsList);
  fixture.componentRef.setInput('limits', limits);
  fixture.componentRef.setInput('failure', failure);
  await fixture.whenStable();
  return fixture.nativeElement as HTMLElement;
}

describe('Library', () => {
  it('is titled "Library" and shows one card per limit', async () => {
    const el = await render([...LIMITS, COP_LIMIT]);
    expect(el.querySelector('#limits-title')?.textContent?.trim()).toBe('Library');
    expect(el.querySelectorAll('.library-card')).toHaveLength(3);
  });

  it('shows the title, formula and statement of each limit', async () => {
    const el = await render(LIMITS);
    const [betz, carnot] = Array.from(el.querySelectorAll('.library-card'));
    expect(betz.querySelector('h3')?.textContent?.trim()).toBe('Betz limit');
    expect(betz.querySelector('.library-formula')?.textContent?.trim()).toBe('Cp <= 16/27');
    expect(betz.querySelector('.library-statement')?.textContent?.trim()).toBe(LIMITS[0].statement);
    expect(carnot.querySelector('h3')?.textContent?.trim()).toBe('Carnot limit');
  });

  it('lists the assumptions and options with their display labels', async () => {
    const el = await render(LIMITS);
    const betz = el.querySelectorAll('.library-card')[0];
    expect(betz.querySelector('.library-assumption')?.textContent?.trim()).toBe('reference area');
    const options = Array.from(betz.querySelectorAll('.library-options li')).map((o) => o.textContent?.trim());
    expect(options).toEqual(['unshrouded rotor', 'shrouded, rotor area only']);
  });

  it('lists the parameters with their label and units', async () => {
    const el = await render(LIMITS);
    const params = Array.from(el.querySelectorAll('.library-card')[1].querySelectorAll('.library-parameter')).map(
      (p) => p.textContent?.replace(/\s+/g, ' ').trim(),
    );
    expect(params).toEqual([
      'Th, hot reservoir temperature, K, required',
      'Tc, cold reservoir temperature, K, required',
    ]);
  });

  it('says the reference is to be added, on every card', async () => {
    const el = await render([...LIMITS, COP_LIMIT]);
    const refs = Array.from(el.querySelectorAll('.library-reference')).map((r) => r.textContent?.trim());
    expect(refs).toEqual(['Reference: to be added', 'Reference: to be added', 'Reference: to be added']);
  });

  it('shows no raw id in the visible text', async () => {
    const el = await render([...LIMITS, COP_LIMIT]);
    const text = el.textContent ?? '';
    for (const id of [
      'betz',
      'carnot',
      'cop',
      'reference_area',
      'unshrouded_rotor',
      'shrouded_rotor_area_only',
      'cycle_model',
      'ideal_reversible_two_reservoirs',
      'AbsoluteTemperature',
    ]) {
      expect(text).not.toMatch(new RegExp(`\\b${id}\\b`));
    }
  });

  it('shows a bordered error block when loading failed', async () => {
    const el = await render([], { status: 0, code: 'NetworkError', message: 'unreachable' });
    expect(el.querySelector('.error-block')?.textContent).toContain('NetworkError');
  });
});
