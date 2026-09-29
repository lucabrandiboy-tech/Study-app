// Offline Study Buddy — a rule-based tutor that runs entirely in the browser.
// No internet, no API key. It teaches from the app's own lessons, examples,
// vocab and question generators, and never hands out homework answers.
import { getState, type PageContext } from './store';
import { getSubjects, findTopic } from '../study/subjects';
import { correctAnswerText } from '../study/QuestionView';
import type { Question, Subject, Topic } from '../study/types';

export type TutorMode = 'explain' | 'hint' | 'check' | 'quiz' | 'differently' | 'piano';

let pendingQuiz: { topic: string; qs: Question[] } | null = null;
let hintStep = 0;
let lastTopicId = '';

const pick = <T,>(a: T[]) => a[Math.floor(Math.random() * a.length)];
const norm = (s: string) => s.toLowerCase().replace(/[^a-z0-9.\-/ ]+/g, ' ').replace(/\s+/g, ' ').trim();
const ENCOURAGE = ['You\'ve got this! 💪', 'Nice thinking — keep going!', 'Step by step, you\'ll get it. 🙌', 'Great question to ask!'];

function subjects(): Subject[] { return getSubjects(getState().settings.language); }

/** The topic the student is looking at, from the URL, or null. */
function currentTopic(): { subject: Subject; topic: Topic } | null {
  const path = decodeURIComponent(window.location.hash || window.location.pathname);
  const m = path.match(/study\/([\w-]+)\/([\w-]+)/);
  if (!m) return null;
  const f = findTopic(subjects(), m[2]);
  return f ? { subject: f.subject, topic: f.topic } : null;
}

/** Best-matching topic for the words the student typed. */
function searchTopic(text: string, subjectId?: string): { subject: Subject; topic: Topic; score: number } | null {
  const STOP = new Set(['explain', 'quiz', 'this', 'that', 'topic', 'hint', 'help', 'about', 'what', 'with', 'please', 'give', 'check', 'work', 'again', 'differently', 'confused', 'still', 'example', 'questions', 'question', 'simple', 'words', 'concept', 'page', 'next', 'step', 'tell', 'answer', 'more', 'some', 'can', 'you', 'the', 'and', 'way', 'different', 'another', 'there', 'here', 'have', 'does', 'mean']);
  const words = norm(text).split(' ').filter((w) => w.length > 2 && !STOP.has(w));
  if (!words.length) return null;
  let best: { subject: Subject; topic: Topic; score: number } | null = null;
  for (const s of subjects()) {
    for (const t of s.topics) {
      const title = norm(t.title), vocab = norm(t.vocab.map((v) => v.term).join(' ')), body = norm(t.lesson);
      let score = s.id === subjectId ? 0.5 : 0;
      for (const w of words) score += (title.includes(w) ? 4 : 0) + (vocab.includes(w) ? 3 : 0) + (body.includes(w) ? 1 : 0);
      if (!best || score > best.score) best = { subject: s, topic: t, score };
    }
  }
  return best && best.score >= 3 ? best : null;
}

/** "What is a ___?" — look the word up in every topic's vocab. */
function define(text: string): string | null {
  const m = norm(text).match(/(?:what(?: is| s| are|'s)|define|meaning of|what does) (?:an? |the )?([a-z0-9 \-]+?)(?: mean)?$/);
  if (!m) return null;
  const term = m[1].trim();
  for (const s of subjects()) for (const t of s.topics) for (const v of t.vocab) {
    const vt = norm(v.term);
    if (vt === term || vt.replace(/^(el|la|los|las|le|les|l) /, '') === term || (term.length > 4 && vt.includes(term))) {
      return `**${v.term}** — ${v.def}\n\n_From ${s.name} › ${t.title}. Open that lesson for examples._`;
    }
  }
  return null;
}

function lessonParas(t: Topic, n = 4) { return t.lesson.split(/\n+/).filter(Boolean).slice(0, n).join('\n\n'); }

function explain(t: Topic) {
  const f = t.formulas?.length ? `\n\n**Key formulas:**\n${t.formulas.map((x) => `• ${x.name}: ${x.f}`).join('\n')}` : '';
  const v = t.vocab.length ? `\n\n**Words to know:** ${t.vocab.slice(0, 5).map((x) => `**${x.term}** (${x.def})`).join(', ')}` : '';
  return `Here's the big idea of **${t.title}**:\n\n${lessonParas(t)}${f}${v}\n\nWant a hint, a worked example (🔄), or a mini quiz (❓)?`;
}

function differently(t: Topic) {
  const ex = t.example;
  const steps = ex.steps.map((s, i) => `${i + 1}. ${s.step}\n   _Why:_ ${s.why}`).join('\n');
  return `Let's look at it through a **worked example** instead (a different problem than yours, so you can copy the method, not the answer):\n\n**${ex.problem}**\n\n${steps}\n\nNow try the same steps on your own problem. Tell me where you get stuck!`;
}

function hint(t: Topic) {
  if (t.id !== lastTopicId) { hintStep = 0; lastTopicId = t.id; }
  const q = t.generate(1);
  const tips = [
    `**Step 1 — Understand it.** What is the question actually asking for? Write it in your own words. Then list what you're *given*.`,
    t.formulas?.length ? `**Step 2 — Pick a tool.** Which of these fits? ${t.formulas.map((f) => `${f.name}: ${f.f}`).join(' · ')}` : `**Step 2 — Recall the lesson.** ${lessonParas(t, 1)}`,
    `**Step 3 — Use the method.** Here's how a similar problem starts: _${t.example.steps[0]?.step ?? ''}_ — ${t.example.steps[0]?.why ?? ''}`,
    `**Step 4 — A hint style that often helps here:** ${q.hints[0] ?? 'Break it into smaller pieces.'}`,
    `**Step 5 — Check it.** Does your answer make sense? Plug it back in, check units, and re-read the question once more.`,
  ];
  const out = tips[Math.min(hintStep, tips.length - 1)];
  hintStep++;
  return `${out}\n\n${hintStep < tips.length ? 'Ask for another hint if you need the next step.' : pick(ENCOURAGE)}`;
}

/** Check arithmetic lines like "3x + 4 = 19" is not checkable, but "12 * 4 = 48" is. */
function check(text: string, t: Topic | null) {
  const lines = text.split(/\n|;/).map((l) => l.trim()).filter(Boolean);
  const results: string[] = [];
  for (const l of lines) {
    const m = l.replace(/×/g, '*').replace(/÷/g, '/').replace(/−/g, '-').match(/^([\d\s.+\-*/()^]+)=\s*(-?[\d.]+)\s*$/);
    if (!m) continue;
    try {
      const expr = m[1].replace(/\^/g, '**');
      if (!/^[\d\s.+\-*/()]+$/.test(expr.replace(/\*\*/g, '*'))) continue;
      const val = Function(`"use strict"; return (${expr});`)() as number;
      const ok = Math.abs(val - Number(m[2])) < 1e-6;
      results.push(ok ? `✅ \`${l}\` — this arithmetic is correct.` : `❌ \`${l}\` — the left side doesn't equal ${m[2]}. Redo this calculation carefully.`);
    } catch { /* not a checkable line */ }
  }
  const checklist = `**Self-check list:**\n• Did you copy the numbers from the question correctly?\n• Did you use the right formula / rule${t ? ` for ${t.title}` : ''}?\n• Order of operations & signs (+/−) OK?\n• Did you include units?\n• Plug your answer back in — does it work?`;
  if (results.length) return `I checked the math in your steps:\n\n${results.join('\n')}\n\n${checklist}`;
  return `I can check arithmetic steps written like \`12 * 4 = 48\` (one per line). I won't grade a final answer to homework, but here's how to check it yourself:\n\n${checklist}`;
}

function formatQ(q: Question, i: number) {
  const a = q.answer;
  let s = `**Q${i + 1}.** ${q.prompt}`;
  if (a.kind === 'choice') s += '\n' + a.choices.map((c, k) => `   ${'ABCDEFG'[k]}) ${c}`).join('\n');
  if (a.kind === 'point') s += '  _(answer like (2, -3))_';
  return s;
}

function startQuiz(t: Topic) {
  const qs: Question[] = [];
  for (let k = 0; k < 12 && qs.length < 3; k++) {
    const q = t.generate(Math.min(2, k % 3) as 0 | 1 | 2);
    if (q.answer.kind !== 'proof' && !q.diagram && !qs.some((x) => x.prompt === q.prompt)) qs.push(q);
  }
  if (!qs.length) return `This topic's questions need diagrams, so use the **Practice** tab for it!`;
  pendingQuiz = { topic: t.title, qs };
  return `Mini quiz on **${t.title}** 🎯\n\n${qs.map(formatQ).join('\n\n')}\n\nReply with your answers in order, separated by commas or new lines (letters are fine for multiple choice).`;
}

function gradeQuiz(text: string) {
  const quiz = pendingQuiz!; pendingQuiz = null;
  const parts = text.split(/\n|,(?![^(]*\))/).map((p) => p.trim().replace(/^(q?\d+[.):]\s*)/i, '')).filter(Boolean);
  let right = 0;
  const lines = quiz.qs.map((q, i) => {
    const given = parts[i] ?? '';
    const a = q.answer;
    let ok = false;
    if (a.kind === 'number') ok = Math.abs(parseFloat(given.replace(/[^\d.\-]/g, '')) - a.value) < 0.011 + Math.abs(a.value) * 0.001;
    else if (a.kind === 'choice') { const L = given.trim().toUpperCase(); ok = (L.length === 1 && 'ABCDEFG'.indexOf(L) === a.correct) || norm(given) === norm(a.choices[a.correct]); }
    else if (a.kind === 'point') { const n = given.match(/-?\d+(\.\d+)?/g)?.map(Number) ?? []; ok = n[0] === a.x && n[1] === a.y; }
    else if (a.kind === 'text') ok = a.accept.some((x) => norm(x) === norm(given));
    if (ok) right++;
    return ok ? `✅ Q${i + 1}: correct!` : `❌ Q${i + 1}: you said "${given || '(blank)'}". Correct: **${correctAnswerText(q)}**\n   _Why:_ ${q.explanation}`;
  });
  return `**Score: ${right}/${quiz.qs.length}** ${right === quiz.qs.length ? '🌟 Perfect!' : ''}\n\n${lines.join('\n\n')}\n\nSay "quiz me" for another round.`;
}

function pianoTips() {
  const s = getState();
  const songs = Object.entries(s.piano.songs);
  const tips: string[] = [];
  const weak = songs.filter(([, r]) => r.accuracy < 80).slice(-3);
  if (weak.length) tips.push(`Your trickiest recent songs: ${weak.map(([id, r]) => `**${id}** (${r.accuracy}%)`).join(', ')}. Replay them in **Wait mode** at 60–70% speed, hands separately first.`);
  const missed = songs.flatMap(([, r]) => r.missed ?? []).slice(-6);
  if (missed.length) tips.push(`Spots you've been missing: ${missed.slice(0, 4).join('; ')}. Loop just those bars 5 times slowly before playing the whole piece.`);
  tips.push(`You're on piano **unit ${s.piano.unlockedUnit}**. Finish its lessons with 3★ to unlock the next.`);
  tips.push(pick([
    'Warm up with a 5-finger scale in C, G and F — 2 minutes each hand.',
    'Count out loud ("1-and-2-and") — rhythm mistakes are the most common ones.',
    'Keep your wrist level and fingers curved like you\'re holding a ball.',
    'Practice little and often: 15 focused minutes beats an hour of rushing.',
    'Play the left hand alone until it feels boring — then add the right hand.',
  ]));
  return `🎹 **Practice plan**\n\n${tips.map((t) => `• ${t}`).join('\n')}`;
}

const HOMEWORK = /\b(answer (to|for)|solve (this|it|for me)|do my|just tell me|give me the answer|what'?s the answer)\b/i;

export function localReply(text: string, mode: TutorMode, ctx: PageContext): string {
  const clean = text.trim();
  if (pendingQuiz && !/quiz|explain|hint|what is|help/i.test(clean)) return gradeQuiz(clean);
  if (mode === 'piano' || ctx.subject === 'piano' && /piano|practice|song|next/i.test(clean)) return pianoTips();

  const def = define(clean);
  if (def && mode !== 'check') return def;

  const here = currentTopic();
  const found = searchTopic(clean, ctx.subject);
  const topic = (found && (!here || found.score >= 4) ? found.topic : here?.topic) ?? null;

  if (HOMEWORK.test(clean)) {
    const lead = `I can't give homework answers — but I can teach you how to get it yourself! 🧠`;
    return topic ? `${lead}\n\n${hint(topic)}` : `${lead}\n\nTell me the **topic** (like "slope" or "cells") and I'll explain the method and give you step-by-step hints.`;
  }
  if (!topic) {
    if (mode === 'check') return check(clean, null);
    return `I'm your offline Study Buddy — I work without internet or an API key. Open any topic in the **Study Zone** and I'll explain it, give step-by-step hints, check your arithmetic, or quiz you.\n\nOr type a topic name, like _"explain photosynthesis"_, _"quiz me on slope"_, or _"what is a noun"_.`;
  }
  if (/quiz/i.test(clean) || mode === 'quiz') return startQuiz(topic);
  if (mode === 'check') return check(clean, topic);
  if (/example|different|confus|don'?t get|still/i.test(clean) || mode === 'differently') return differently(topic);
  if (mode === 'hint' || /hint|stuck|next step|help/i.test(clean)) return hint(topic);
  return explain(topic);
}
