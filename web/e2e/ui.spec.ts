import { expect, Page, test } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { join } from 'node:path';

/**
 * Screenshots of the three states (input, running, reviewed) at three sizes
 * into docs/screens/, and DOM checks on each: no horizontal overflow, no
 * overlapping key boxes, no text under 12px, no page scroll on desktop
 * sizes, a visible keyboard focus. Needs the app and the API running.
 */
const SIZES = [
  { name: '1920x1080', width: 1920, height: 1080, desktop: true },
  { name: '1440x900', width: 1440, height: 900, desktop: true },
  { name: '390x844', width: 390, height: 844, desktop: false },
];
const OUT = join(__dirname, '..', '..', 'docs', 'screens');
mkdirSync(OUT, { recursive: true });

/** Elements whose boxes must not overlap each other, per state. */
const KEY = {
  input: ['.topbar', '.hero-title', '.hero-sub', '.input-card', '.samples'],
  running: ['.topbar', '.run-bar', '.stepper', '.stream'],
  reviewed: ['.topbar', '.summary', '.claim-list', '.reading', '.inst'],
  instrument: ['.inst-head', '.inst-sentence', '.tiles', '.explorer-axis', '.explorer-figure svg', '.readout', '.explorer-figure .fig-caption', '.explorer-sliders', '.visual-side', '.gauge svg', '.source'],
};

async function checks(page: Page, state: keyof typeof KEY, desktop: boolean) {
  const r = await page.evaluate(
    ({ keys, desktop }) => {
      const problems: string[] = [];
      const de = document.documentElement;
      if (de.scrollWidth > innerWidth + 1) problems.push(`horizontal overflow ${de.scrollWidth} > ${innerWidth}`);
      if (desktop && de.scrollHeight > innerHeight + 1) problems.push(`page scrolls ${de.scrollHeight} > ${innerHeight}`);
      const boxes = keys
        .map((k) => ({ k, el: document.querySelector(k) as HTMLElement | null }))
        .filter((b) => b.el && b.el.offsetParent !== null)
        .map((b) => ({ k: b.k, r: (b.el as HTMLElement).getBoundingClientRect() }));
      for (let i = 0; i < boxes.length; i++)
        for (let j = i + 1; j < boxes.length; j++) {
          const a = boxes[i].r;
          const b = boxes[j].r;
          const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left);
          const oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
          if (ox > 1 && oy > 1) problems.push(`overlap ${boxes[i].k} / ${boxes[j].k}`);
        }
      const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
      const small = new Set<string>();
      for (let n = walker.nextNode(); n; n = walker.nextNode()) {
        const t = (n.textContent ?? '').trim();
        const el = n.parentElement;
        if (!t || !el || el.closest('[hidden], .sr-only, .status-text, .gauge-alt, svg, title, desc')) continue;
        const cs = getComputedStyle(el);
        if (cs.visibility === 'hidden' || cs.display === 'none' || el.getClientRects().length === 0) continue;
        if (parseFloat(cs.fontSize) < 12) small.add(`${el.tagName}.${el.className} ${cs.fontSize} "${t.slice(0, 30)}"`);
      }
      problems.push(...Array.from(small).map((s) => `text under 12px: ${s}`));
      return problems;
    },
    { keys: KEY[state], desktop },
  );
  expect(r, `${state}: ${r.join('; ')}`).toEqual([]);
}

/** The first Tab stops show a visible focus indicator (outline or ring). */
async function focusVisible(page: Page) {
  for (let i = 0; i < 4; i++) {
    await page.keyboard.press('Tab');
    const ok = await page.evaluate(() => {
      const el = document.activeElement as HTMLElement | null;
      if (!el || el === document.body) return true;
      const ring = (e: Element) => {
        const cs = getComputedStyle(e);
        return (cs.outlineStyle !== 'none' && parseFloat(cs.outlineWidth) > 0) || cs.boxShadow !== 'none';
      };
      // The textarea shows its focus as the glow of its card (:focus-within).
      const card = el.closest('.input-card');
      return ring(el) || (card !== null && ring(card));
    });
    expect(ok, `focus visible on tab stop ${i + 1}`).toBe(true);
  }
}

for (const size of SIZES) {
  test(`three states at ${size.name}`, async ({ page }) => {
    await page.setViewportSize({ width: size.width, height: size.height });
    await page.route('**/v1/analyze', async (route) => {
      await new Promise((r) => setTimeout(r, 900));
      await route.continue();
    });
    await page.goto('/');
    await expect(page.locator('.status--online')).toBeVisible({ timeout: 20_000 });
    await expect(page.locator('#wb-text')).toBeVisible();
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(OUT, `input-${size.name}.png`) });
    await checks(page, 'input', false);
    await focusVisible(page);

    await page.locator('.wb-analyze').click();
    await expect(page.locator('.stream-card:not(.skeleton)').nth(1)).toBeVisible({ timeout: 60_000 });
    await page.screenshot({ path: join(OUT, `running-${size.name}.png`) });
    await checks(page, 'running', false);

    await expect(page.locator('.summary')).toBeVisible({ timeout: 120_000 });
    await page.waitForTimeout(2500);
    await page.screenshot({ path: join(OUT, `reviewed-${size.name}.png`) });
    await checks(page, 'reviewed', size.desktop);
    await checks(page, 'instrument', false);

    // Keyboard navigation and the trace drawer.
    await page.locator('.claim-item').first().focus();
    await page.keyboard.press('j');
    await expect(page.locator('.claim-item[aria-current="true"] .claim-n')).toHaveText('2');
    await page.keyboard.press('k');
    await expect(page.locator('.claim-item[aria-current="true"] .claim-n')).toHaveText('1');
    if (size.desktop) {
      await page.keyboard.press('Control+k');
      await expect(page.locator('.palette')).toBeVisible();
      await page.screenshot({ path: join(OUT, `palette-${size.name}.png`) });
      await page.keyboard.press('Escape');
      await expect(page.locator('.palette')).toHaveCount(0);
      await page.locator('.how-button').click();
      await expect(page.locator('#how-drawer')).toBeVisible();
      await page.screenshot({ path: join(OUT, `trace-${size.name}.png`) });
      await page.keyboard.press('Escape');
      await expect(page.locator('#how-drawer')).toBeHidden();
    }
    // A claim with parameters: the explorer with its crosshair readout, and the COP apparatus.
    await page.locator('.claim-item').nth(3).click();
    await expect(page.locator('.explorer-hit')).toBeVisible({ timeout: 20_000 });
    await page.waitForTimeout(1500);
    await page.locator('.explorer-hit').scrollIntoViewIfNeeded();
    const hit = await page.locator('.explorer-hit').boundingBox();
    if (hit) await page.mouse.move(hit.x + hit.width * 0.4, hit.y + hit.height / 2);
    await expect(page.locator('.readout')).toContainText('bound');
    await page.screenshot({ path: join(OUT, `reviewed-explorer-${size.name}.png`) });
    await page.evaluate(() => scrollTo(0, 0));
    await checks(page, 'reviewed', size.desktop);
    await checks(page, 'instrument', false);
  });
}

test('reduced motion turns animations off', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await page.locator('.wb-analyze').click();
  await expect(page.locator('.summary')).toBeVisible({ timeout: 120_000 });
  const animated = await page.evaluate(() =>
    Array.from(document.querySelectorAll('*'))
      .map((e) => getComputedStyle(e).animationName)
      .filter((n) => n && n !== 'none'),
  );
  expect(animated).toEqual([]);
});
