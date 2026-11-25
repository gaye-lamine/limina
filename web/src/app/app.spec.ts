import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { App } from './app';
import { LIMITS } from './testing-fixtures';

describe('App', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [App],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
  });

  function flushStart(http: HttpTestingController): void {
    for (const r of http.match('/v1/limits')) r.flush({ limits: LIMITS });
    for (const r of http.match('/v1/pipeline'))
      r.flush({ extraction: true, law_authoring: true, vision: false, models: [] });
  }

  it('shows the analysis screen with no example select, no limit select and no Evaluate button', async () => {
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    flushStart(http);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('h1')?.textContent).toContain('Limina');
    expect(el.querySelector('#wb-text')).not.toBeNull();
    expect(el.querySelector('.wb-analyze')?.textContent).toContain('Analyze');
    expect(el.textContent).toContain('Illustrative text');
    expect(el.querySelector('#limit')).toBeNull();
    expect(el.querySelector('#evaluate-title')).toBeNull();
    expect(el.querySelector('#assumption-reference_area')).toBeNull();
    http.verify();
  });

  it('keeps the Library and How it works in a drawer, opened from the top bar and closed by Escape', async () => {
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    flushStart(http);
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    const drawer = el.querySelector('#drawer') as HTMLElement;
    expect(drawer.hidden).toBe(true);
    (el.querySelector('.topbar-button') as HTMLButtonElement).click();
    fixture.detectChanges();
    for (const r of http.match('/v1/limits')) r.flush({ limits: LIMITS });
    await fixture.whenStable();
    expect(drawer.hidden).toBe(false);
    expect(drawer.querySelector('#limits-title')?.textContent).toContain('Library');
    expect(drawer.querySelectorAll('.limit')).toHaveLength(2);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    fixture.detectChanges();
    expect(drawer.hidden).toBe(true);
  });

  it('shows the error block in the Library when loading fails', async () => {
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    for (const r of http.match('/v1/limits')) r.error(new ProgressEvent('error'));
    for (const r of http.match('/v1/pipeline')) r.error(new ProgressEvent('error'));
    await fixture.whenStable();
    expect((fixture.nativeElement as HTMLElement).querySelector('.error-block')?.textContent).toContain(
      'NetworkError',
    );
  });
});
