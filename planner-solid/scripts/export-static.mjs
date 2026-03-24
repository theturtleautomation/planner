import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";

const root = process.cwd();
const clientDir = path.join(root, "dist", "client");
const staticDir = path.join(root, "dist", "static");
const manifestPath = path.join(clientDir, ".vite", "manifest.json");

const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const entry = manifest["src/entry-client.tsx"];

if (!entry?.file) {
  throw new Error("Solid client manifest is missing src/entry-client.tsx");
}

const cssLinks = (entry.css ?? [])
  .map((href) => `    <link rel="stylesheet" href="/${href}">`)
  .join("\n");

const modulePreloads = (entry.imports ?? [])
  .map((key) => manifest[key]?.file)
  .filter(Boolean)
  .map((href) => `    <link rel="modulepreload" href="/${href}">`)
  .join("\n");

const html = `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Planner</title>
    <link rel="icon" href="/favicon.ico">
${cssLinks}
${modulePreloads}
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/${entry.file}"></script>
  </body>
</html>
`;

await rm(staticDir, { recursive: true, force: true });
await mkdir(staticDir, { recursive: true });
await cp(clientDir, staticDir, { recursive: true });
await writeFile(path.join(staticDir, "index.html"), html, "utf8");
