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

  async function start(extraction = true) {
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    for (const r of http.match('/v1/limits')) r.flush({ limits: LIMITS });
    for (const r of http.match('/v1/pipeline')) r.flush({ extraction, law_authoring: true, vision: false, models: [] });
    fixture.detectChanges();
    await fixture.whenStable();
    return { http, fixture, el: fixture.nativeElement as HTMLElement };
  }

  it('shows the top bar with the wordmark, the links and the Ctrl+K hint, and no technical status', async () => {
    const { el } = await start();
    expect(el.querySelector('.wordmark')?.textContent).toContain('Limina');
    expect(el.querySelector('.status')).toBeNull();
    expect(el.querySelector('.topbar')?.textContent).not.toMatch(/engine|extraction|online|offline/i);
    expect(el.querySelector('.topbar-library')?.textContent).toContain('Library');
    expect(el.querySelector('.topbar-how')?.textContent).toContain('How it works');
    expect(el.querySelector('.kbd-hint')?.textContent).toContain('Ctrl');
    expect(el.querySelector('#wb-text')).not.toBeNull();
    expect(el.querySelector('#limit')).toBeNull();
  });

  it('says in plain words when free text cannot be checked', async () => {
    const { el } = await start(false);
    expect(el.querySelector('.notice')?.textContent).toContain('Free text cannot be checked right now');
    expect(el.textContent).not.toMatch(/(?<!heat )\b(extraction|engine)\b/i);
  });

  it('opens the Library drawer from the top bar and closes it with Escape', async () => {
    const { http, fixture, el } = await start();
    const drawer = el.querySelector('#drawer') as HTMLElement;
    expect(drawer.hidden).toBe(true);
    (el.querySelector('.topbar-library') as HTMLButtonElement).click();
    fixture.detectChanges();
    for (const r of http.match('/v1/limits')) r.flush({ limits: LIMITS });
    await fixture.whenStable();
    fixture.detectChanges();
    expect(drawer.hidden).toBe(false);
    expect(drawer.querySelectorAll('.limit')).toHaveLength(2);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    fixture.detectChanges();
    expect(drawer.hidden).toBe(true);
  });

  it('opens the command palette with Ctrl+K and runs a command', async () => {
    const { http, fixture } = await start();
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }));
    fixture.detectChanges();
    await fixture.whenStable();
    const palette = document.querySelector('.palette') as HTMLElement;
    expect(palette).not.toBeNull();
    const items = Array.from(palette.querySelectorAll('.palette-item')).map((i) => i.textContent?.trim());
    expect(items).toEqual(['Open Library', 'Open How it works']);
    const input = palette.querySelector('.palette-input') as HTMLInputElement;
    input.value = 'how';
    input.dispatchEvent(new Event('input'));
    fixture.detectChanges();
    input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    fixture.detectChanges();
    for (const r of http.match('/v1/limits')) r.flush({ limits: LIMITS });
    await fixture.whenStable();
    fixture.detectChanges();
    expect(document.querySelector('.palette')).toBeNull();
    const drawer = (fixture.nativeElement as HTMLElement).querySelector('#drawer') as HTMLElement;
    expect(drawer.hidden).toBe(false);
    expect(drawer.querySelector('#drawer-title')?.textContent).toContain('How it works');
  });

  it('shows plain messages in the Library and on the page when the service is down', async () => {
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    for (const r of http.match('/v1/limits')) r.error(new ProgressEvent('error'));
    for (const r of http.match('/v1/pipeline')) r.error(new ProgressEvent('error'));
    fixture.detectChanges();
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('.error-block')?.textContent).toContain('The library could not be loaded');
    expect(el.querySelector('.notice--error')?.textContent).toContain('Limina is unavailable right now');
    expect(el.textContent).not.toMatch(/NetworkError|API/);
  });
});
