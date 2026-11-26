import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ILLUSTRATIVE_TEXT, Workbench } from './workbench';
import { LIMITS } from './testing-fixtures';
import { claimOf } from './workbench-logic.spec';
import { AnalyzedClaim } from './analysis.models';
import { ToastService } from './toast.service';
import { SAMPLES } from './samples';

interface Fixture {
  whenStable(): Promise<unknown>;
  detectChanges(): void;
}

describe('Workbench', { timeout: 30_000 }, () => {
  async function setup(pipeline = { extraction: true, law_authoring: true, vision: true, models: ['a'] }) {
    await TestBed.configureTestingModule({
      imports: [Workbench],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(Workbench);
    await fixture.whenStable();
    for (const r of http.match('/v1/limits')) r.flush({ limits: LIMITS });
    for (const r of http.match('/v1/pipeline')) r.flush(pipeline);
    await settle(fixture);
    return { http, fixture, el: fixture.nativeElement as HTMLElement };
  }

  async function settle(fixture: Fixture) {
    for (let i = 0; i < 4; i++) {
      await new Promise((r) => setTimeout(r, 0));
      fixture.detectChanges();
      await fixture.whenStable();
    }
  }

  /** Answers every pending side request: limits, solve, curve, surface, evaluate of the COP twins. */
  async function drain(http: HttpTestingController, fixture: Fixture) {
    for (let k = 0; k < 4; k++) {
      for (const x of http.match('/v1/limits')) x.flush({ limits: LIMITS });
      for (const x of http.match('/v1/solve')) x.flush({ error: { code: 'NoSolution', message: 'none' } }, { status: 422, statusText: 'x' });
      for (const x of http.match('/v1/curve')) x.flush({ error: { code: 'x', message: 'x' } }, { status: 422, statusText: 'x' });
      for (const x of http.match('/v1/surface')) x.flush({ error: { code: 'x', message: 'x' } }, { status: 422, statusText: 'x' });
      await settle(fixture);
    }
  }

  function type(el: HTMLElement, text: string) {
    const area = el.querySelector('#wb-text') as HTMLTextAreaElement;
    area.value = text;
    area.dispatchEvent(new Event('input'));
  }

  function answer(http: HttpTestingController, claims: AnalyzedClaim[]) {
    http.expectOne('/v1/analyze').flush({ claims, missing: [], extraction: { model: 'm', cached: false, ms: 1 } });
  }

  const result = (verdict: 'IMPOSSIBLE' | 'CONDITIONAL' | 'COMPATIBLE') => ({
    ...claimOf().result!,
    verdict,
    wording: verdict === 'CONDITIONAL' ? 'possible only if mode = heating' : 'inconsistent with limit carnot under assumptions none',
  });

  /** Three paragraphs analysed by the model: IMPOSSIBLE; CONDITIONAL and COMPATIBLE; no law. */
  async function reviewed() {
    const ctx = await setup();
    const { http, fixture, el } = ctx;
    type(el, 'Engine 60% at 600 K\n\nPump 4 at 300 K\n\nCell 5 kWh in 1 kg');
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    answer(http, [claimOf({ evidence: { text: '60%', start: 7, end: 10 } })]);
    await settle(fixture);
    // Running: compact bar, stepper on Reading, the first card, skeletons for the rest.
    expect(el.getAttribute('data-state')).toBe('running');
    expect(el.querySelector('.step--active')?.textContent).toContain('Reading');
    expect(el.querySelectorAll('.stream-card:not(.skeleton)')).toHaveLength(1);
    expect(el.querySelectorAll('.skeleton').length).toBeGreaterThan(0);
    answer(http, [
      claimOf({ raw_value: 4, evidence: { text: '4', start: 5, end: 6 }, parameters: {}, result: result('CONDITIONAL') }),
      claimOf({ raw_value: 300, evidence: { text: '300 K', start: 10, end: 15 }, parameters: {}, result: result('COMPATIBLE') }),
    ]);
    await settle(fixture);
    answer(http, [
      claimOf({ limit_id: null, quantity: 'specific energy', status: 'needs_input', result: null, reason: 'no established law identified for this quantity', evidence: { text: 'Cell 5 kWh in 1 kg', start: 0, end: 18 }, parameters: {} }),
    ]);
    await settle(fixture);
    await drain(http, fixture);
    expect(el.getAttribute('data-state')).toBe('reviewed');
    return ctx;
  }

  it('starts with the hero, a pre-filled text, a character counter and sample chips', async () => {
    const { fixture, el } = await setup();
    expect(el.querySelector('.hero-title')?.textContent).toContain('Check any technical claim against the laws of physics');
    expect((el.querySelector('#wb-text') as HTMLTextAreaElement).value).toBe(ILLUSTRATIVE_TEXT);
    expect(el.querySelector('.char-count')?.textContent).toContain(`${Array.from(ILLUSTRATIVE_TEXT).length} characters`);
    const chips = el.querySelectorAll<HTMLButtonElement>('.sample-chip');
    expect(chips).toHaveLength(3);
    expect(el.textContent).toContain('Illustrative');
    chips[1].click();
    await settle(fixture);
    expect((el.querySelector('#wb-text') as HTMLTextAreaElement).value).toBe(SAMPLES[1].text);
    expect(chips[1].getAttribute('aria-pressed')).toBe('true');
    expect(el.querySelector('.dropzone')).not.toBeNull();
  });

  it('streams claims, then shows the summary, the claim list, the highlighted text and the instrument', async () => {
    const { el } = await reviewed();
    expect(el.querySelector('.summary-n')?.textContent).toBe('4');
    const pills = Array.from(el.querySelectorAll('.summary .pill-filter')).map((p) => p.textContent?.replace(/\s+/g, ' ').trim());
    expect(pills).toEqual(['Impossible 1', 'Conditional 1', 'Compatible 1', 'Needs input 1']);
    expect(el.querySelectorAll('.claim-item')).toHaveLength(4);
    // Each claim sentence is tinted; only the value is underlined, with its badge.
    const sentences = Array.from(el.querySelectorAll('.reading .sent'));
    expect(sentences.map((m) => m.className.match(/hl--(\w+)/)?.[1])).toEqual(['impossible', 'conditional', 'needs']);
    expect(sentences[0].textContent?.replace(/\s+/g, ' ').trim()).toBe('Engine 160% at 600 K');
    const values = Array.from(el.querySelectorAll('.reading .val')).map((v) => v.textContent);
    expect(values).toEqual(['160', '24', '3300', '4Cell 5 kWh in 1 kg']);
    // Short chips, never cut: the law's short quantity name and the value.
    expect(Array.from(el.querySelectorAll('.claim-q')).map((c) => c.textContent)).toEqual([
      'efficiency 60.0 %',
      'efficiency 4.00 %',
      'efficiency 300 %',
      'specific energy 60.0 %',
    ]);
    expect(el.querySelector('.inst-pill')?.textContent).toContain('IMPOSSIBLE');
    expect(el.querySelector('.inst-sentence')?.textContent).toBe('The claim is 1.20× what the law allows.');
    const tiles = Array.from(el.querySelectorAll('.tile-n')).map((t) => t.textContent?.trim());
    expect(tiles).toEqual(['0.600', '0.500', '1.20×']);
    expect(el.querySelector('.wording')?.textContent).toContain('inconsistent with limit carnot');
  });

  it('filters the claims by verdict from the summary pills', async () => {
    const { fixture, el } = await reviewed();
    const pills = el.querySelectorAll<HTMLButtonElement>('.summary .pill-filter');
    pills[1].click();
    await settle(fixture);
    expect(pills[1].getAttribute('aria-pressed')).toBe('true');
    expect(el.querySelectorAll('.claim-item')).toHaveLength(1);
    expect(el.querySelector('.inst-pill')?.textContent).toContain('CONDITIONAL');
    expect(Array.from(el.querySelectorAll('.reading .sent')).map((m) => m.classList.contains('is-dim'))).toEqual([true, false, true]);
    pills[1].click();
    await settle(fixture);
    expect(el.querySelectorAll('.claim-item')).toHaveLength(4);
  });

  it('navigates with J, K and the arrow keys, wrapping, and ignores keys typed in fields', async () => {
    const { fixture, el } = await reviewed();
    const current = () => el.querySelector('.claim-item[aria-current="true"] .claim-n')?.textContent;
    const press = async (key: string, target: EventTarget = document) => {
      target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
      await settle(fixture);
    };
    expect(current()).toBe('1');
    await press('j');
    expect(current()).toBe('2');
    await press('ArrowDown');
    expect(current()).toBe('3');
    await press('k');
    expect(current()).toBe('2');
    await press('ArrowUp');
    await press('ArrowUp');
    expect(current()).toBe('4');
    const input = document.createElement('input');
    document.body.appendChild(input);
    await press('j', input);
    expect(current()).toBe('4');
    input.remove();
    (el.querySelectorAll<HTMLElement>('.reading .sent')[1]).click();
    await settle(fixture);
    expect(current()).toBe('2');
  });

  it('opens the source accordion and the trace drawer, closed by Escape', async () => {
    const { fixture, el } = await reviewed();
    const toggle = el.querySelector('.source-toggle') as HTMLButtonElement;
    expect((el.querySelector('#source-panel') as HTMLElement).hidden).toBe(true);
    toggle.click();
    await settle(fixture);
    expect(toggle.getAttribute('aria-expanded')).toBe('true');
    const panel = el.querySelector('#source-panel') as HTMLElement;
    expect(panel.hidden).toBe(false);
    expect(panel.textContent).toContain('60%');
    expect(panel.textContent).toContain('built-in law');
    expect(panel.textContent).toMatch(/\d{4}-\d{2}-\d{2} \d{2}:\d{2} UTC/);
    const drawer = el.querySelector('#how-drawer') as HTMLElement;
    expect(drawer.hidden).toBe(true);
    (el.querySelector('.how-button') as HTMLButtonElement).click();
    await settle(fixture);
    expect(drawer.hidden).toBe(false);
    expect(drawer.textContent).toContain('Paragraph 3 of 3: 1 claim(s)');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await settle(fixture);
    expect(drawer.hidden).toBe(true);
  });

  it('downloads the review as a Markdown report', async () => {
    const { el } = await reviewed();
    let blob: Blob | null = null;
    let name = '';
    const create = URL.createObjectURL;
    const revoke = URL.revokeObjectURL;
    const click = HTMLAnchorElement.prototype.click;
    URL.createObjectURL = (b: Blob) => ((blob = b), 'blob:x');
    URL.revokeObjectURL = () => undefined;
    HTMLAnchorElement.prototype.click = function (this: HTMLAnchorElement) {
      name = this.download;
    };
    try {
      (el.querySelector('.download-button') as HTMLButtonElement).click();
    } finally {
      HTMLAnchorElement.prototype.click = click;
      URL.createObjectURL = create;
      URL.revokeObjectURL = revoke;
    }
    expect(name).toBe('limina-review.md');
    const md = await (blob as unknown as Blob).text();
    expect(md).toContain('# Limina review');
    expect(md).toContain('4 claims, 3 paragraphs.');
    expect(md).toContain('- Verdict: IMPOSSIBLE');
    expect(md).toContain('Generated by Limina; AI-written laws are not validated');
    expect(TestBed.inject(ToastService).toasts().map((t) => t.text)).toContain('Review downloaded as Markdown');
  });

  it('retries an unusable answer, reports a failed paragraph with an error toast', async () => {
    const { http, fixture, el } = await setup();
    type(el, 'COP 1');
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    for (let i = 0; i < 3; i++) {
      const r = http.expectOne('/v1/analyze');
      expect(r.request.body).toEqual({ text: 'COP 1' });
      r.flush({ error: { code: 'ExtractionFailed', message: 'no JSON' } }, { status: 502, statusText: 'Bad Gateway' });
      await settle(fixture);
    }
    await drain(http, fixture);
    expect(el.querySelector('.inst-pill')?.textContent).toContain('NOT ANALYSED');
    expect(el.querySelector('.nv')?.textContent).toContain('ExtractionFailed');
    expect(el.querySelector('.tiles')).toBeNull();
    const toasts = TestBed.inject(ToastService).toasts();
    expect(toasts.some((t) => t.kind === 'error' && t.text.includes('Paragraph 1 was not analysed: ExtractionFailed'))).toBe(true);
  });

  it('advances the stepper only on pipeline events', async () => {
    const { http, fixture, el } = await setup();
    type(el, 'Engine 60% at 600 K\n\nPump 4 at 300 K');
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    const states = () => Array.from(el.querySelectorAll('.step')).map((s) => s.className.match(/step--(\w+)/)?.[1]);
    // Request sent, no answer yet: only Reading is active.
    expect(states()).toEqual(['active', 'waiting', 'waiting', 'waiting']);
    expect(el.querySelector('.reading')).not.toBeNull();
    expect(el.querySelector('.feed')).not.toBeNull();
    answer(http, [
      claimOf({
        evidence: { text: '60%', start: 7, end: 10 },
        trace: [
          { step: 'extraction', status: 'done', detail: 'read efficiency' },
          { step: 'limit', status: 'done', detail: 'built-in limit carnot' },
        ],
      }),
    ]);
    await settle(fixture);
    // An answer carrying the server's law identification.
    expect(states()).toEqual(['active', 'active', 'waiting', 'waiting']);
    expect(el.querySelector('.reading .val')).not.toBeNull();
    answer(http, [claimOf({ raw_value: 4, evidence: { text: '4', start: 5, end: 6 }, parameters: {} })]);
    await settle(fixture);
    // Every paragraph answered: Reading and Identifying are done; sources are being researched.
    expect(states()).toEqual(['done', 'done', 'active', 'waiting']);
    await drain(http, fixture);
    expect(el.getAttribute('data-state')).toBe('reviewed');
  });

  it('shows a card instead of metrics for a disputed law, with both candidates and their divergence', async () => {
    const { http, fixture, el } = await setup();
    type(el, 'Probe 3.6e8 m/s');
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    answer(http, [
      claimOf({
        limit_id: null,
        quantity: 'speed',
        raw_value: 3.6e8,
        raw_unit: 'm/s',
        value: null,
        evidence: { text: '3.6e8 m/s', start: 6, end: 15 },
        parameters: {},
        status: 'needs_input',
        result: null,
        reason: 'the two models wrote different laws; no verdict is given',
        trace: [
          { step: 'extraction', status: 'done', detail: 'read speed: 3.6e8 m/s' },
          { step: 'law', status: 'done', detail: 'Nothing with mass reaches c' },
        ],
        law: {
          source: 'draft_disputed',
          limit_id: null,
          sentence: 'Nothing with mass reaches c',
          comparison: null,
          attempts: [
            { model: 'model-a', ok: true, attempts: 1, errors: [], definition: { id: 'a', title: 'a', direction: 'max', formula: 'c' }, bound_at_claim: 299792458, bound_note: null },
            { model: 'model-b', ok: true, attempts: 1, errors: [], definition: { id: 'b', title: 'b', direction: 'max', formula: '2*c' }, bound_at_claim: 599584916, bound_note: null },
          ],
        },
      }),
    ]);
    await drain(http, fixture);
    expect(el.querySelector('.inst-pill')?.textContent).toContain('DISPUTED');
    expect(el.querySelector('.tiles')).toBeNull();
    expect(el.querySelector('.visual')).toBeNull();
    const card = el.querySelector('.nv')?.textContent ?? '';
    expect(card).toContain('What happened');
    expect(card).toContain('What was tried');
    expect(card).toContain('What would settle it');
    expect(Array.from(el.querySelectorAll('.cand-formula')).map((f) => f.textContent)).toEqual(['c', '2*c']);
    expect(el.querySelector('.nv-div')?.textContent).toContain('50.0 %');
    // The plain sentence is shown once.
    expect((card.match(/Two models wrote/g) ?? []).length).toBe(1);
  });

  it('asks for a missing quantity and re-evaluates through the engine', async () => {
    const { http, fixture, el } = await setup();
    type(el, 'Engine 60%');
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    const insufficient = { ...claimOf().result!, verdict: 'INSUFFICIENT_DATA' as const, bounds: null, wording: 'insufficient data', missing_parameters: [{ id: 'Tc', status: 'NoDomain' as const }] };
    answer(http, [claimOf({ evidence: { text: '60%', start: 7, end: 10 }, result: insufficient })]);
    await drain(http, fixture);
    expect(el.querySelector('.tiles')).toBeNull();
    const input = el.querySelector('.supply-input') as HTMLInputElement;
    expect(input).not.toBeNull();
    const button = el.querySelector('.reevaluate') as HTMLButtonElement;
    expect(button.disabled).toBe(true);
    input.value = '300';
    input.dispatchEvent(new Event('input'));
    await settle(fixture);
    expect(button.disabled).toBe(false);
    button.click();
    await settle(fixture);
    const req = http.expectOne('/v1/evaluate');
    expect(req.request.body.limit_id).toBe('carnot');
    expect(req.request.body.parameters).toEqual({ Th: { value: 600, unit: 'K' }, Tc: { value: 300, unit: 'K' } });
    req.flush(claimOf().result);
    await drain(http, fixture);
    expect(el.querySelector('.inst-pill')?.textContent).toContain('IMPOSSIBLE');
    expect(el.querySelector('.tiles')).not.toBeNull();
    expect(el.querySelector('.nv')).toBeNull();
  });

  it('lists the sources searched for a claim with no law', async () => {
    const { fixture, el } = await reviewed();
    (el.querySelectorAll<HTMLButtonElement>('.claim-item')[3]).click();
    await settle(fixture);
    expect(el.querySelector('.tiles')).toBeNull();
    const sources = Array.from(el.querySelectorAll('.nv-list:not(.nv-steps) li')).map((l) => l.textContent?.trim());
    expect(sources[0]).toBe(`Built-in library: ${LIMITS.length} laws compared by quantity name and dimension, none matched`);
    expect(sources).toContain('No web research is run by this server');
  });

  it('shows the empty state when no claim is found', async () => {
    const { http, fixture, el } = await setup();
    type(el, 'Nothing here 42');
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    answer(http, []);
    await drain(http, fixture);
    expect(el.querySelector('.empty-title')?.textContent).toContain('No claim found');
  });

  it('runs a built-in sample through the engine when extraction is unavailable', async () => {
    const { http, fixture, el } = await setup({ extraction: false, law_authoring: false, vision: false, models: [] });
    expect(el.querySelector('.notice')?.textContent).toContain('Extraction is unavailable');
    expect(el.querySelector('.dropzone')).toBeNull();
    (el.querySelectorAll<HTMLButtonElement>('.sample-chip')[2]).click();
    await settle(fixture);
    (el.querySelector('.wb-analyze') as HTMLButtonElement).click();
    await settle(fixture);
    const verdicts = ['IMPOSSIBLE', 'COMPATIBLE', 'COMPATIBLE'];
    for (const v of verdicts) {
      const r = http.expectOne('/v1/evaluate');
      r.flush({ ...claimOf().result!, verdict: v, limit_id: r.request.body.limit_id, wording: 'engine wording' });
      await settle(fixture);
    }
    http.expectNone('/v1/analyze');
    await drain(http, fixture);
    expect(el.querySelectorAll('.claim-item')).toHaveLength(3);
    expect(el.querySelector('.inst-pill')?.textContent).toContain('IMPOSSIBLE');
    expect(Array.from(el.querySelectorAll('.reading .val')).map((m) => m.textContent)).toEqual(['170', '245', '3300']);
  });

  it('shows the offline notice when the engine cannot be reached', async () => {
    await TestBed.configureTestingModule({
      imports: [Workbench],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();
    const http = TestBed.inject(HttpTestingController);
    const fixture = TestBed.createComponent(Workbench);
    await fixture.whenStable();
    for (const r of http.match('/v1/limits')) r.error(new ProgressEvent('error'));
    for (const r of http.match('/v1/pipeline')) r.error(new ProgressEvent('error'));
    await settle(fixture);
    expect((fixture.nativeElement as HTMLElement).querySelector('.notice--error')?.textContent).toContain('The engine is offline');
  });
});
