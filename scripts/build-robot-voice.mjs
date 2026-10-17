// Builds the separate Robot Voice app (robot-voice/) into dist-pwa/robot-voice/. Run after build-pwa.mjs, which empties dist-pwa/.
// robot-voice/page.html is written for the claude.ai artifact viewer (no <html>/<head>); here it gets its own page shell.
import fs from 'node:fs';
import { build } from 'esbuild';

const out = 'dist-pwa/robot-voice';
fs.mkdirSync(out, { recursive: true });

// eSpeak's compiled code has a few Latin-1 characters in its comments; read it as Latin-1 so the bundle is valid UTF-8.
const latin1 = { name: 'latin1', setup(b) { b.onLoad({ filter: /ESpeak\.js$/ }, (a) => ({ contents: fs.readFileSync(a.path, 'latin1'), loader: 'js' })); } };
await build({
  entryPoints: ['robot-voice/voice-worker.cjs'], bundle: true, format: 'iife', platform: 'browser', outfile: `${out}/voice-worker.js`,
  plugins: [latin1], logLevel: 'warning',
  banner: { js: '/* Robot Voice speech engine. Contains eSpeak 1.43 (Jonathan Duddington, https://espeak.sourceforge.net/), speak.js (Alon Zakai, https://github.com/kripken/speak.js) and meSpeak.js (Norbert Landsteiner, https://www.masswerk.at/mespeak/). All free software under the GNU General Public License v3: https://www.gnu.org/licenses/gpl-3.0.html */' },
});

const page = fs.readFileSync('robot-voice/page.html', 'utf8');
const split = page.indexOf('<main');
const icon = 'data:image/svg+xml,' + encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100"><text y=".9em" font-size="90">🤖</text></svg>');
fs.writeFileSync(`${out}/index.html`, `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<link rel="icon" href="${icon}">
<style>:root{color-scheme:light;padding-top:env(safe-area-inset-top,0px);padding-bottom:env(safe-area-inset-bottom,0px)}body{margin:0;font-size:14px;font-family:system-ui,sans-serif}img{max-width:100%}[hidden]{display:none!important}</style>
${page.slice(0, split).trim()}
</head>
<body>
${page.slice(split).trim()}
</body>
</html>
`);
console.log(`Robot Voice ready in ${out}/`);
