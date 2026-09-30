// In-app search: one index over pages, courses, topics, vocab, piano lessons, songs and the student's own homework/notes/decks.
// Everything runs locally (no network, no API key).
import type { AppState } from './store';
import type { Subject } from '../study/types';
import { COURSE } from '../piano/course';
import { songLibrary } from '../piano/songs';

export type Kind = 'page' | 'topic' | 'vocab' | 'piano' | 'song' | 'homework' | 'test' | 'note' | 'deck';
export type Group = 'app' | 'study' | 'piano' | 'yours';
export const KIND_GROUP: Record<Kind, Group> = { page: 'app', topic: 'study', vocab: 'study', piano: 'piano', song: 'piano', homework: 'yours', test: 'yours', note: 'yours', deck: 'yours' };
export const KIND_LABEL: Record<Kind, string> = { page: 'Page', topic: 'Lesson', vocab: 'Vocab', piano: 'Piano', song: 'Song', homework: 'Homework', test: 'Test', note: 'Your notes', deck: 'Your deck' };

export interface SearchItem {
  id: string;
  kind: Kind;
  icon: string;
  title: string;
  sub: string; // where it lives, e.g. "Science › Life Science"
  body: string; // longer text: searched, shown as the snippet and in the preview
  keys: string; // extra words to match (vocab terms, composer, category…)
  path: string; // the in-app page "Open" goes to
  ref: string; // id of the underlying thing (topic, song, homework…)
  nt: string; nk: string; nb: string; // normalized copies for matching
}

/** Lowercase, accents removed, anything that isn't a letter or digit becomes a space. Keeps the length, so match positions line up with the original text. */
export function norm(s: string): string {
  let out = '';
  for (let i = 0; i < s.length; i++) {
    let c = s.charCodeAt(i);
    if (c >= 65 && c <= 90) c += 32;
    else if (c >= 128) c = s[i].toLowerCase().normalize('NFD').charCodeAt(0);
    out += (c >= 97 && c <= 122) || (c >= 48 && c <= 57) ? String.fromCharCode(c) : ' ';
  }
  return out;
}
export const queryTerms = (q: string) => [...new Set(norm(q).split(' ').filter(Boolean))];

function item(p: Omit<SearchItem, 'nt' | 'nk' | 'nb'>): SearchItem {
  return { ...p, nt: ` ${norm(p.title)}`, nk: ` ${norm(`${p.sub} ${p.keys}`)}`, nb: ` ${norm(p.body)}` };
}

const PAGES: [string, string, string, string, string][] = [
  ['/', '🏠', 'Home', 'Your streak, daily goals, reminders and where you left off.', 'dashboard start today goals streak'],
  ['/plan', '🎯', 'A+ Plan', 'Daily spaced review, prep for upcoming tests, and your grade tracker.', 'grades grade tracker review tests test prep spaced repetition straight'],
  ['/calendar', '📅', 'Homework Calendar', "Add assignments and check them off. Unfinished homework due tomorrow blocks that day's streak.", 'homework assignments due calendar planner'],
  ['/study', '📚', 'Study Zone', 'Full courses for every subject, with units, unit tests and final exams.', 'courses subjects classes school'],
  ['/advanced', '🚀', 'Super Advanced', 'Courses one grade ahead.', 'harder ahead advanced high school'],
  ['/study', '🧮', 'Calculator & notepad', 'The TI-84 style calculator and the notepad open from the buttons at the bottom of every study page.', 'calculator ti 84 graphing notepad scratch'],
  ['/study/mistakes', '🩹', 'Mistakes', 'Every question you missed, with the full solution, to review.', 'wrong errors missed review'],
  ['/study/flashcards', '🃏', 'Flashcards', 'Auto-made vocab decks for every topic, plus decks you make.', 'cards vocab deck memorize'],
  ['/study/quiz', '❓', 'Practice Quiz', '10 mixed questions, then a review of anything you missed.', 'quiz test yourself practice'],
  ['/study/timer', '🍅', 'Focus Timer', 'A Pomodoro timer: work, short break, long break.', 'pomodoro focus timer break'],
  ['/study/notes', '📝', 'Notes', 'A notes page for every subject and unit.', 'notes notebook write'],
  ['/study/essay', '🖋️', 'Essay Coach', 'Write an essay draft and get feedback on it.', 'essay writing draft paragraph feedback'],
  ['/piano', '🎹', 'Piano Course', 'Piano lessons in units, from first notes to advanced pieces.', 'piano lessons course units'],
  ['/practice', '🎯', 'Practice Games', 'Warm-ups and note-reading games.', 'games note reading warm up'],
  ['/songs', '🎼', 'Song Player', 'Over 1,000 songs to play along with, plus MusicXML / MIDI import.', 'songs music play along import midi musicxml library'],
  ['/sheets', '📜', 'Sheet Music', 'Public-domain sheet music, from first songs to concert pieces.', 'sheet music score print'],
  ['/free', '🎶', 'Free Play', 'Play anything, record yourself and listen back.', 'free play record keyboard'],
  ['/progress', '📈', 'Progress', 'Charts, stats and badges.', 'stats charts badges xp level'],
  ['/settings', '⚙️', 'Settings', 'Your name and goals, sound, MIDI keyboard, focus timer and more.', 'settings options midi volume language backup save install phone'],
];

/** Everything built into the app. Rebuild when the language setting changes. */
export function staticIndex(subjects: Subject[]): SearchItem[] {
  const out: SearchItem[] = PAGES.map(([path, icon, title, body, keys], i) => item({ id: `page:${i}`, kind: 'page', icon, title, sub: 'App page', body, keys, path, ref: path }));
  for (const s of subjects) {
    const unitOf = (tid: string) => s.units?.find((u) => u.topicIds.includes(tid))?.title;
    out.push(item({ id: `course:${s.id}`, kind: 'page', icon: s.icon, title: s.name, sub: s.advanced ? 'Super Advanced course' : 'Course', body: [s.blurb, ...(s.units ?? []).map((u, i) => `Unit ${i + 1}: ${u.title}`)].join('\n'), keys: '', path: `/study/${s.id}`, ref: s.id }));
    for (const t of s.topics) {
      const unit = unitOf(t.id);
      const ref = `${s.id}/${t.id}`;
      out.push(item({
        id: `topic:${ref}`, kind: 'topic', icon: s.icon, title: t.title, sub: unit ? `${s.name} › ${unit}` : s.name,
        body: `${t.lesson}\nExample: ${t.example.problem}`, keys: [...t.vocab.map((v) => v.term), ...(t.formulas ?? []).map((f) => `${f.name} ${f.f}`)].join(' '),
        path: `/study/${ref}`, ref,
      }));
      t.vocab.forEach((v, i) => out.push(item({ id: `vocab:${ref}:${i}`, kind: 'vocab', icon: '🔤', title: v.term, sub: `${t.title} · ${s.name}`, body: v.def, keys: '', path: `/study/${ref}`, ref })));
    }
  }
  for (const u of COURSE) {
    const name = u.exam ? 'Final Exam' : `Unit ${u.n}: ${u.title}`;
    out.push(item({ id: `unit:${u.n}`, kind: 'piano', icon: '🎹', title: name, sub: `Piano Course · ${u.level}`, body: u.lessons.map((l, i) => `${i + 1}. ${l.title}`).join('\n'), keys: u.song?.title ?? '', path: `/piano/unit/${u.n}`, ref: `unit:${u.n}` }));
    for (const l of u.lessons) out.push(item({ id: `lesson:${l.id}`, kind: 'piano', icon: '🎹', title: l.title, sub: `Piano › ${name}`, body: l.text, keys: '', path: `/piano/lesson/${l.id}`, ref: l.id }));
  }
  for (const s of songLibrary()) out.push(item({ id: `song:${s.id}`, kind: 'song', icon: '🎵', title: s.title, sub: `${s.composer} · ${s.level}`, body: '', keys: s.category, path: `/songs?id=${encodeURIComponent(s.id)}`, ref: s.id }));
  return out;
}

/** The student's own things: homework, upcoming tests, notes, flashcard decks, imported songs. */
export function userIndex(s: Pick<AppState, 'homework' | 'tests' | 'notes' | 'decks' | 'importedSongs'>, subjects: Subject[]): SearchItem[] {
  const out: SearchItem[] = [];
  const subj = (id: string) => subjects.find((x) => x.id === id);
  for (const h of s.homework) out.push(item({ id: `hw:${h.id}`, kind: 'homework', icon: h.done ? '✅' : '📝', title: h.title, sub: `${h.cls ? `${h.cls} · ` : ''}due ${h.due}${h.done ? ' · done' : ''}`, body: h.notes ?? '', keys: 'homework', path: '/calendar', ref: h.id }));
  for (const t of s.tests) out.push(item({ id: `test:${t.id}`, kind: 'test', icon: '🧪', title: t.title, sub: `${subj(t.subjectId)?.name ?? 'Test'} · ${t.date}`, body: t.topicIds.map((id) => subjects.flatMap((x) => x.topics).find((x) => x.id === id)?.title).filter(Boolean).join(', '), keys: 'test quiz', path: '/plan', ref: t.id }));
  for (const [key, text] of Object.entries(s.notes)) {
    if (!text?.trim()) continue;
    const [sid, tid] = key.split('/');
    const sub = subj(sid);
    const topic = sub?.topics.find((t) => t.id === tid);
    const title = key === 'english/essay-draft' ? 'Essay draft' : topic ? `Notes: ${topic.title}` : `Notes: ${sub?.name ?? sid}`;
    const path = key === 'english/essay-draft' ? '/study/essay' : topic ? `/study/${sid}/${tid}?tab=notes` : '/study/notes';
    out.push(item({ id: `note:${key}`, kind: 'note', icon: '📝', title, sub: sub?.name ?? 'Notes', body: text, keys: 'notes', path, ref: key }));
  }
  for (const d of s.decks) out.push(item({ id: `deck:${d.id}`, kind: 'deck', icon: '🃏', title: d.name, sub: `${d.cards.length} card${d.cards.length === 1 ? '' : 's'}`, body: d.cards.map((c) => `${c.front} — ${c.back}`).join('\n'), keys: 'flashcards deck', path: '/study/flashcards', ref: d.id }));
  for (const m of s.importedSongs) out.push(item({ id: `song:${m.id}`, kind: 'song', icon: '📂', title: m.title, sub: `${m.composer} · Imported`, body: '', keys: 'imported', path: `/songs?id=${encodeURIComponent(m.id)}`, ref: m.id }));
  return out;
}

const KIND_BOOST: Record<Kind, number> = { page: 3, topic: 2, vocab: 1, piano: 1, song: 0, homework: 2, test: 2, note: 2, deck: 1 };

/** Every search word must appear somewhere. Title matches count most, then keywords, then the body. */
export function scoreItem(it: SearchItem, terms: string[], phrase: string): number {
  let score = 0;
  for (const t of terms) {
    const inner = t.length > 1; // one letter only matches the start of a word
    if (it.nt.includes(` ${t} `) || it.nt.endsWith(` ${t}`)) score += 12;
    else if (it.nt.includes(` ${t}`)) score += 8;
    else if (inner && it.nt.includes(t)) score += 5;
    else if (it.nk.includes(` ${t}`)) score += 4;
    else if (inner && it.nk.includes(t)) score += 3;
    else if (it.nb.includes(` ${t}`)) score += 1.5;
    else if (inner && it.nb.includes(t)) score += 1;
    else return 0;
  }
  if (phrase) {
    const title = it.nt.trim().replace(/ +/g, ' ');
    if (title === phrase) score += 30;
    else if (title.startsWith(`${phrase} `)) score += 12;
    else if (title.startsWith(phrase)) score += 4;
    else if (terms.length > 1 && title.includes(phrase)) score += 8;
  }
  return score + KIND_BOOST[it.kind];
}

export function runSearch(items: SearchItem[], q: string): SearchItem[] {
  const terms = queryTerms(q);
  if (!terms.length) return [];
  const phrase = terms.join(' ');
  const hits: { it: SearchItem; s: number }[] = [];
  for (const it of items) { const s = scoreItem(it, terms, phrase); if (s > 0) hits.push({ it, s }); }
  hits.sort((a, b) => b.s - a.s || a.it.title.length - b.it.title.length);
  return hits.map((h) => h.it);
}

/** A short piece of `text` around the first matching word. */
export function snippet(text: string, terms: string[], len = 170): string {
  const plain = text.replace(/\*\*/g, '').replace(/\s+/g, ' ').trim();
  if (plain.length <= len) return plain;
  const n = norm(plain);
  let at = -1;
  for (const t of terms) { const i = wordIndex(n, t); if (i >= 0 && (at < 0 || i < at)) at = i; }
  if (at < 0) for (const t of terms) { const i = n.indexOf(t); if (i >= 0 && (at < 0 || i < at)) at = i; }
  if (at < 0) return `${plain.slice(0, len).trimEnd()}…`;
  const start = Math.max(0, Math.min(at - 50, plain.length - len));
  const cut = plain.slice(start, start + len).trim();
  return `${start > 0 ? '…' : ''}${cut}${start + len < plain.length ? '…' : ''}`;
}

/** Position of the first place in normalized text `n` where a word starts with `t`, or -1. */
function wordIndex(n: string, t: string, from = 0): number {
  for (let i = n.indexOf(t, from); i >= 0; i = n.indexOf(t, i + 1)) if (i === 0 || n[i - 1] === ' ') return i;
  return -1;
}

/** [start, end) ranges of `text` where a word starts with a search word, merged (for highlighting). */
export function matchRanges(text: string, terms: string[]): [number, number][] {
  if (!terms.length) return [];
  const n = norm(text);
  const r: [number, number][] = [];
  for (const t of terms) for (let i = wordIndex(n, t); i >= 0; i = wordIndex(n, t, i + t.length)) r.push([i, i + t.length]);
  r.sort((a, b) => a[0] - b[0]);
  const merged: [number, number][] = [];
  for (const x of r) { const last = merged[merged.length - 1]; if (last && x[0] <= last[1]) last[1] = Math.max(last[1], x[1]); else merged.push([...x]); }
  return merged;
}
