import { expect, Page, test } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

/**
 * The 3D laboratory against the running app and API (spec laboratory-3d,
 * task 4.3): live evaluation, the hatched region past the limit, the
 * corridor's zones outside the validity domain, DOM checks at four sizes,
 * and the release of the WebGL context on a round trip through the
 * analysis screen. Needs the app and the API running.
 */
const OUT = join(__dirname, '..', '..', 'docs', 'screens');
mkdirSync(OUT, { recursive: true });

const VERDICT = /^(IMPOSSIBLE|CONDITIONAL|COMPATIBLE|INSUFFICIENT DATA|OUTSIDE MODEL DOMAIN|NEEDS AN ASSUMPTION)$/;
const TECH = /(?<!heat )\b(engine|extraction|pipeline|api|server|endpoint|json|fraud|lie|scam|hoax)\b/;

const SIZES = [
  { name: '1920x1080', width: 1920, height: 1080, desktop: true },
  { name: '1440x900', width: 1440, height: 900, desktop: true },
  { name: '768x1024', width: 768, height: 1024, desktop: false },
  { name: '390x844', width: 390, height: 844, desktop: false },
];

/** Boxes that must not overlap on the laboratory page. */
const KEY = ['.topbar', '.lab-view', '.lab-hud'];

/** Counts the WebGL contexts the page creates and the ones it loses (released). */
async function trackWebgl(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const w = window as unknown as { __gl: { created: number; lost: number } };
    w.__gl = { created: 0, lost: 0 };
    const original = HTMLCanvasElement.prototype.getContext;
    HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement & { __counted?: boolean }, type: string, ...rest: unknown[]) {
      const ctx = (original as (this: HTMLCanvasElement, t: string, ...r: unknown[]) => RenderingContext | null).call(this, type, ...rest);
      if (ctx && /webgl/.test(type) && !this.__counted) {
        this.__counted = true;
        w.__gl.created++;
        this.addEventListener('webglcontextlost', () => w.__gl.lost++);
      }
      return ctx;
    } as typeof original;
  });
}

const live = (page: Page) => page.evaluate(() => {
  const g = (window as unknown as { __gl: { created: number; lost: number } }).__gl;
  return g.created - g.lost;
});

/** Console errors and uncaught exceptions; refused requests are expected answers (422), not errors. */
function collectErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error' && !/Failed to load resource/.test(m.text())) errors.push(`console: ${m.text()}`);
  });
  return errors;
}

async function openLab(page: Page): Promise<void> {
  await page.goto('/laboratory');
  await expect(page.locator('select.lab-law')).toBeVisible({ timeout: 30_000 });
  await expect(page.locator('.lab-verdict')).toHaveText(VERDICT, { timeout: 30_000 });
}

/** Picks one of the two views with its segmented control, and waits for the stage to show it. */
async function showView(page: Page, mode: 'apparatus' | 'boundary'): Promise<void> {
  await page.locator(`.lab-segment[data-mode='${mode}']`).click();
  await expect(page.locator('.lab-stage')).toHaveAttribute('data-view', mode);
}

async function domChecks(page: Page, desktop: boolean): Promise<string[]> {
  return page.evaluate(
    ({ keys, desktop }) => {
      const problems: string[] = [];
      const de = document.documentElement;
      if (de.scrollWidth > innerWidth + 1) problems.push(`horizontal overflow ${de.scrollWidth} > ${innerWidth}`);
      if (desktop && de.scrollHeight > innerHeight + 1) problems.push(`page scrolls ${de.scrollHeight} > ${innerHeight}`);
      const boxes = keys
        .map((k) => ({ k, el: document.querySelector(k) as HTMLElement | null }))
        .filter((b): b is { k: string; el: HTMLElement } => !!b.el && b.el.getClientRects().length > 0)
        .map((b) => ({ k: b.k, r: b.el.getBoundingClientRect() }));
      for (let i = 0; i < boxes.length; i++)
        for (let j = i + 1; j < boxes.length; j++) {
          const a = boxes[i].r;
          const b = boxes[j].r;
          const w = Math.min(a.right, b.right) - Math.max(a.left, b.left);
          const h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
          if (w > 1 && h > 1) problems.push(`${boxes[i].k} overlaps ${boxes[j].k}`);
        }
      for (const el of Array.from(document.querySelectorAll<HTMLElement>('.lab *, .topbar *'))) {
        const own = Array.from(el.childNodes).some((n) => n.nodeType === 3 && (n.textContent ?? '').trim() !== '');
        if (!own || el.getClientRects().length === 0) continue;
        const size = parseFloat(getComputedStyle(el).fontSize);
        if (size < 12) problems.push(`text under 12px: ${el.className || el.tagName} ${size}px`);
      }
      for (const k of ['.lab-hud', '.lab-view']) {
        const el = document.querySelector(k);
        if (el && el.getBoundingClientRect().right > innerWidth + 1) problems.push(`${k} overflows the window`);
      }
      return problems;
    },
    { keys: KEY, desktop },
  );
}

test('the laboratory opens, changes law and evaluates live', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openLab(page);
  const law = page.locator('select.lab-law');
  const verdict = page.locator('.lab-verdict');
  const stage = page.locator('.lab-stage');
  await showView(page, 'boundary');

  // Carnot starts with a heat engine cycle chosen, so it has a verdict.
  await law.selectOption('carnot');
  await expect(page.locator('.lab-choice').first()).toBeVisible();
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  await expect(stage).toHaveAttribute('data-topology', 'surface');
  const before = await page.locator('.lab-figures dd').first().textContent();

  // Pushing the claim to the end of its track moves it past the limit: the region is hatched.
  const track = page.locator('#lab-track-claim');
  await track.focus();
  await track.press('End');
  await expect(verdict).toHaveText('IMPOSSIBLE', { timeout: 30_000 });
  await expect(page.locator('.lab-figures dd').first()).not.toHaveText(before ?? '');
  await expect(page.locator('.lab-wording')).toContainText('inconsistent');
  await expect.poll(async () => Number(await stage.getAttribute('data-hatched')), { timeout: 30_000 }).toBeGreaterThan(0);
  await page.waitForTimeout(600);
  await page.screenshot({ path: join(OUT, 'laboratory-surface-1440x900.png') });

  // Back near zero, the claim is within the limit and nothing is hatched.
  await track.press('Home');
  await expect(verdict).toHaveText(/^(COMPATIBLE|CONDITIONAL)$/, { timeout: 30_000 });
  await expect.poll(async () => Number(await stage.getAttribute('data-hatched')), { timeout: 30_000 }).toBe(0);

  // Find breaking point answers with a sentence and no technical words.
  await page.locator('.lab-solve').click();
  await expect(page.locator('.lab-note')).toBeVisible({ timeout: 30_000 });
  expect((await page.locator('.lab').innerText()).toLowerCase()).not.toMatch(TECH);

  // A law with no parameter gets the chamber; the verdict is still written.
  const chamber = await page.evaluate(async () => {
    const r = await fetch('/v1/limits').then((x) => x.json());
    return (r.limits as { id: string; parameters: unknown[] }[]).find((l) => l.parameters.length === 0)?.id ?? null;
  });
  if (chamber) {
    await law.selectOption(chamber);
    await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
    await expect(stage).toHaveAttribute('data-topology', 'chamber');
    await page.waitForTimeout(600);
    await page.screenshot({ path: join(OUT, 'laboratory-chamber-1440x900.png') });
  }
});

test('the corridor marks the values where the law does not apply', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openLab(page);
  const stage = page.locator('.lab-stage');
  await showView(page, 'boundary');
  // Hydrogen levels: the quantum number must be at least 1; typing 0.2 widens the corridor below it.
  await page.locator('select.lab-law').selectOption('hydrogen_levels');
  await expect(stage).toHaveAttribute('data-topology', 'corridor', { timeout: 30_000 });
  // The law's values are loaded once its first verdict is written.
  await expect(page.locator('.lab-verdict')).toHaveText(VERDICT, { timeout: 30_000 });
  const field = page.locator('.lab-control', { has: page.locator('#lab-track-n') }).locator('.lab-number');
  await field.fill('0.2');
  await field.press('Enter');
  await field.blur();
  await expect.poll(async () => Number(await stage.getAttribute('data-zones')), { timeout: 30_000 }).toBeGreaterThan(0);
  await expect(stage).toHaveAttribute('aria-label', /where the law does not apply/);
  await page.waitForTimeout(600);
  await page.screenshot({ path: join(OUT, 'laboratory-corridor-1440x900.png') });
});

test('the physical apparatus is the default view and follows the engine', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openLab(page);
  const law = page.locator('select.lab-law');
  const verdict = page.locator('.lab-verdict');
  const stage = page.locator('.lab-stage');
  await expect(stage).toHaveAttribute('data-view', 'apparatus');
  await expect(page.locator(".lab-segment[data-mode='apparatus']")).toHaveAttribute('aria-checked', 'true');

  // Carnot: the energy bench; past the limit the rejected flow collapses behind a barrier.
  await law.selectOption('carnot');
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  await expect(stage).toHaveAttribute('data-apparatus', 'flow');
  const track = page.locator('#lab-track-claim');
  await track.focus();
  await track.press('End');
  await expect(verdict).toHaveText('IMPOSSIBLE', { timeout: 30_000 });
  await expect(stage).toHaveAttribute('data-bench', 'collapsed', { timeout: 30_000 });
  await page.waitForTimeout(600);
  await page.screenshot({ path: join(OUT, 'laboratory-apparatus-flow-1440x900.png') });
  await track.press('Home');
  await expect(verdict).toHaveText(/^(COMPATIBLE|CONDITIONAL)$/, { timeout: 30_000 });
  await expect(stage).toHaveAttribute('data-bench', 'flowing', { timeout: 30_000 });

  // The same values drive the mathematical boundary, and back, by keyboard.
  await showView(page, 'boundary');
  await expect(stage).toHaveAttribute('data-topology', 'surface');
  await expect(stage).not.toHaveAttribute('data-bench', /.+/);
  await page.locator(".lab-segment[data-mode='boundary']").press('ArrowLeft');
  await expect(stage).toHaveAttribute('data-view', 'apparatus');
  await expect(page.locator(".lab-segment[data-mode='apparatus']")).toBeFocused();
  await expect(verdict).toHaveText(/^(COMPATIBLE|CONDITIONAL)$/);
  expect((await page.locator('.lab').innerText()).toLowerCase()).not.toMatch(TECH);
});

test('each kind of law gets its bench, in both views', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openLab(page);
  const law = page.locator('select.lab-law');
  const verdict = page.locator('.lab-verdict');
  const stage = page.locator('.lab-stage');
  const cases = [
    { id: 'escape_speed_min', bench: 'trajectory', states: /^(escapes|captured)$/ },
    { id: 'schwarzschild_radius', bench: 'trajectory', states: /^radius$/ },
    { id: 'radioactive_decay', bench: 'quantum', states: /^population$/ },
    { id: 'rayleigh_criterion', bench: 'wave', states: /^(resolved|merged)$/ },
    { id: 'hydrogen_levels', bench: 'quantum', states: /^shells$/ },
    { id: 'heisenberg', bench: 'quantum', states: /^packet$/ },
  ];
  for (const c of cases) {
    await law.selectOption(c.id);
    await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
    await expect(stage).toHaveAttribute('data-apparatus', c.bench);
    await expect(stage).toHaveAttribute('data-bench', c.states, { timeout: 30_000 });
    await page.waitForTimeout(500);
    await page.screenshot({ path: join(OUT, `laboratory-apparatus-${c.bench}-${c.id}-1440x900.png`) });
    await showView(page, 'boundary');
    await expect(verdict).toHaveText(VERDICT);
    await showView(page, 'apparatus');
    await expect(stage).toHaveAttribute('data-bench', c.states);
  }
  // The optics bench merges the peaks once the claim goes past the limit.
  await law.selectOption('rayleigh_criterion');
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  const track = page.locator('#lab-track-claim');
  await track.focus();
  await track.press('Home');
  await expect(verdict).toHaveText('IMPOSSIBLE', { timeout: 30_000 });
  await expect(stage).toHaveAttribute('data-bench', 'merged', { timeout: 30_000 });
});

test('switching views releases what the other view drew', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await trackWebgl(page);
  const errors = collectErrors(page);
  await openLab(page);
  await page.locator('select.lab-law').selectOption('carnot');
  await expect(page.locator('.lab-verdict')).toHaveText(VERDICT, { timeout: 30_000 });
  const stage = page.locator('.lab-stage');
  const geometries = async () => {
    await page.waitForTimeout(700);
    return Number(await stage.getAttribute('data-geometries'));
  };
  await showView(page, 'boundary');
  const boundary = await geometries();
  await showView(page, 'apparatus');
  const apparatus = await geometries();
  for (let i = 0; i < 4; i++) {
    await showView(page, 'boundary');
    await showView(page, 'apparatus');
  }
  // After four round trips the GPU holds what one view needs, not four times it.
  expect(await geometries()).toBe(apparatus);
  await showView(page, 'boundary');
  expect(await geometries()).toBe(boundary);
  expect(await live(page)).toBe(1);
  expect(errors).toEqual([]);
});

/** The screen place of a named handle, in page pixels, once the observatory has published it. */
async function handleAt(page: Page, name: string): Promise<{ x: number; y: number }> {
  const stage = page.locator('.lab-stage');
  let found: { x: number; y: number } | null = null;
  await expect
    .poll(
      async () => {
        const list = JSON.parse((await stage.getAttribute('data-handles')) ?? '[]') as { name: string; x: number; y: number }[];
        const h = list.find((x) => x.name === name);
        const box = await page.locator('canvas.lab-canvas').boundingBox();
        found = h && box ? { x: box.x + h.x, y: box.y + h.y } : null;
        return found !== null;
      },
      { timeout: 15_000 },
    )
    .toBe(true);
  return found as unknown as { x: number; y: number };
}

/** Drags from a point by an offset in small steps, as a hand would. */
async function drag(page: Page, from: { x: number; y: number }, dx: number, dy: number): Promise<void> {
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  for (let i = 1; i <= 8; i++) await page.mouse.move(from.x + (dx * i) / 8, from.y + (dy * i) / 8);
  await page.mouse.up();
}

test('dragging the probe, then a wall, moves the sliders and asks the engine again', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await openLab(page);
  const law = page.locator('select.lab-law');
  const verdict = page.locator('.lab-verdict');
  const canvas = page.locator('canvas.lab-canvas');

  // Mathematical boundary: the probe of Carnot; dragging it up raises the claim past the limit.
  await law.selectOption('carnot');
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  await showView(page, 'boundary');
  const claim = page.locator('#lab-track-claim');
  const before = await claim.inputValue();
  const probe = await handleAt(page, 'probe');
  await page.mouse.move(probe.x, probe.y);
  await expect.poll(() => canvas.evaluate((c) => (c as HTMLElement).style.cursor)).toBe('grab');
  await drag(page, probe, 0, -260);
  await expect.poll(() => claim.inputValue()).not.toBe(before);
  expect(Number(await claim.inputValue())).toBeGreaterThan(Number(before));
  await expect(verdict).toHaveText('IMPOSSIBLE', { timeout: 30_000 });

  // Physical apparatus: Heisenberg's confinement walls; dragging one outward widens the box.
  await law.selectOption('heisenberg');
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  await showView(page, 'apparatus');
  const dp = page.locator('#lab-track-dp');
  const width = await dp.inputValue();
  await handleAt(page, 'wall');
  // The wall on the right of the screen, dragged further right: away from the centre.
  const box = (await canvas.boundingBox())!;
  const walls = (JSON.parse((await page.locator('.lab-stage').getAttribute('data-handles')) ?? '[]') as { name: string; x: number; y: number }[]).filter((h) => h.name === 'wall');
  const right = walls.reduce((a, b) => (b.x > a.x ? b : a));
  await drag(page, { x: box.x + right.x, y: box.y + right.y }, 120, 0);
  await expect.poll(() => dp.inputValue()).not.toBe(width);
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  // The keyboard still drives the same slider.
  await dp.focus();
  await dp.press('Home');
  await expect(dp).toHaveValue('0');
});

test('the glow is on where the browser supports it, and the sound is off until asked', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.addInitScript(() => {
    const w = window as unknown as { __audio: { created: number; closed: number }; AudioContext: typeof AudioContext };
    w.__audio = { created: 0, closed: 0 };
    const Original = w.AudioContext;
    w.AudioContext = class extends Original {
      constructor(options?: AudioContextOptions) {
        super(options);
        w.__audio.created++;
      }
      override close(): Promise<void> {
        w.__audio.closed++;
        return super.close();
      }
    };
  });
  const errors = collectErrors(page);
  await openLab(page);
  await expect(page.locator('.lab-stage')).toHaveAttribute('data-glow', /^(on|off)$/);
  const audio = () => page.evaluate(() => (window as unknown as { __audio: { created: number; closed: number } }).__audio);
  const sound = page.locator('.lab-sound');
  await expect(sound).toHaveAttribute('aria-pressed', 'false');
  await expect(sound).toHaveText('Sound: Off');
  expect((await audio()).created).toBe(0);

  // On by keyboard, then off: one context, closed.
  await sound.focus();
  await page.keyboard.press('Enter');
  await expect(sound).toHaveAttribute('aria-pressed', 'true');
  await expect(sound).toHaveText('Sound: On');
  expect(await audio()).toEqual({ created: 1, closed: 0 });
  await sound.click();
  await expect(sound).toHaveAttribute('aria-pressed', 'false');
  expect(await audio()).toEqual({ created: 1, closed: 1 });

  // On again, then leaving the laboratory closes it.
  await sound.click();
  await page.locator('.lab-back').click();
  await expect(page).toHaveURL(/\/$/);
  await expect.poll(audio).toEqual({ created: 2, closed: 2 });
  expect(errors).toEqual([]);
});

const FORGED = 'magnetic_pressure_forged';
const MU0 = 1.25663706212e-6;

/** A forged draft served by mocked routes: the library gains it, and its requests are answered like the API's. */
async function mockForgedLaw(page: Page): Promise<void> {
  let forged = false;
  await page.route('**/v1/limits/author', (route) => {
    forged = true;
    return route.fulfill({
      json: {
        limit_id: FORGED,
        status: 'draft',
        attempts: 1,
        definition: {},
        errors: [],
        overrides: [],
        report: {
          limit_id: FORGED,
          status: 'draft',
          validated: false,
          dimension: 'passed',
          formulas: [{ configuration: '', formula: 'B^2 / (2 mu0)', monotonicity: { B: 'Increasing' }, samples: 200, finite: true, non_negative: true }],
          validity: [],
        },
      },
    });
  });
  await page.route('**/v1/limits', async (route) => {
    const res = await route.fetch();
    const body = await res.json();
    if (forged)
      body.limits.push({
        id: FORGED,
        direction: 'Max',
        title: 'Magnetic pressure on a coil',
        formula: 'p <= B^2 / (2 mu0)',
        statement: 'The magnetic pressure of a field B on a coil, B^2 / (2 mu0).',
        claim_unit: 'Pa',
        assumptions: [{ id: 'draft_formula', options: ['as_written', 'not_validated'] }],
        parameters: [{ id: 'B', kind: 'Plain', required: true, units: ['T', 'mT'] }],
        assumption_labels: { draft_formula: 'formula status' },
        option_labels: { as_written: 'as written', not_validated: 'not validated' },
        parameter_labels: { B: 'magnetic field' },
        status: 'draft',
      });
    await route.fulfill({ response: res, json: body });
  });
  const boundAt = (b: number) => (b * b) / (2 * MU0);
  await page.route('**/v1/evaluate', async (route) => {
    const req = route.request().postDataJSON();
    if (req.limit_id !== FORGED) return route.continue();
    const bound = boundAt(req.parameters.B.value);
    const e = { b_min: bound, b_max: bound, margin: null };
    await route.fulfill({
      json: {
        verdict: req.claim.value > bound ? 'CONDITIONAL' : 'COMPATIBLE',
        limit_id: FORGED,
        engine_version: 'test',
        wording: req.claim.value > bound ? 'inconsistent with limit magnetic pressure under assumptions formula status = as written' : 'does not exceed the bound',
        bounds: { permissive: e, restrictive: e },
        configurations: [],
        missing_parameters: [],
      },
    });
  });
  await page.route('**/v1/curve', async (route) => {
    const req = route.request().postDataJSON();
    if (req.limit_id !== FORGED) return route.continue();
    const [lo, hi] = req.range ?? [0.1, 10];
    const n = req.points ?? 101;
    const points = Array.from({ length: n }, (_, i) => {
      const x = lo + ((hi - lo) * i) / (n - 1);
      const v = boundAt(x);
      return { x, b_min: v, b_max: v, decisive: v };
    });
    await route.fulfill({ json: { limit_id: FORGED, direction: 'Max', free_parameter: 'B', range: [lo, hi], points, kind: 'bound', validity_zones: [], spacing: 'linear' } });
  });
}

test('forging a law explains a refusal and keeps the sentence', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.route('**/v1/limits/author', (route) =>
    route.fulfill({
      status: 422,
      json: { error: { code: 'LimitNotAuthored', message: 'no valid definition after 3 attempts; last error: the definition does not match the schema' } },
    }),
  );
  await openLab(page);
  await page.locator('.lab-forge-open').click();
  const box = page.locator('.lab-forge-text');
  await expect(box).toBeFocused();
  await page.locator('.lab-forge-example').first().click();
  const sentence = await box.inputValue();
  expect(sentence.length).toBeGreaterThan(20);
  await page.locator('.lab-forge-submit').click();
  const error = page.locator('.lab-forge-error');
  await expect(error).toContainText('No valid law could be written from this sentence after 3 tries');
  expect((await error.innerText()).toLowerCase()).not.toMatch(/(?<!heat )\b(engine|schema|llm|rust|api|server|json)\b/);
  await expect(box).toHaveValue(sentence);
  await expect(page.locator('.lab-forge-panel')).toBeVisible();
  // Escape closes the panel and gives the focus back.
  await box.press('Escape');
  await expect(page.locator('.lab-forge-panel')).toHaveCount(0);
  await expect(page.locator('.lab-forge-open')).toBeFocused();
});

test('a forged law opens at once, flagged as a draft, with its apparatus and boundary', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await mockForgedLaw(page);
  await openLab(page);
  await page.locator('.lab-forge-open').click();
  await page.locator('.lab-forge-text').fill('The magnetic pressure B^2 / (2 mu0) of a field B, in pascals, cannot exceed the yield strength of the coil.');
  await page.locator('.lab-forge-submit').click();
  await expect(page.locator('.lab-forge-step').nth(2)).toHaveText('Checked on 200 sample points');
  await expect(page.locator('select.lab-law')).toHaveValue(FORGED, { timeout: 30_000 });
  await expect(page.locator('.lab-draft')).toHaveText('Draft: written by AI, not validated');
  await expect(page.locator('.lab-forge-note')).toContainText('Magnetic pressure on a coil');
  await expect(page.locator('#lab-track-B')).toBeVisible();
  const stage = page.locator('.lab-stage');
  await expect(page.locator('.lab-verdict')).toHaveText(VERDICT, { timeout: 30_000 });
  await expect(stage).toHaveAttribute('data-view', 'apparatus');
  await expect(stage).toHaveAttribute('data-apparatus', /^(flow|trajectory|wave|quantum)$/);
  await expect(stage).toHaveAttribute('data-bench', /.+/);
  await page.waitForTimeout(500);
  await page.screenshot({ path: join(OUT, 'laboratory-forged-apparatus-1440x900.png') });
  await showView(page, 'boundary');
  await expect(stage).toHaveAttribute('data-topology', 'corridor');
  await expect(page.locator('.lab-verdict')).not.toHaveText('IMPOSSIBLE');
});

test('the stress test names the first breaking limit and shows any law in 3D', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const errors = collectErrors(page);
  await openLab(page);
  const law = page.locator('select.lab-law');
  const verdict = page.locator('.lab-verdict');
  // Carnot pushed far past its limit, then Betz at its starting claim.
  await law.selectOption('carnot');
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  const claim = page.locator('#lab-track-claim');
  await claim.focus();
  await claim.press('End');
  await expect(verdict).toHaveText('IMPOSSIBLE', { timeout: 30_000 });
  await page.locator('.lab-cascade-add').click();
  await law.selectOption('betz');
  await expect(verdict).toHaveText(VERDICT, { timeout: 30_000 });
  await page.locator('.lab-cascade-add').click();
  const items = page.locator('.lab-cascade-item');
  await expect(items).toHaveCount(2);
  const load = page.locator('.lab-cascade-load');
  // Light load: nothing breaks.
  await load.fill('20');
  await expect(items.locator('.lab-cascade-verdict')).toHaveText([/^(COMPATIBLE|CONDITIONAL)$/, /^(COMPATIBLE|CONDITIONAL)$/], { timeout: 30_000 });
  await expect(page.locator('.lab-cascade-first')).toHaveCount(0);
  // Past 100 %: the most constrained law breaks first, and only it.
  await load.fill('120');
  const carnot = page.locator(".lab-cascade-item[data-limit='carnot']");
  const betz = page.locator(".lab-cascade-item[data-limit='betz']");
  await expect(carnot.locator('.lab-cascade-verdict')).toHaveText('IMPOSSIBLE', { timeout: 30_000 });
  await expect(carnot.locator('.lab-cascade-first')).toHaveText('First breaking limit');
  await expect(betz.locator('.lab-cascade-verdict')).toHaveText(/^(COMPATIBLE|CONDITIONAL)$/);
  await expect(page.locator('.lab-cascade-first')).toHaveCount(1);
  await page.screenshot({ path: join(OUT, 'laboratory-stress-test-1440x900.png') });
  // Showing Betz in 3D keeps the stress test and its first breaking limit.
  await betz.locator('.lab-cascade-open').click();
  await expect(law).toHaveValue('betz');
  await expect(page.locator('.lab-stage')).toHaveAttribute('data-apparatus', 'flow');
  await expect(items).toHaveCount(2);
  await expect(carnot.locator('.lab-cascade-first')).toBeVisible();
  await expect(betz).toHaveClass(/is-shown/);
  expect(errors).toEqual([]);
});

test('the analysis sends its laws to the stress test', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  const claim = (limit_id: string, quantity: string, value: number, start: number, constraints: Record<string, string>, parameters: Record<string, unknown>) => ({
    limit_id,
    quantity,
    value,
    unit: 'dimensionless',
    raw_value: value * 100,
    raw_unit: '%',
    evidence: { text: `${value * 100}%`, start, end: start + 3 },
    constraints,
    parameters,
    notes: [],
    flags: [],
    status: 'evaluated',
    result: null,
    reason: null,
    code: null,
    trace: [],
    law: { source: 'built_in', limit_id, sentence: null, attempts: [], comparison: null },
  });
  await page.route('**/v1/analyze', async (route) => {
    const res = await page.request.post('http://localhost:3000/v1/evaluate', {
      data: { limit_id: 'betz', claim: { value: 0.5, unit: 'dimensionless' }, constraints: { reference_area: ['unshrouded_rotor'] }, parameters: {} },
    });
    const betzResult = await res.json();
    const carnotRes = await page.request.post('http://localhost:3000/v1/evaluate', {
      data: { limit_id: 'carnot', claim: { value: 0.6, unit: 'dimensionless' }, constraints: { conversion_process: ['heat_engine_cycle'] }, parameters: { Th: { value: 600, unit: 'K' }, Tc: { value: 300, unit: 'K' } } },
    });
    const carnotResult = await carnotRes.json();
    const a = claim('betz', 'power coefficient', 0.5, 10, { reference_area: 'unshrouded_rotor' }, {});
    const b = claim('carnot', 'efficiency', 0.6, 40, { conversion_process: 'heat_engine_cycle' }, {
      Th: { value: 600, raw_value: 600, unit: 'K', evidence: null, flags: [] },
      Tc: { value: 300, raw_value: 300, unit: 'K', evidence: null, flags: [] },
    });
    await route.fulfill({ json: { claims: [{ ...a, result: betzResult }, { ...b, result: carnotResult }], missing: [], extraction: { model: 'test', cached: true, ms: 1 } } });
  });
  await page.goto('/');
  await page.locator('#wb-text').fill('The rotor reaches 50% and the engine between 600 K and 300 K reaches 60% efficiency.');
  await page.locator('.wb-analyze').click();
  const stress = page.locator('.stress-button');
  await expect(stress).toBeVisible({ timeout: 60_000 });
  await stress.click();
  await expect(page).toHaveURL(/\/laboratory$/);
  await expect(page.locator('.lab-cascade-item')).toHaveCount(2, { timeout: 30_000 });
  await expect(page.locator('.lab-cascade-verdict')).toHaveCount(2, { timeout: 30_000 });
  await expect(page.locator('select.lab-law')).toHaveValue('betz');
});

for (const size of SIZES) {
  test(`laboratory layout at ${size.name}`, async ({ page }) => {
    await page.setViewportSize({ width: size.width, height: size.height });
    await openLab(page);
    await page.waitForTimeout(400);
    expect(await domChecks(page, size.desktop)).toEqual([]);
    // Keyboard: the law chooser takes a visible focus, and the view takes the keys.
    await page.keyboard.press('Tab');
    await page.locator('select.lab-law').focus();
    const focus = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement;
      const s = getComputedStyle(el);
      return { visible: el.matches(':focus-visible'), outline: parseFloat(s.outlineWidth) || 0, style: s.outlineStyle };
    });
    expect(focus.visible).toBe(true);
    expect(focus.outline).toBeGreaterThan(0);
    expect(focus.style).not.toBe('none');
    await page.screenshot({ path: join(OUT, `laboratory-${size.name}.png`) });
  });
}

test('the WebGL context is released on a round trip through the analysis screen', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await trackWebgl(page);
  const errors = collectErrors(page);
  await openLab(page);
  await expect(page.locator('canvas.lab-canvas')).toHaveCount(1);
  await expect.poll(() => live(page)).toBe(1);

  // Leaving the laboratory releases its context and removes its canvas.
  await page.locator('.lab-back').click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator('canvas.lab-canvas')).toHaveCount(0);
  await expect.poll(() => live(page), { timeout: 10_000 }).toBe(0);

  // Coming back creates exactly one again, and the laboratory still works.
  await page.locator('a.topbar-lab').click();
  await expect(page).toHaveURL(/\/laboratory$/);
  await expect(page.locator('.lab-verdict')).toHaveText(VERDICT, { timeout: 30_000 });
  await expect(page.locator('canvas.lab-canvas')).toHaveCount(1);
  await expect.poll(() => live(page)).toBe(1);
  expect(errors).toEqual([]);
});
