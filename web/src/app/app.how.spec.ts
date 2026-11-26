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
    const http = TestBed.inject(HttpTestingController);
    for (const r of http.match('/v1/limits')) r.flush({ limits: [] });
    for (const r of http.match('/v1/pipeline'))
      r.flush({ extraction: false, law_authoring: false, vision: false, models: [] });
    await fixture.whenStable();
    return fixture.nativeElement as HTMLElement;
  }

  it('explains that the engine decides and the language model only reads numbers', async () => {
    const el = await render();
    const how = el.querySelector('#how-title')?.closest('section') as HTMLElement;
    expect(how.querySelector('h2')?.textContent).toContain('How it works');
    const text = how.textContent ?? '';
    expect(text).toContain('exact calculation with an established law of physics');
    expect(text).toContain('never from an AI guess');
    expect(text).toContain('only used to find the numbers');
  });

  it('credits ONTO Limits as prior art, once, without any external link', async () => {
    const el = await render();
    expect((el.textContent ?? '').match(/ONTO Limits/g)).toHaveLength(1);
    expect(el.querySelector('a[href^="http"]')).toBeNull();
    expect(el.querySelector('img, link, script')).toBeNull();
  });
});
