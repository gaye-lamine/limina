import { ComponentFixture, TestBed } from '@angular/core/testing';
import { GRADUATION_FONT, Thermometer } from './thermometer';

async function render(value: string): Promise<ComponentFixture<Thermometer>> {
  await TestBed.configureTestingModule({ imports: [Thermometer] }).compileComponents();
  const fixture = TestBed.createComponent(Thermometer);
  fixture.componentRef.setInput('paramId', 'Th');
  fixture.componentRef.setInput('subscript', 'h');
  fixture.componentRef.setInput('value', value);
  await fixture.whenStable();
  return fixture;
}

describe('Thermometer', () => {
  it('labels the field with the symbol, a real subscript', async () => {
    const el = (await render('600')).nativeElement as HTMLElement;
    const label = el.querySelector('label[for="param-Th-value"]') as HTMLElement;
    expect(label.querySelector('i')?.textContent).toBe('T');
    expect(label.querySelector('sub')?.textContent).toBe('h');
  });

  it('shows the value in the number field and the slider, and the unit as read-only text', async () => {
    const el = (await render('600')).nativeElement as HTMLElement;
    expect((el.querySelector('#param-Th-value') as HTMLInputElement).value).toBe('600');
    expect((el.querySelector('#param-Th-range') as HTMLInputElement).value).toBe('600');
    const unit = el.querySelector('#param-Th-unit') as HTMLInputElement;
    expect(unit.readOnly).toBe(true);
    expect(unit.value).toBe('K');
  });

  it('raises the liquid with the temperature and empties it without a value', async () => {
    const level = async (v: string) => {
      TestBed.resetTestingModule();
      const el = (await render(v)).nativeElement as HTMLElement;
      return Number(el.querySelector('rect.liquid')?.getAttribute('y'));
    };
    const low = await level('200');
    const high = await level('800');
    const empty = await level('');
    expect(high).toBeLessThan(low);
    expect(empty).toBeGreaterThan(low);
  });

  it('graduates the tube every 100 K and labels every 200 K', async () => {
    const el = (await render('600')).nativeElement as HTMLElement;
    expect(el.querySelectorAll('.graduation')).toHaveLength(11);
    const labels = Array.from(el.querySelectorAll('.graduation-label')).map((t) => t.textContent);
    expect(labels).toEqual(['0', '200', '400', '600', '800', '1000']);
  });

  it('draws its graduation labels at 1 px per unit, never below 12 px', async () => {
    const el = (await render('600')).nativeElement as HTMLElement;
    const svg = el.querySelector('svg') as SVGElement;
    expect(svg.getAttribute('viewBox')).toBe(`0 0 ${svg.getAttribute('width')} ${svg.getAttribute('height')}`);
    for (const t of Array.from(el.querySelectorAll('.graduation-label'))) {
      expect(Number(t.getAttribute('font-size'))).toBeGreaterThanOrEqual(12);
    }
    expect(GRADUATION_FONT).toBeGreaterThanOrEqual(12);
  });

  it('uses a decimal text field that accepts a comma and keeps the text as typed', async () => {
    TestBed.resetTestingModule();
    const el = (await render('300,5')).nativeElement as HTMLElement;
    const field = el.querySelector('#param-Th-value') as HTMLInputElement;
    expect(field.type).toBe('text');
    expect(field.getAttribute('inputmode')).toBe('decimal');
    expect(field.value).toBe('300,5');
    expect((el.querySelector('#param-Th-range') as HTMLInputElement).value).toBe('300.5');
  });

  it('emits from both the number field and the slider', async () => {
    const fixture = await render('600');
    const el = fixture.nativeElement as HTMLElement;
    const got: string[] = [];
    fixture.componentInstance.valueChange.subscribe((v) => got.push(v));
    const number = el.querySelector('#param-Th-value') as HTMLInputElement;
    number.value = '650';
    number.dispatchEvent(new Event('input'));
    const range = el.querySelector('#param-Th-range') as HTMLInputElement;
    range.value = '400';
    range.dispatchEvent(new Event('input'));
    expect(got).toEqual(['650', '400']);
  });
});
