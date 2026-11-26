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
