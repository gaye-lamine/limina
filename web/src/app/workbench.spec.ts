import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ILLUSTRATIVE_TEXT, Workbench } from './workbench';
import { LIMITS } from './testing-fixtures';
import { claimOf } from './workbench-logic.spec';
import { AnalyzedClaim } from './analysis.models';

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

  function start(el: HTMLElement, text: string) {
    const area = el.querySelector('#wb-text') as HTMLTextAreaElement;
    area.value = text;
    area.dispatchEvent(new Event('input'));
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
  }

  function answer(http: HttpTestingController, claims: AnalyzedClaim[]) {
    http.expectOne('/v1/analyze').flush({ claims, missing: [], extraction: { model: 'm', cached: false, ms: 1 } });
  }

  function drain(http: HttpTestingController) {
    for (const x of http.match('/v1/limits')) x.flush({ limits: LIMITS });
    for (const x of http.match('/v1/solve')) x.flush({ error: { code: 'NoSolution', message: 'none' } }, { status: 422, statusText: 'x' });
    for (const x of http.match('/v1/curve')) x.flush({ error: { code: 'x', message: 'x' } }, { status: 422, statusText: 'x' });
    for (const x of http.match('/v1/surface')) x.flush({ error: { code: 'x', message: 'x' } }, { status: 422, statusText: 'x' });
  }

  const result = (verdict: 'IMPOSSIBLE' | 'CONDITIONAL' | 'COMPATIBLE') => ({
    ...claimOf().result!,
    verdict,
    wording: verdict === 'CONDITIONAL' ? 'possible only if mode = heating' : 'inconsistent with limit carnot under assumptions none',
  });

  /** Three paragraphs: IMPOSSIBLE, CONDITIONAL and COMPATIBLE, then a claim with no law. */
  async function reviewed() {
    const ctx = await setup();
    const { http, fixture, el } = ctx;
    start(el, 'Engine 60% at 600 K\n\nPump 4 at 300 K\n\nCell 5 kWh in 1 kg');
    await settle(fixture);
    answer(http, [
      claimOf({
        evidence: { text: '60%', start: 7, end: 10 },
        parameters: { Th: { value: 600, raw_value: 600, unit: 'K', evidence: { text: '600 K', start: 14, end: 19 }, flags: [] } },
      }),
    ]);
    await settle(fixture);
    expect(el.querySelector('.rv-scan')).not.toBeNull();
    expect(el.querySelector('.rv-ticker')?.textContent).toContain('Paragraph 2 of 3');
    answer(http, [
      claimOf({ evidence: { text: '4', start: 5, end: 6 }, parameters: {}, result: result('CONDITIONAL') }),
      claimOf({ evidence: { text: '300 K', start: 10, end: 15 }, parameters: {}, result: result('COMPATIBLE') }),
    ]);
    await settle(fixture);
    answer(http, [
      claimOf({ limit_id: null, status: 'needs_input', result: null, reason: 'no established law identified for this quantity', evidence: { text: 'Cell 5 kWh in 1 kg', start: 0, end: 18 }, parameters: {} }),
    ]);
    await settle(fixture);
    drain(http);
    await settle(fixture);
    drain(http);
    await settle(fixture);
    return ctx;
  }

  it('starts as a large pre-filled text box with the tagline', async () => {
    const { el } = await setup();
    const area = el.querySelector('#wb-text') as HTMLTextAreaElement;
    expect(area.value).toBe(ILLUSTRATIVE_TEXT);
    expect(ILLUSTRATIVE_TEXT.split('\n\n')).toHaveLength(6);
    expect(el.querySelector('.rv-tagline')?.textContent).toContain('Paste a technical text. Each physical claim is checked against physical laws, with sources.');
    expect(el.textContent).toContain('Illustrative text');
    expect(el.textContent).toContain('paste or drop an image');
    expect(el.querySelector('.rv-doc')).toBeNull();
  });

  it('stamps each claim in the margin in the style of its verdict, with a plain sentence', async () => {
    const { el } = await reviewed();
    const stamps = Array.from(el.querySelectorAll<HTMLElement>('.rv-doc .rv-stamp'));
    expect(stamps.map((s) => s.textContent?.trim())).toEqual(['IMPOSSIBLE', 'CONDITIONAL', 'COMPATIBLE', 'NEEDS INPUT']);
    expect(stamps.map((s) => s.className.match(/stamp--(\w+)/)?.[1])).toEqual(['impossible', 'conditional', 'compatible', 'neutral']);
    for (const s of stamps) {
      const deg = parseFloat(s.style.getPropertyValue('--rot'));
      expect(deg).toBeGreaterThanOrEqual(-3);
      expect(deg).toBeLessThanOrEqual(2);
    }
    const notes = Array.from(el.querySelectorAll('.rv-stamp-note')).map((n) => n.textContent?.trim());
    expect(notes[0]).toBe('1.20 x what the law allows');
    expect(notes[1]).toBe('possible only if mode = heating');
    expect(notes[3]).toBe('no established law identified for this quantity');
    // Stamps sit in the paragraph of their claim; numbers inside the evidence are underlined.
    const paras = el.querySelectorAll('.rv-para');
    expect(paras).toHaveLength(3);
    expect(paras[1].querySelectorAll('.rv-stamp')).toHaveLength(2);
    expect(Array.from(paras[2].querySelectorAll('.rv-num')).map((n) => n.textContent)).toEqual(['5', '1']);
    expect(Array.from(paras[0].querySelectorAll('.rv-num')).map((n) => n.textContent)).toEqual(['60', '600']);
  });

  it('shows the scoreboard and highlights the claims of a count', async () => {
    const { fixture, el } = await reviewed();
    expect(el.querySelector('.rv-board-total')?.textContent?.trim()).toBe('3 paragraphs, 4 claims');
    const counts = Array.from(el.querySelectorAll<HTMLButtonElement>('.rv-count'));
    expect(counts.map((c) => c.textContent?.replace(/\s+/g, ' ').trim())).toEqual(['1 IMPOSSIBLE', '1 CONDITIONAL', '1 COMPATIBLE', '1 NEEDS INPUT']);
    expect(counts[1].className).toContain('stamp--conditional');
    counts[1].click();
    await settle(fixture);
    expect(counts[1].getAttribute('aria-pressed')).toBe('true');
    const marks = Array.from(el.querySelectorAll('.rv-mark'));
    expect(marks.map((m) => m.classList.contains('is-lit'))).toEqual([false, true, false, false]);
    expect(marks[0].classList.contains('is-dim')).toBe(true);
    expect(el.querySelector('.rv-inst-stamp')?.textContent?.trim()).toBe('CONDITIONAL');
  });

  it('moves between claims with the arrow keys and marks the selected one in the document', async () => {
    const { fixture, el } = await reviewed();
    const pressed = () => Array.from(el.querySelectorAll('.rv-stamp')).map((s) => s.getAttribute('aria-pressed'));
    expect(pressed()).toEqual(['true', 'false', 'false', 'false']);
    expect(el.querySelector('.rv-para.is-selected')).toBe(el.querySelectorAll('.rv-para')[0]);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }));
    await settle(fixture);
    expect(pressed()).toEqual(['false', 'true', 'false', 'false']);
    expect(el.querySelector('.rv-para.is-selected')).toBe(el.querySelectorAll('.rv-para')[1]);
    expect(el.querySelector('.rv-inst-stamp')?.textContent?.trim()).toBe('CONDITIONAL');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp' }));
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowUp' }));
    await settle(fixture);
    expect(pressed()).toEqual(['false', 'false', 'false', 'true']);
    (el.querySelectorAll<HTMLButtonElement>('.rv-stamp')[2]).click();
    await settle(fixture);
    expect(el.querySelector('.rv-inst-stamp')?.textContent?.trim()).toBe('COMPATIBLE');
  });

  it('shows the source of the law in the instrument panel', async () => {
    const { el } = await reviewed();
    const source = el.querySelector('.rv-source')?.textContent ?? '';
    expect(source).toContain('built-in law');
    expect(el.querySelector('.wording')?.textContent).toContain('inconsistent with limit carnot');
    expect(el.querySelector('.rv-plain')?.textContent).toContain('1.20 x what the law allows');
  });

  it('keeps the pipeline trace in a drawer closed by Escape', async () => {
    const { fixture, el } = await reviewed();
    const drawer = el.querySelector('#rv-how') as HTMLElement;
    expect(drawer.hidden).toBe(true);
    (el.querySelector('.rv-how') as HTMLButtonElement).click();
    await settle(fixture);
    expect(drawer.hidden).toBe(false);
    expect(drawer.textContent).toContain('Paragraph 3 of 3: 1 claim(s)');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await settle(fixture);
    expect(drawer.hidden).toBe(true);
  });

  it('retries an unusable answer and stamps a paragraph that could not be analysed', async () => {
    const { http, fixture, el } = await setup();
    start(el, 'COP 1');
    await settle(fixture);
    for (let i = 0; i < 3; i++) {
      const r = http.expectOne('/v1/analyze');
      expect(r.request.body).toEqual({ text: 'COP 1' });
      r.flush({ error: { code: 'ExtractionFailed', message: 'extraction failed: the answer holds no JSON object' } }, { status: 502, statusText: 'Bad Gateway' });
      await settle(fixture);
    }
    drain(http);
    await settle(fixture);
    expect(el.querySelector('.rv-stamp')?.textContent?.trim()).toBe('NOT ANALYSED');
    expect(el.querySelector('.rv-stamp')?.className).toContain('stamp--neutral');
    expect(el.querySelector('.error-block')?.textContent).toContain('ExtractionFailed');
  });

  it('downloads the review as Markdown', async () => {
    const { el } = await reviewed();
    let blob: Blob | null = null;
    const create = URL.createObjectURL;
    const revoke = URL.revokeObjectURL;
    URL.createObjectURL = (b: Blob) => {
      blob = b;
      return 'blob:x';
    };
    URL.revokeObjectURL = () => undefined;
    const click = HTMLAnchorElement.prototype.click;
    let name = '';
    HTMLAnchorElement.prototype.click = function (this: HTMLAnchorElement) {
      name = this.download;
    };
    try {
      (el.querySelector('.rv-download') as HTMLButtonElement).click();
    } finally {
      HTMLAnchorElement.prototype.click = click;
      URL.createObjectURL = create;
      URL.revokeObjectURL = revoke;
    }
    expect(name).toBe('limina-review.md');
    const md = await (blob as unknown as Blob).text();
    expect(md).toContain('# Limina review');
    expect(md).toContain('3 paragraphs, 4 claims.');
    expect(md).toContain('Generated by Limina; AI-written laws are not validated');
  });
});
