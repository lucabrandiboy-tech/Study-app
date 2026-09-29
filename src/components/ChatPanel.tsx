import { useEffect, useRef, useState } from 'react';
import { useApp, useUi, openChat, closeChat, clearChatDraft, setChat, ChatMsg, getState } from '../lib/store';
import { Rich } from './ui';
import { localReply } from '../lib/localTutor';

const MODES = [
  { id: 'explain', label: '💡 Explain it' }, { id: 'hint', label: '🔎 Give me a hint' }, { id: 'check', label: '✅ Check my work' },
  { id: 'quiz', label: '❓ Quiz me' }, { id: 'differently', label: '🔄 Explain it differently' }, { id: 'piano', label: '🎹 Piano help' },
] as const;
type Mode = (typeof MODES)[number]['id'];
const STARTERS: Record<Mode, string> = {
  explain: 'Can you explain the concept on this page in simple words?', hint: 'Can I get a hint for the next step?', check: 'Here are my steps: ',
  quiz: 'Quiz me on this topic!', differently: "I'm still confused. Can you explain it a different way?", piano: 'Based on my recent scores, what should I practice next?',
};

function progressSummary(subject: string) {
  const s = getState();
  if (subject === 'piano') {
    const songs = Object.entries(s.piano.songs).slice(-5).map(([id, r]) => `${id}: ${r.stars}★ ${r.accuracy}%${r.missed?.length ? ` (missed: ${r.missed.slice(0, 3).join('; ')})` : ''}`).join(' | ');
    const lessons = Object.entries(s.piano.lessons).slice(-8).map(([id, st]) => `${id}:${st}★`).join(', ');
    return `Highest unlocked piano unit: ${s.piano.unlockedUnit}. Recent lessons: ${lessons || 'none'}. Recent songs: ${songs || 'none'}.`;
  }
  const weak = Object.entries(s.topics).filter(([, t]) => t.attempts >= 3).map(([id, t]) => `${id} ${Math.round((t.correct / t.attempts) * 100)}%`).slice(-8).join(', ');
  return weak ? `Topic accuracy: ${weak}.` : '';
}

export function ChatButton() {
  const open = useUi((u) => u.chatOpen);
  if (open) return null;
  return (
    <button onClick={() => openChat()} className="fixed bottom-6 right-6 z-40 w-16 h-16 rounded-full bg-accent text-3xl shadow-[0_0_24px_rgba(169,112,255,.8)] border-2 border-edge hover:scale-105 transition" title="Ask Study Buddy">🤖</button>
  );
}

export function ChatPanel() {
  const open = useUi((u) => u.chatOpen);
  const ctx = useUi((u) => u.context);
  const draft = useUi((u) => u.chatDraft);
  const history = useApp((s) => s.chats[ctx.subject]);
  const msgs: ChatMsg[] = history ?? [];
  const [mode, setMode] = useState<Mode>(ctx.subject === 'piano' ? 'piano' : 'hint');
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<'unknown' | 'ok' | 'no-key' | 'offline'>('unknown');
  const [err, setErr] = useState<string | null>(null);
  const [useCloud, setUseCloud] = useState(false);
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    fetch('/api/status').then((r) => r.json()).then((j) => setStatus(j.configured ? 'ok' : 'no-key')).catch(() => setStatus('offline'));
  }, [open]);
  useEffect(() => { if (draft) { setInput(draft); clearChatDraft(); } }, [draft]);
  useEffect(() => { if (ctx.subject === 'piano') setMode('piano'); }, [ctx.subject]);
  useEffect(() => { scroller.current?.scrollTo({ top: 1e9, behavior: 'smooth' }); }, [msgs.length, busy, open]);

  const send = async (text = input) => {
    if (!text.trim() || busy) return;
    const next: ChatMsg[] = [...msgs, { role: 'user', content: text.trim() }];
    setChat(ctx.subject, next);
    setInput(''); setBusy(true); setErr(null);
    if (!useCloud || status !== 'ok') {
      await new Promise((r) => setTimeout(r, 350));
      let reply: string;
      try { reply = localReply(text, mode, ctx); } catch (e) { reply = `Oops, I got confused (${(e as Error).message}). Try asking another way!`; }
      setChat(ctx.subject, [...next, { role: 'assistant', content: reply }]);
      setBusy(false);
      return;
    }
    try {
      const r = await fetch('/api/chat', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ messages: next, mode, context: { ...ctx, progress: progressSummary(ctx.subject) } }) });
      const j = await r.json();
      if (!r.ok) { if (j.error === 'no-key') setStatus('no-key'); throw new Error(j.error ?? 'Request failed'); }
      setChat(ctx.subject, [...next, { role: 'assistant', content: j.reply }]);
    } catch (e) {
      setErr((e as Error).message === 'no-key' ? null : `Couldn't reach Study Buddy: ${(e as Error).message}`);
    }
    setBusy(false);
  };

  if (!open) return null;
  return (
    <aside className="fixed right-0 top-0 bottom-0 w-[420px] z-40 bg-card2 border-l-2 border-edge shadow-[-10px_0_40px_rgba(0,0,0,.5)] flex flex-col animate-[pop_.25s_ease-out]">
      <div className="p-4 border-b border-edge/30">
        <div className="flex items-center justify-between">
          <div className="font-extrabold text-lg">🤖 Study Buddy</div>
          <div className="flex gap-1">
            {msgs.length > 0 && <button className="btn-ghost py-1 px-2 text-xs" onClick={() => setChat(ctx.subject, [])} title="Clear this subject's chat">Clear</button>}
            <button className="btn-ghost py-1 px-2" onClick={closeChat} aria-label="Close">✕</button>
          </div>
        </div>
        <div className="text-xs muted mt-1">📍 {ctx.label}{ctx.subject !== 'general' ? ` · chat saved under "${ctx.subject}"` : ''}</div>
        <div className="flex flex-wrap gap-1 mt-3">
          {MODES.map((m) => (
            <button key={m.id} onClick={() => { setMode(m.id); if (!input) setInput(STARTERS[m.id]); }} className={`text-xs px-2 py-1 rounded-lg border ${mode === m.id ? 'bg-accent border-accent' : 'border-edge/40 hover:border-edge'}`}>{m.label}</button>
          ))}
        </div>
      </div>
      <div ref={scroller} className="flex-1 overflow-auto p-4 space-y-3">
        {status === 'ok' && (
          <label className="flex items-center gap-2 text-xs muted"><input type="checkbox" checked={useCloud} onChange={(e) => setUseCloud(e.target.checked)} /> Use Claude AI (API key found on the helper server)</label>
        )}
        {msgs.length === 0 ? (
          <div className="muted text-sm space-y-2">
            <p>Hi! I'm your Study Buddy. I can explain ideas, give hints, check your work, and quiz you — <b className="text-ink">no internet or API key needed</b>.</p>
            <p>Try: <i>"explain this"</i>, <i>"quiz me"</i>, <i>"what is a ratio"</i>, or pick a button above.</p>
            <p>I <b className="text-ink">won't give you final answers</b> — I'll help you figure them out yourself. You've got this! 💪</p>
          </div>
        ) : null}
        {msgs.map((m, i) => (
          <div key={i} className={`rounded-2xl px-3 py-2 text-sm ${m.role === 'user' ? 'bg-accent/30 ml-8 border border-accent/40' : 'bg-card mr-4 border border-edge/30'}`}>
            {m.role === 'assistant' ? <Rich text={m.content} /> : <div className="whitespace-pre-wrap">{m.content}</div>}
          </div>
        ))}
        {busy && <div className="bg-card mr-4 border border-edge/30 rounded-2xl px-3 py-2 text-sm muted animate-pulse">Thinking…</div>}
        {err && <div className="text-bad text-sm">{err}</div>}
      </div>
      <div className="p-3 border-t border-edge/30">
        <textarea className="input w-full h-24 text-sm resize-none" placeholder="Type your question… (Enter to send, Shift+Enter for new line)" value={input}
          onChange={(e) => setInput(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); void send(); } }} />
        <div className="flex justify-between items-center mt-2">
          <span className="text-xs muted">Mode: {MODES.find((m) => m.id === mode)?.label}</span>
          <button className="btn py-1.5" disabled={busy || !input.trim()} onClick={() => send()}>Send</button>
        </div>
      </div>
    </aside>
  );
}
