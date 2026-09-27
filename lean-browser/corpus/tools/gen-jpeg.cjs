// Generates corpus/img/photo.jpg (64x48 baseline JPEG) with the Playwright
// Chromium's screenshot encoder, since no JPEG encoder is available in the
// build environment's stdlib. CommonJS so NODE_PATH can point at a global
// playwright install; run it like make-refs.cjs (see that file).
const { chromium } = require('playwright');
const path = require('node:path');

(async () => {
  const browser = await chromium.launch({ args: ['--disable-gpu'] });
  const page = await browser.newPage({ viewport: { width: 64, height: 48 }, deviceScaleFactor: 1 });
  await page.setContent(
    '<body style="margin:0"><div style="width:64px;height:48px;background:linear-gradient(135deg,#f4b860,#c04848 60%,#2b2d42)"></div></body>'
  );
  const out = path.join(__dirname, '..', 'img', 'photo.jpg');
  await page.screenshot({ path: out, type: 'jpeg', quality: 60 });
  await browser.close();
  console.log(out);
})();
