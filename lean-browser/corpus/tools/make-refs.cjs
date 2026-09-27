// Chromium reference PNGs for the corpus (plan §9): same viewport, DPR 1,
// --disable-gpu --hide-scrollbars, JavaScript disabled, one PNG per scroll
// position (top / 50% / bottom) matching what `lean-browser --serve` paints.
//
//   NODE_PATH=/opt/node22/lib/node_modules PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers \
//     node corpus/tools/make-refs.cjs [--viewport 1280x800] [--only name]
//
// Needs `playwright` resolvable through NODE_PATH (CommonJS honours it) and
// the Chromium version pinned in refs/VERSION. Fonts: the corpus pins
// DejaVu, so the container running this needs the same DejaVu files the
// renderer is pointed at (LEAN_FONT_DIR / system fonts).
const { chromium } = require('playwright');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '..', '..');
const corpus = path.join(root, 'corpus');
const refs = path.join(root, 'refs');
const args = process.argv.slice(2);
let viewport = { width: 1280, height: 800 };
let only = [];
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--viewport') {
    const [w, h] = args[++i].split('x').map(Number);
    viewport = { width: w, height: h };
  } else if (args[i] === '--only') {
    only.push(args[++i]);
  }
}
const positions = [['top', 0], ['mid', 0.5], ['bottom', 1]];

(async () => {
  const manifest = JSON.parse(fs.readFileSync(path.join(corpus, 'manifest.json'), 'utf8'));
  const browser = await chromium.launch({ args: ['--disable-gpu', '--hide-scrollbars', '--font-render-hinting=none'] });
  fs.mkdirSync(refs, { recursive: true });
  fs.writeFileSync(path.join(refs, 'VERSION'), `chromium ${browser.version()}\nviewport ${viewport.width}x${viewport.height} dpr 1\n`);
  const context = await browser.newContext({ viewport, deviceScaleFactor: 1, javaScriptEnabled: false, colorScheme: 'light' });
  for (const entry of manifest.pages) {
    if (only.length && !only.includes(entry.name)) continue;
    const page = await context.newPage();
    await page.goto('file://' + path.join(corpus, entry.file), { waitUntil: 'load' });
    for (const [name, frac] of positions) {
      // page.evaluate works with JS disabled for page scripts (it runs via CDP).
      await page.evaluate((f) => {
        const max = Math.max(0, document.documentElement.scrollHeight - window.innerHeight);
        window.scrollTo(0, Math.round(f * max));
      }, frac);
      await page.screenshot({ path: path.join(refs, `${entry.name}-${name}.png`), fullPage: false });
    }
    console.log(entry.name);
    await page.close();
  }
  await browser.close();
})().catch((e) => { console.error(e); process.exit(1); });
