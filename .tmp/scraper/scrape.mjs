import { chromium } from 'playwright-core';
import fs from 'fs';
import path from 'path';

const routes = [
  '/',
  '/projects',
  '/sessions',
  '/blueprint',
  '/knowledge',
  '/events',
  '/admin',
  '/discovery'
];

const baseUrl = 'http://localhost:3100';
const outDir = path.join(process.cwd(), 'aura_html_export');

if (!fs.existsSync(outDir)) {
  fs.mkdirSync(outDir, { recursive: true });
}

(async () => {
  console.log('Starting browser...');
  const browser = await chromium.launch({
    executablePath: '/usr/bin/chromium',
    headless: true
  });
  
  const context = await browser.newContext({
    viewport: { width: 1280, height: 1024 }
  });
  const page = await context.newPage();
  
  for (const route of routes) {
    console.log(`Scraping ${route}...`);
    try {
      await page.goto(`${baseUrl}${route}`, { waitUntil: 'networkidle', timeout: 15000 });
      
      // Wait to ensure React has fully mounted and fetched basic data
      // For a more robust scrape, we might look for a specific element
      await page.waitForTimeout(2500); 
      
      const html = await page.content();
      
      const filename = route === '/' ? 'index.html' : `${route.substring(1).replace(/\//g, '_')}.html`;
      fs.writeFileSync(path.join(outDir, filename), html);
      console.log(`Saved ${filename}`);
    } catch (err) {
      console.error(`Failed to scrape ${route}:`, err.message);
    }
  }
  
  await browser.close();
  console.log('Done scraping!');
})();
