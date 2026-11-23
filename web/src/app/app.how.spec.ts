import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { App } from './app';

describe('App "How it works"', () => {
  async function render(): Promise<HTMLElement> {
    await TestBed.configureTestingModule({
      imports: [App],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    TestBed.inject(HttpTestingController).expectOne('/v1/limits').flush({ limits: [] });
    await fixture.whenStable();
    return fixture.nativeElement as HTMLElement;
  }

  it('explains that the engine decides and the language model only reads numbers', async () => {
    const el = await render();
    const how = el.querySelector('#how-title')?.closest('section') as HTMLElement;
    expect(how.querySelector('h2')?.textContent).toContain('How it works');
    const text = how.textContent ?? '';
    expect(text).toContain('deterministic engine');
    expect(text).toContain('never by a language model');
    expect(text).toContain('only reads the numbers from the text');
  });

  it('credits ONTO Limits as prior art, once, without any external link', async () => {
    const el = await render();
    expect((el.textContent ?? '').match(/ONTO Limits/g)).toHaveLength(1);
    expect(el.querySelector('a[href^="http"]')).toBeNull();
    expect(el.querySelector('img, link, script')).toBeNull();
  });
});
