// Layout check of the stamped review in headless Edge or Chrome, against a running app and API.
// Usage: node --experimental-websocket web/scripts/layout-check.cjs <width> <height> <out-prefix> [browser] [url]
// Analyzes the illustrative text, then for each claim (arrow keys) reports any scrolling or
// clipping element and saves a screenshot; finally checks that stamps do not animate under
// prefers-reduced-motion. Exits 1 when a check fails.
const { spawn } = require('child_process');
const fs = require('fs');
const [w = '1920', h = '1080', out = 'layout', browser = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe', url = 'http://localhost:4200'] = process.argv.slice(2);
const prof = require('os').tmpdir() + '/limina-layout-' + Date.now();
const p = spawn(browser, ['--headless=new', '--remote-debugging-port=9335', `--window-size=${w},${h}`, '--user-data-dir=' + prof, 'about:blank'], { stdio: 'ignore' });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const MEASURE = `(() => {
  const bad = [];
  const de = document.documentElement;
  if (de.scrollHeight > innerHeight + 1 || de.scrollWidth > innerWidth + 1) bad.push('page scrolls ' + de.scrollWidth + 'x' + de.scrollHeight);
  for (const e of document.querySelectorAll('app-workbench *')) {
    if (e.closest('[hidden]') || e.closest('svg')) continue;
    const cs = getComputedStyle(e);
    const over = e.scrollHeight > e.clientHeight + 1 || e.scrollWidth > e.clientWidth + 1;
    if (!over || e.clientHeight === 0) continue;
    if (/auto|scroll/.test(cs.overflowY + cs.overflowX)) bad.push('scroll container ' + e.className + ' ' + e.scrollHeight + '/' + e.clientHeight);
    else if (/hidden|clip/.test(cs.overflowY + cs.overflowX) && !/ellipsis/.test(cs.textOverflow)) bad.push('clipped ' + e.className + ' ' + e.scrollHeight + '/' + e.clientHeight + ' ' + e.scrollWidth + '/' + e.clientWidth);
  }
  for (const e of document.querySelectorAll('.rv-inst *, .rv-doc *')) {
    const b = e.getBoundingClientRect();
    if (b.height > 0 && b.bottom > innerHeight + 1) { bad.push('below the fold ' + e.tagName + '.' + e.className); break; }
  }
  return { bad, stamp: document.querySelector('.rv-inst-stamp')?.innerText, claim: document.querySelector('.rv-inst-claim .mono')?.innerText };
})()`;
(async () => {
  let target;
  for (let i = 0; i < 50 && !target; i++) { try { const l = await (await fetch('http://127.0.0.1:9335/json')).json(); target = l.find((t) => t.type === 'page'); } catch {} await sleep(200); }
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0; const pending = new Map();
  ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.id && pending.has(d.id)) { pending.get(d.id)(d.result); pending.delete(d.id); } };
  const send = (method, params = {}) => new Promise((r) => { const i = ++id; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
  const ev = async (e) => (await send('Runtime.evaluate', { expression: e, awaitPromise: true, returnByValue: true })).result.value;
  const shot = async (name) => fs.writeFileSync(name, Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).data, 'base64'));
  await send('Emulation.setDeviceMetricsOverride', { width: +w, height: +h, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url });
  await sleep(4000);
  await shot(`${out}-input.png`);
  await ev(`document.querySelector('.wb-analyze').click()`);
  await sleep(1500);
  await shot(`${out}-running.png`);
  const t0 = Date.now();
  while (Date.now() - t0 < 300000 && !(await ev(`!!document.querySelector('.rv-board')`))) await sleep(1000);
  const board = await ev(`({ total: document.querySelector('.rv-board-total')?.innerText, counts: Array.from(document.querySelectorAll('.rv-count')).map(c => c.innerText.replace(/\\s+/g, ' ')),
    stamps: Array.from(document.querySelectorAll('.rv-mark')).map(m => m.innerText.replace(/\\s+/g, ' ')) })`);
  console.log(JSON.stringify(board, null, 1));
  const n = await ev(`document.querySelectorAll('.rv-stamp').length`);
  let failed = false;
  for (let i = 0; i < n; i++) {
    await sleep(3500);
    const r = await ev(MEASURE);
    console.log(`claim ${i + 1}: ${r.stamp} | ${r.claim} | ${r.bad.length ? r.bad.join('; ') : 'no scroll, nothing clipped'}`);
    if (r.bad.length) failed = true;
    await shot(`${out}-claim${i + 1}.png`);
    await ev(`document.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown' }))`);
  }
  await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
  await sleep(300);
  const anim = await ev(`Array.from(document.querySelectorAll('.rv-stamp')).map(s => getComputedStyle(s).animationName)`);
  const still = anim.every((a) => a === 'none');
  console.log(`reduced motion: stamp animations ${JSON.stringify(anim)} -> ${still ? 'none' : 'ANIMATED'}`);
  if (!still) failed = true;
  ws.close(); p.kill(); process.exit(failed ? 1 : 0);
})().catch((e) => { console.error(e); p.kill(); process.exit(1); });
