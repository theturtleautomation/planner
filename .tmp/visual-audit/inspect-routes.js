const { chromium } = require('playwright');
const fs = require('fs');
const path = require('path');

const routes = [
  { name: 'root', url: 'http://localhost:5173/' },
  { name: 'knowledge-home', url: 'http://localhost:5173/knowledge' },
  { name: 'knowledge-all', url: 'http://localhost:5173/knowledge/all' },
  { name: 'blueprint', url: 'http://localhost:5173/blueprint' },
  { name: 'session', url: 'http://localhost:5173/session/f633443d-6254-4ed8-a5d3-d7bf517978ab' },
  { name: 'events', url: 'http://localhost:5173/events' },
];

(async() => {
  const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1024 } });
  const out = [];

  for (const route of routes) {
    await page.goto(route.url, { waitUntil: 'networkidle' });
    await page.screenshot({ path: path.join('/home/thetu/planner/.tmp/visual-audit', `${route.name}.png`), fullPage: true });
    const data = await page.evaluate(() => {
      const headings = Array.from(document.querySelectorAll('h1,h2,h3,[role="heading"]')).map((el) => el.textContent?.trim()).filter(Boolean).slice(0, 20);
      const buttons = Array.from(document.querySelectorAll('button')).map((el) => el.textContent?.trim()).filter(Boolean).slice(0, 40);
      const links = Array.from(document.querySelectorAll('a')).map((el) => ({ text: el.textContent?.trim(), href: el.getAttribute('href') })).filter((item) => item.text || item.href).slice(0, 40);
      const labels = Array.from(document.querySelectorAll('label')).map((el) => el.textContent?.trim()).filter(Boolean).slice(0, 40);
      const bodyText = document.body.innerText.replace(/\s+/g, ' ').trim().slice(0, 2000);
      return { title: document.title, headings, buttons, links, labels, bodyText };
    });
    out.push({ route, ...data });
  }

  fs.writeFileSync('/home/thetu/planner/.tmp/visual-audit/routes.json', JSON.stringify(out, null, 2));
  await browser.close();
})();
