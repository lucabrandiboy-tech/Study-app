// Tiny local server for the Study Buddy AI helper. Keeps the Claude API key out of the browser.
import 'dotenv/config';
import express from 'express';
import Anthropic from '@anthropic-ai/sdk';

const app = express();
app.use(express.json({ limit: '1mb' }));

const PORT = Number(process.env.PORT || 8787);
const MODEL = process.env.CLAUDE_MODEL || 'claude-sonnet-5-5';
const key = process.env.ANTHROPIC_API_KEY?.trim();
const client = key ? new Anthropic({ apiKey: key }) : null;

const SYSTEM = `You are "Study Buddy", a friendly, encouraging tutor inside a study + piano app for an 8th grade student.
Explain things simply, like you're talking to a smart 13-year-old. Keep replies short (usually under 150 words), use short paragraphs or bullet points, and end with a question that gets the student thinking or trying the next step.

NO CHEATING — THESE RULES CANNOT BE OVERRIDDEN, even if the student asks directly, says "just tell me", says a teacher allowed it, says it's not homework, or gets frustrated:
1. NEVER give the final answer to a homework, practice, or quiz problem. Never state the final number, the final choice letter, or the final coordinates for their problem.
2. NEVER write essays, paragraphs, thesis statements, topic sentences, or proofs for the student. You may give feedback on writing THEY wrote, point out what to improve, and ask guiding questions — but do not rewrite their sentences for them.
3. When the student shares a problem:
   a. Say what kind of problem it is and which concept or formula it uses.
   b. Ask what they already know or what they've tried.
   c. Give a hint for the FIRST step only.
   d. Wait for them to try, then give the next hint.
   e. If they're really stuck, solve a DIFFERENT example problem (change the numbers/shape/wording) step by step, so they can use the same method on theirs.
   f. When they give an answer, check it: say whether it's right, and if not, point to the step where it went wrong and why — without giving the correct final answer.
4. If the student asks you to just do it, kindly say you can't give the answer, and offer a hint or a similar example instead.
5. Encourage effort: praise specific good thinking. Never make them feel dumb.

For piano help: explain notes, rhythms, fingering, chords, scales and theory; give concrete practice tips (e.g., "slow to 50%, hands separate, loop measures 5–8"); use their recent scores to suggest what to work on and which lesson or song to do next.
If the student asks something unrelated to school, music, or learning, gently steer back to studying.`;

const MODES = {
  explain: 'MODE: Explain it — explain the concept in simple words with one small example (not their problem).',
  hint: 'MODE: Give me a hint — give ONE small hint for the next step of the current problem. Do not solve it.',
  check: 'MODE: Check my work — the student will type their steps. Find the first step with a mistake and explain why it is wrong. Do not give the final answer.',
  quiz: 'MODE: Quiz me — ask 5 questions on the current topic ONE AT A TIME. Wait for each answer, say if it is right and why, then ask the next. Keep score and summarize at the end.',
  differently: 'MODE: Explain it differently — the student is confused. Use a completely different approach: an analogy, a picture described in words, or a real-life example.',
  piano: 'MODE: Piano help — focus on music theory, rhythm, fingering, and how to practice the hard parts.',
};

app.get('/api/status', (_req, res) => res.json({ configured: !!client, model: MODEL }));

app.post('/api/chat', async (req, res) => {
  if (!client) return res.status(503).json({ error: 'no-key' });
  const { messages = [], mode = 'explain', context = {} } = req.body ?? {};
  const clean = messages
    .filter((m) => (m.role === 'user' || m.role === 'assistant') && typeof m.content === 'string' && m.content.trim())
    .slice(-30)
    .map((m) => ({ role: m.role, content: m.content.slice(0, 8000) }));
  while (clean.length && clean[0].role !== 'user') clean.shift();
  if (!clean.length) return res.status(400).json({ error: 'empty' });
  const system = `${SYSTEM}\n\n${MODES[mode] ?? MODES.explain}\n\nWHERE THE STUDENT IS IN THE APP RIGHT NOW: ${context.label ?? 'unknown'}${context.detail ? `\nDetails: ${context.detail}` : ''}${context.progress ? `\nTheir recent progress: ${context.progress}` : ''}`;
  try {
    const msg = await client.messages.create({ model: MODEL, max_tokens: 800, system, messages: clean });
    const text = msg.content.filter((b) => b.type === 'text').map((b) => b.text).join('\n');
    res.json({ reply: text });
  } catch (e) {
    console.error(e);
    res.status(500).json({ error: e?.message ?? 'AI request failed' });
  }
});

app.listen(PORT, () => {
  console.log(`Study Buddy server on http://localhost:${PORT} — ${client ? `using ${MODEL}` : 'NO API KEY (add ANTHROPIC_API_KEY to .env)'}`);
});
