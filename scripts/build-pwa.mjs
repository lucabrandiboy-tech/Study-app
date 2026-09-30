// Builds the installable phone/computer app (PWA) into dist-pwa/ from the single-file build.
import fs from 'node:fs';
const out = 'dist-pwa';
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out);
let html = fs.readFileSync('dist-single/index.html', 'utf8');
const head = [
  '<link rel="manifest" href="./manifest.webmanifest">',
  '<meta name="theme-color" content="#0B1026">',
  '<meta name="mobile-web-app-capable" content="yes">',
  '<meta name="apple-mobile-web-app-capable" content="yes">',
  '<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">',
  '<meta name="apple-mobile-web-app-title" content="Study+Piano">',
  '<link rel="apple-touch-icon" href="./apple-touch-icon.png">',
  '<link rel="icon" type="image/png" href="./icon-192.png">',
].join('\n    ');
html = html.replace('</head>', `    ${head}\n  </head>`).replace('content="width=device-width, initial-scale=1.0"', 'content="width=device-width, initial-scale=1.0, viewport-fit=cover"');
fs.writeFileSync(`${out}/index.html`, html);
for (const f of ['manifest.webmanifest', 'icon-192.png', 'icon-512.png', 'icon-maskable-512.png', 'apple-touch-icon.png']) fs.copyFileSync(`pwa/${f}`, `${out}/${f}`);
fs.writeFileSync(`${out}/sw.js`, fs.readFileSync('pwa/sw.js', 'utf8').replace('__VERSION__', Date.now().toString(36)));
fs.writeFileSync(`${out}/.nojekyll`, '');
console.log(`PWA ready in ${out}/ (${(html.length / 1e6).toFixed(1)} MB page)`);
