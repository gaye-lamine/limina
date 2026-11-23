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

  it('loads the limits once and renders the Evaluate page and the Limits section', async () => {
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    http.expectOne('/v1/limits').flush({ limits: LIMITS });
    await fixture.whenStable();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('h1')?.textContent).toContain('Limina');
    expect(el.querySelector('#evaluate-title')?.textContent).toContain('Evaluate');
    expect(el.querySelector('#limits-title')?.textContent).toContain('Library');
    expect(el.querySelectorAll('.limit')).toHaveLength(2);
    expect(el.querySelector('#assumption-reference_area')).not.toBeNull();
    http.verify();
  });

  it('shows the error block in the Limits section when loading fails', async () => {
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(App);
    await fixture.whenStable();
    http.expectOne('/v1/limits').error(new ProgressEvent('error'));
    await fixture.whenStable();
    expect((fixture.nativeElement as HTMLElement).querySelector('.error-block')?.textContent).toContain(
      'NetworkError',
    );
  });
});
