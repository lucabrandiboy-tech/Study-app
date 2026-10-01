// Sanity checks: every generator produces valid questions; every course piece parses and lines up with its bars.
import { getSubjects } from '../src/study/subjects';
import { COURSE, lessonPiece } from '../src/piano/course';
import { songLibrary } from '../src/piano/songs';
import { measureLen, toMusicXML } from '../src/piano/notation';
import { staticIndex, runSearch, norm } from '../src/lib/search';
import { parseExtract, stripHtml, parseCommons, parseLeadImages, unsafeText } from '../src/lib/wiki';

let problems = 0;
const fail = (m: string) => { problems++; console.log('✗', m); };

for (const lang of ['spanish', 'french'] as const) {
  for (const s of getSubjects(lang)) for (const t of s.topics) for (const d of [0, 1, 2, 3] as const) {
    for (let i = 0; i < 150; i++) {
      let q;
      try { q = t.generate(d); } catch (e) { fail(`${t.id} d${d} threw ${(e as Error).message}`); break; }
      const a = q.answer;
      if (!q.prompt || q.hints.length === 0 || !q.explanation) fail(`${t.id} missing prompt/hints/explanation`);
      if (a.kind === 'number' && !Number.isFinite(a.value)) fail(`${t.id} d${d} NaN answer: ${q.prompt}`);
      if (a.kind === 'choice' && (a.correct < 0 || new Set(a.choices).size !== a.choices.length || a.choices.length < 2)) fail(`${t.id} d${d} bad choices: ${JSON.stringify(a.choices)} ${q.prompt}`);
      if (a.kind === 'point' && (!Number.isInteger(a.x) || !Number.isInteger(a.y))) fail(`${t.id} bad point`);
      if (a.kind === 'number' && q.hints.some((h) => h.includes(`= ${a.value}.`))) fail(`${t.id} hint may reveal answer: ${q.hints.join(' / ')}`);
    }
  }
}
const pieces = [
  ...COURSE.flatMap((u) => [...u.lessons.map((l) => ({ id: l.id, ex: l.ex })), ...(u.song ? [{ id: `u${u.n}-song`, ex: u.song.ex }] : []), ...(u.test ? [{ id: `u${u.n}-test`, ex: u.test }] : [])]),
];
const lib = songLibrary();
for (const s of lib) if (s.ex) pieces.push({ id: `song:${s.id}`, ex: s.ex });
for (const p of pieces) {
  try {
    const pc = lessonPiece(p.id, p.id, p.ex);
    if (!pc.events.length) fail(`${p.id} has no notes`);
    const ml = measureLen(pc);
    const lens: Record<string, number> = {};
    for (const [h, src] of [['R', p.ex.rh], ['L', p.ex.lh]] as const) {
      if (!src) continue;
      // bar check: each "|" segment must be a whole measure
      const segs = src.split('|');
      let dur = 1, total = 0;
      segs.forEach((seg, i) => {
        let len = 0;
        const toks = seg.replace(/\[[^\]]*\]/g, 'X').trim().split(/\s+/).filter(Boolean);
        for (const tok of toks) {
          const d = tok.replace(/~$/, '').split(':')[1];
          if (d) { const base = { w: 4, h: 2, q: 1, e: 0.5, s: 0.25, z: 0.125 }[d[0] as 'w']; dur = base * (d.includes('.') ? 1.5 : 1) * (d.includes('t') ? 2 / 3 : 1); }
          len += dur;
        }
        total += len;
        if (segs.length > 1 && Math.abs(len / ml - Math.round(len / ml)) > 1e-6) fail(`${p.id} ${h} bar ${i + 1} has ${len} beats, expected ${ml}`);
      });
      lens[h] = total;
    }
    if (lens.R !== undefined && lens.L !== undefined && Math.abs(lens.R - lens.L) > 1e-6) fail(`${p.id} RH ${lens.R} beats vs LH ${lens.L}`);
    const xml = toMusicXML(pc);
    if (!xml.includes('<measure')) fail(`${p.id} no xml`);
  } catch (e) { fail(`${p.id} threw ${(e as Error).message}`); }
}
for (const s of songLibrary()) { try { s.piece(); } catch (e) { fail(`song ${s.id}: ${(e as Error).message}`); } }

// Search: unique result ids, sensible top results, and the Wikipedia text cleanup.
for (const lang of ['spanish', 'french'] as const) {
  const seen = new Set<string>();
  for (const it of staticIndex(getSubjects(lang))) { if (seen.has(it.id)) fail(`search: duplicate id ${it.id}`); seen.add(it.id); }
}
const index = staticIndex(getSubjects('spanish'));
for (const [q, want] of [['pythagorean theorem', 'Pythagorean Theorem'], ['moon', 'Earth, Moon & Space'], ['fur elise', 'Für Elise'], ['calendar', 'Homework Calendar'], ['esta', 'estar']]) {
  const got = runSearch(index, q)[0]?.title;
  if (got !== want) fail(`search: "${q}" should find "${want}" first, got "${got}"`);
}
if (norm('Für Élise 𝄞!') !== 'fur elise    ') fail(`search: norm() gave "${norm('Für Élise 𝄞!')}"`);
if (stripHtml('a <span class="searchmatch">cell</span> &quot;x&quot; &amp; &#039;y&#039;') !== 'a cell "x" & \'y\'') fail('wiki: stripHtml');
const secs = parseExtract('Intro text {\\displaystyle a^{2}+b^{2}}.\n\n== History ==\nOld times.\n=== Empty ===\n== References ==\nRef.\n=== More refs ===\nRef.\n== Life ==\nText.');
if (JSON.stringify(secs.map((x) => [x.heading, x.paras])) !== JSON.stringify([['', ['Intro text.']], ['History', ['Old times.']], ['Life', ['Text.']]])) fail(`wiki: parseExtract gave ${JSON.stringify(secs)}`);
// Picture search: safety filter and result parsing (sample API replies; no network needed).
for (const bad of ['nude beach', 'Sexual_content', 'Category:Nudity in art', 'naked', 'corpse']) if (!unsafeText(bad)) fail(`pictures: "${bad}" should be filtered`);
for (const ok of ['Sussex', 'photosynthesis', 'Civil War soldiers', 'Essex county', 'breast cancer awareness ribbon']) if (unsafeText(ok)) fail(`pictures: "${ok}" should not be filtered`);
const commons = parseCommons({ query: { pages: [
  { title: 'File:Leaf_2.jpg', index: 2, categories: [{ title: 'Category:Leaves' }], imageinfo: [{ thumburl: 'https://upload.wikimedia.org/b.jpg', extmetadata: { ImageDescription: { value: 'A <b>green</b> leaf' }, Artist: { value: '<a href="x">Ann</a>' }, LicenseShortName: { value: 'CC BY-SA 4.0' } } }] },
  { title: 'File:Leaf_1.jpg', index: 1, categories: [{ title: 'Category:Plants' }], imageinfo: [{ thumburl: 'https://upload.wikimedia.org/a.jpg' }] },
  { title: 'File:Hidden.jpg', index: 3, categories: [{ title: 'Category:Nude people' }], imageinfo: [{ thumburl: 'https://upload.wikimedia.org/c.jpg' }] },
  { title: 'File:Unchecked.jpg', index: 4, imageinfo: [{ thumburl: 'https://upload.wikimedia.org/d.jpg' }] },
] } });
if (JSON.stringify(commons.map((c) => [c.file, c.caption, c.credit])) !== JSON.stringify([['Leaf_1.jpg', '', ''], ['Leaf_2.jpg', 'A green leaf', 'Ann · CC BY-SA 4.0']])) fail(`pictures: parseCommons gave ${JSON.stringify(commons)}`);
const lead = parseLeadImages({ query: { pages: [{ title: 'Leaf', index: 1, pageimage: 'Leaf_1.jpg', thumbnail: { source: 'https://upload.wikimedia.org/a.jpg' } }, { title: 'No picture', index: 2 }] } }, 'simple');
if (lead.length !== 1 || lead[0].article !== 'Leaf' || lead[0].host !== 'simple.wikipedia.org') fail(`pictures: parseLeadImages gave ${JSON.stringify(lead)}`);
console.log(problems ? `${problems} problem(s)` : `All good: ${pieces.length} pieces (${lib.length} library songs), all topic generators OK.`);
process.exit(problems ? 1 : 0);
