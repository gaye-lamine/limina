import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ILLUSTRATIVE_TEXT, Workbench } from './workbench';
import { LIMITS } from './testing-fixtures';
import { claimOf } from './workbench-logic.spec';

describe('Workbench', () => {
  async function setup() {
    await TestBed.configureTestingModule({
      imports: [Workbench],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(Workbench);
    await fixture.whenStable();
    http.expectOne('/v1/limits').flush({ limits: LIMITS });
    http.expectOne('/v1/pipeline').flush({ extraction: true, law_authoring: true, vision: true, models: ['a', 'b', 'v'] });
    await fixture.whenStable();
    return { http, fixture, el: fixture.nativeElement as HTMLElement };
  }

  async function settle(fixture: { whenStable(): Promise<unknown>; detectChanges(): void }) {
    for (let i = 0; i < 5; i++) {
      await new Promise((r) => setTimeout(r, 0));
      fixture.detectChanges();
      await fixture.whenStable();
    }
  }

  it('is pre-filled with a six-claim illustrative text', async () => {
    const { el } = await setup();
    const area = el.querySelector('#wb-text') as HTMLTextAreaElement;
    expect(area.value).toBe(ILLUSTRATIVE_TEXT);
    expect(ILLUSTRATIVE_TEXT.split('\n\n')).toHaveLength(6);
    expect(el.textContent).toContain('Illustrative text');
    expect(el.textContent).toContain('paste or drop an image');
  });

  it('analyses each paragraph, retries an unusable answer, reports a failure and shows the trace', async () => {
    const { http, fixture, el } = await setup();
    (el.querySelector('#wb-text') as HTMLTextAreaElement).value = 'COP 1\n\nTurbine 0.7';
    (el.querySelector('#wb-text') as HTMLTextAreaElement).dispatchEvent(new Event('input'));
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    // Paragraph 1: three unusable answers, then reported.
    for (let i = 0; i < 3; i++) {
      const r = http.expectOne('/v1/analyze');
      expect(r.request.body).toEqual({ text: 'COP 1' });
      r.flush({ error: { code: 'ExtractionFailed', message: 'extraction failed: the answer holds no JSON object' } }, { status: 502, statusText: 'Bad Gateway' });
      await settle(fixture);
    }
    // Paragraph 2: one claim with its trace.
    const claim = claimOf({
      evidence: { text: '0.7', start: 8, end: 11 },
      trace: [
        { step: 'extraction', status: 'done', detail: 'read efficiency: 60 %' },
        { step: 'limit', status: 'done', detail: 'built-in limit carnot' },
        { step: 'evaluation', status: 'done', detail: 'engine verdict IMPOSSIBLE' },
      ],
    });
    const r = http.expectOne('/v1/analyze');
    expect(r.request.body).toEqual({ text: 'Turbine 0.7' });
    r.flush({ claims: [claim], missing: [], extraction: { model: 'm', cached: false, ms: 1 } });
    await settle(fixture);
    for (const x of http.match('/v1/limits')) x.flush({ limits: LIMITS });
    await settle(fixture);
    const tabs = el.querySelectorAll('.wb-tab');
    expect(tabs).toHaveLength(2);
    expect(tabs[0].textContent).toContain('Paragraph not analysed');
    expect(tabs[1].textContent).toContain('IMPOSSIBLE');
    expect(el.querySelector('.wb-log')?.textContent).toContain('ExtractionFailed');
    (tabs[1] as HTMLButtonElement).click();
    await settle(fixture);
    const steps = Array.from(el.querySelectorAll('.wb-step')).map((s) => s.textContent ?? '');
    expect(steps).toHaveLength(3);
    expect(steps[2]).toContain('engine verdict IMPOSSIBLE');
    expect(el.querySelector('.verdict--impossible')?.textContent).toContain('IMPOSSIBLE');
    expect(el.querySelector('.wb-badge')?.textContent).toContain('built-in law');
    expect(el.querySelector('.wb-badge')?.textContent).toContain('theorem');
    expect(el.querySelector('.wording')?.textContent).toContain('inconsistent with limit carnot');
    expect(el.querySelector('.wb-explain')?.textContent).toContain('1.20 times what the law allows');
    // Offsets are shifted to the whole text: paragraph 2 starts at character 7.
    for (const s of http.match('/v1/solve')) {
      expect(s.request.body.free_parameter).toBe('Th');
      s.flush({ threshold: 750, direction: 'at least', unit: 'K', sentence: 'Not inconsistent with limit carnot only if Th is at least 750 K (other parameters fixed at Tc = 300 K)' });
    }
    await settle(fixture);
    expect(el.querySelector('.wb-explain')?.textContent).toContain('It would be consistent only if Th is at least 750 K');
    const curves = http.match('/v1/curve');
    expect(curves.length).toBeGreaterThan(0);
    expect(curves[0].request.body.limit_id).toBe('carnot');
  });
});
