import { useSyncExternalStore } from 'react';
import { addDays, dayKey, daysBetween } from './date';
import { BADGES } from './badges';

export type Difficulty = 0 | 1 | 2 | 3; // Easy, Medium, Hard, Challenge
export const DIFF_NAMES = ['Easy', 'Medium', 'Hard', 'Challenge'];

export interface TopicStats {
  attempts: number;
  correct: number;
  inARow: number;
  masteredLevel: number; // -1 = none, else highest difficulty mastered
  difficulty: Difficulty;
}
export interface DayActivity { study: number; piano: number; studyMin: number; pianoMin: number; xp: number; focusRounds: number }
export interface ChatMsg { role: 'user' | 'assistant'; content: string }
export interface Deck { id: string; name: string; cards: { front: string; back: string }[] }
export interface Recording { id: string; name: string; date: string; events: { t: number; midi: number; on: boolean; vel: number }[] }

export interface Mistake { id: string; date: string; topicId: string; topic: string; prompt: string; given: string; correct: string; why: string; solution: string; reviewed?: boolean }

export interface AppState {
  version: 1;
  xp: number;
  days: Record<string, DayActivity>;
  streak: { current: number; best: number; last: string | null; freezes: number; frozenDays: string[] };
  badges: Record<string, string>;
  topics: Record<string, TopicStats>;
  quizzes: { date: string; subject: string; score: number; total: number }[];
  piano: {
    lessons: Record<string, number>; // lessonId -> best stars
    unitTests: Record<string, number>;
    unlockedUnit: number; // highest unlocked unit number
    placementDone: boolean;
    songs: Record<string, { stars: number; accuracy: number; plays: number; missed?: string[]; date: string }>;
    noteGameNPM: { date: string; npm: number }[];
  };
  settings: {
    goalStudy: number; goalPiano: number;
    focusWork: number; focusShort: number; focusLong: number;
    volume: number; metronome: 'click' | 'wood' | 'beep';
    language: 'spanish' | 'french';
    name: string;
  };
  notes: Record<string, string>;
  decks: Deck[];
  recordings: Recording[];
  lastStudy: { path: string; label: string } | null;
  lastPiano: { path: string; label: string } | null;
  chats: Record<string, ChatMsg[]>;
  mistakes: Mistake[];
}

const KEY = 'study-piano-app-v1';

const fresh = (): AppState => ({
  version: 1,
  xp: 0,
  days: {},
  streak: { current: 0, best: 0, last: null, freezes: 0, frozenDays: [] },
  badges: {},
  topics: {},
  quizzes: [],
  piano: { lessons: {}, unitTests: {}, unlockedUnit: 1, placementDone: false, songs: {}, noteGameNPM: [] },
  settings: { goalStudy: 1, goalPiano: 1, focusWork: 25, focusShort: 5, focusLong: 15, volume: 0.8, metronome: 'click', language: 'spanish', name: '' },
  notes: {},
  decks: [],
  recordings: [],
  lastStudy: null,
  lastPiano: null,
  chats: {},
  mistakes: [],
});

/** Fill in any fields missing from older saves. */
function normalize(parsed: Partial<AppState>): AppState {
  const base = fresh();
  return { ...base, ...parsed, piano: { ...base.piano, ...parsed.piano }, settings: { ...base.settings, ...parsed.settings }, streak: { ...base.streak, ...parsed.streak } } as AppState;
}

function load(): AppState {
  try {
    const raw = localStorage.getItem(KEY);
    return raw ? normalize(JSON.parse(raw)) : fresh();
  } catch {
    return fresh();
  }
}

let state: AppState = load();
const listeners = new Set<() => void>();

// ---- transient (not saved) UI state ----
export interface Celebration { id: number; title: string; subtitle?: string; icon: string }
export interface PageContext { label: string; detail?: string; subject: string }
interface UiState { celebrations: Celebration[]; context: PageContext; chatOpen: boolean; chatDraft: string | null }
let ui: UiState = { celebrations: [], context: { label: 'Home', subject: 'general' }, chatOpen: false, chatDraft: null };
const uiListeners = new Set<() => void>();

function persist() {
  try { localStorage.setItem(KEY, JSON.stringify(state)); } catch { /* storage full or blocked */ }
}
function emit() { persist(); listeners.forEach((l) => l()); }
function update(fn: (s: AppState) => AppState) { state = fn(state); emit(); }

export const getState = () => state;
export function useApp<T>(sel: (s: AppState) => T): T {
  return useSyncExternalStore((l) => { listeners.add(l); return () => listeners.delete(l); }, () => sel(state));
}
export function useUi<T>(sel: (u: UiState) => T): T {
  return useSyncExternalStore((l) => { uiListeners.add(l); return () => uiListeners.delete(l); }, () => sel(ui));
}
function setUi(p: Partial<UiState>) { ui = { ...ui, ...p }; uiListeners.forEach((l) => l()); }

let celebId = 1;
export function celebrate(title: string, icon: string, subtitle?: string) {
  setUi({ celebrations: [...ui.celebrations, { id: celebId++, title, icon, subtitle }] });
}
export function dismissCelebration(id: number) { setUi({ celebrations: ui.celebrations.filter((c) => c.id !== id) }); }
export function setPageContext(c: PageContext) {
  if (c.label === ui.context.label && c.detail === ui.context.detail) return;
  setUi({ context: c });
}
export function openChat(draft?: string) { setUi({ chatOpen: true, chatDraft: draft ?? null }); }
export function closeChat() { setUi({ chatOpen: false }); }
export function clearChatDraft() { setUi({ chatDraft: null }); }

// ---- levels ----
export const xpForLevel = (lvl: number) => 100 + (lvl - 1) * 50; // xp needed to go from lvl to lvl+1
export function levelInfo(xp: number) {
  let lvl = 1, rest = xp;
  while (rest >= xpForLevel(lvl)) { rest -= xpForLevel(lvl); lvl++; }
  return { level: lvl, into: rest, need: xpForLevel(lvl) };
}

// ---- streak ----
/** Streak as displayed right now (drops to 0 if a day was missed without enough freezes). */
export function liveStreak(s: AppState = state) {
  const { last, current, freezes } = s.streak;
  if (!last) return 0;
  const gap = daysBetween(last, dayKey());
  if (gap <= 1) return current;
  return gap - 1 <= freezes ? current : 0;
}

const emptyDay = (): DayActivity => ({ study: 0, piano: 0, studyMin: 0, pianoMin: 0, xp: 0, focusRounds: 0 });

function awardBadgeIn(s: AppState, id: string): AppState {
  if (s.badges[id]) return s;
  const b = BADGES.find((x) => x.id === id);
  if (b) setTimeout(() => celebrate(`Badge unlocked: ${b.name}`, b.icon, b.desc), 50);
  return { ...s, badges: { ...s.badges, [id]: dayKey() } };
}
export function awardBadge(id: string) { update((s) => awardBadgeIn(s, id)); }

function addXpIn(s: AppState, n: number): AppState {
  const before = levelInfo(s.xp).level;
  const today = dayKey();
  const d = s.days[today] ?? emptyDay();
  const next = { ...s, xp: s.xp + n, days: { ...s.days, [today]: { ...d, xp: d.xp + n } } };
  const after = levelInfo(next.xp).level;
  if (after > before) setTimeout(() => celebrate(`Level ${after}!`, '⭐', 'Your XP bar filled up. Keep going!'), 50);
  return next;
}
export function addXp(n: number) { if (n > 0) update((s) => addXpIn(s, n)); }

/** Records a completed study session or piano lesson; updates streak, freezes, and milestone badges. */
export function recordActivity(kind: 'study' | 'piano') {
  update((s0) => {
    let s = s0;
    const today = dayKey();
    const d = s.days[today] ?? emptyDay();
    s = { ...s, days: { ...s.days, [today]: { ...d, [kind]: d[kind] + 1 } } };
    s = awardBadgeIn(s, 'first-lesson');
    const st = { ...s.streak, frozenDays: [...s.streak.frozenDays] };
    if (st.last === today) return { ...s, streak: st };
    const gap = st.last ? daysBetween(st.last, today) : Infinity;
    if (gap === 1) st.current += 1;
    else if (gap !== Infinity && gap - 1 <= st.freezes) {
      for (let i = 1; i < gap; i++) st.frozenDays.push(addDays(st.last!, i));
      st.freezes -= gap - 1;
      st.current += 1;
      setTimeout(() => celebrate('Streak freeze used!', '🧊', `Your streak was protected for ${gap - 1} missed day${gap > 2 ? 's' : ''}.`), 50);
    } else st.current = 1;
    st.last = today;
    st.best = Math.max(st.best, st.current);
    let bonus = 0;
    if (st.current % 7 === 0 && st.freezes < 2) {
      st.freezes += 1;
      setTimeout(() => celebrate('Streak freeze earned!', '🧊', 'You earned a freeze for your 7-day run (max 2).'), 80);
    }
    if (st.current === 7) bonus = 50;
    if (st.current === 30) bonus = 200;
    if (st.current === 100) bonus = 500;
    else if (st.current % 10 === 0) bonus = Math.max(bonus, 25);
    setTimeout(() => celebrate(`${st.current}-day streak!`, '🔥', bonus ? `+${bonus} bonus XP` : 'You practiced today. Nice!'), 20);
    s = { ...s, streak: st };
    if (bonus) s = addXpIn(s, bonus);
    if (st.current >= 7) s = awardBadgeIn(s, 'streak-7');
    if (st.current >= 30) s = awardBadgeIn(s, 'streak-30');
    if (st.current >= 100) s = awardBadgeIn(s, 'streak-100');
    return s;
  });
}

export function addMinutes(kind: 'study' | 'piano', min: number) {
  update((s) => {
    const today = dayKey();
    const d = s.days[today] ?? emptyDay();
    const k = kind === 'study' ? 'studyMin' : 'pianoMin';
    return { ...s, days: { ...s.days, [today]: { ...d, [k]: d[k] + min } } };
  });
}
export function addFocusRound() {
  update((s) => {
    const today = dayKey();
    const d = s.days[today] ?? emptyDay();
    const next = { ...s, days: { ...s.days, [today]: { ...d, focusRounds: d.focusRounds + 1 } } };
    return d.focusRounds + 1 >= 4 ? awardBadgeIn(next, 'focus-4') : next;
  });
}

// ---- study topics ----
export const topicStats = (id: string): TopicStats => state.topics[id] ?? { attempts: 0, correct: 0, inARow: 0, masteredLevel: -1, difficulty: 0 };

/** Returns true if this answer just mastered the current difficulty. */
export function recordAnswer(topicId: string, correct: boolean, allGeometryIds: string[]): boolean {
  let justMastered = false;
  update((s0) => {
    const t = { ...(s0.topics[topicId] ?? topicStats(topicId)) };
    t.attempts++;
    if (correct) { t.correct++; t.inARow++; } else t.inARow = 0;
    if (t.inARow >= 5 && t.masteredLevel < t.difficulty) {
      t.masteredLevel = t.difficulty;
      t.inARow = 0;
      justMastered = true;
    }
    let s: AppState = { ...s0, topics: { ...s0.topics, [topicId]: t } };
    s = addXpIn(s, correct ? 10 + t.difficulty * 5 : 0);
    if (justMastered) {
      s = addXpIn(s, 50);
      s = awardBadgeIn(s, 'first-mastery');
      if (allGeometryIds.every((id) => (s.topics[id]?.masteredLevel ?? -1) >= 0)) s = awardBadgeIn(s, 'geometry-master');
    }
    return s;
  });
  return justMastered;
}
export function setTopicDifficulty(topicId: string, d: Difficulty) {
  update((s) => ({ ...s, topics: { ...s.topics, [topicId]: { ...topicStats(topicId), ...s.topics[topicId], difficulty: d, inARow: 0 } } }));
}
export function recordQuiz(subject: string, score: number, total: number) {
  update((s0) => {
    let s = { ...s0, quizzes: [...s0.quizzes, { date: dayKey(), subject, score, total }] };
    s = addXpIn(s, score * 5 + (score === total ? 50 : 0));
    if (score === total) s = awardBadgeIn(s, 'perfect-quiz');
    return s;
  });
}

// ---- piano ----
export function recordLesson(lessonId: string, stars: number) {
  update((s) => {
    const prev = s.piano.lessons[lessonId] ?? 0;
    const gained = Math.max(0, stars - prev) * 15 + 10;
    return addXpIn({ ...s, piano: { ...s.piano, lessons: { ...s.piano.lessons, [lessonId]: Math.max(prev, stars) } } }, gained + (stars === 3 ? 20 : 0));
  });
}
export function recordUnitTest(unit: number, stars: number, totalUnits: number) {
  update((s0) => {
    const prev = s0.piano.unitTests[unit] ?? 0;
    let s: AppState = { ...s0, piano: { ...s0.piano, unitTests: { ...s0.piano.unitTests, [unit]: Math.max(prev, stars) } } };
    if (stars >= 2) {
      s = { ...s, piano: { ...s.piano, unlockedUnit: Math.max(s.piano.unlockedUnit, Math.min(unit + 1, totalUnits)) } };
      s = addXpIn(s, 60 + (stars === 3 ? 30 : 0));
      const passed = (u: number) => (s.piano.unitTests[u] ?? 0) >= 2;
      if (passed(7)) s = awardBadgeIn(s, 'both-hands');
      if (passed(10) && passed(11)) s = awardBadgeIn(s, 'scale-master');
      if (passed(12) && passed(13)) s = awardBadgeIn(s, 'chord-crusher');
      if (unit === totalUnits) s = awardBadgeIn(s, 'advanced-pianist');
    }
    return s;
  });
}
export function setPlacement(unlockedUnit: number) {
  update((s) => ({ ...s, piano: { ...s.piano, placementDone: true, unlockedUnit: Math.max(s.piano.unlockedUnit, unlockedUnit) } }));
}
export function recordSong(songId: string, stars: number, accuracy: number, missed: string[]) {
  update((s0) => {
    const prev = s0.piano.songs[songId];
    let s: AppState = {
      ...s0,
      piano: { ...s0.piano, songs: { ...s0.piano.songs, [songId]: { stars: Math.max(prev?.stars ?? 0, stars), accuracy: Math.max(prev?.accuracy ?? 0, accuracy), plays: (prev?.plays ?? 0) + 1, missed, date: dayKey() } } },
    };
    s = addXpIn(s, 20 + stars * 10 + (accuracy >= 100 ? 30 : 0));
    return awardBadgeIn(s, 'first-song');
  });
}
export function recordNoteGame(npm: number) {
  update((s) => addXpIn({ ...s, piano: { ...s.piano, noteGameNPM: [...s.piano.noteGameNPM, { date: dayKey(), npm }] } }, Math.round(npm / 2)));
}

// ---- misc ----
export function setSettings(p: Partial<AppState['settings']>) { update((s) => ({ ...s, settings: { ...s.settings, ...p } })); }
export function setNote(key: string, text: string) { update((s) => ({ ...s, notes: { ...s.notes, [key]: text } })); }
export function setDecks(decks: Deck[]) { update((s) => ({ ...s, decks })); }
export function setRecordings(recordings: Recording[]) { update((s) => ({ ...s, recordings })); }
export function setLast(kind: 'study' | 'piano', path: string, label: string) {
  const cur = kind === 'study' ? state.lastStudy : state.lastPiano;
  if (cur?.path === path) return;
  update((s) => (kind === 'study' ? { ...s, lastStudy: { path, label } } : { ...s, lastPiano: { path, label } }));
}
export function setChat(subject: string, msgs: ChatMsg[]) { update((s) => ({ ...s, chats: { ...s.chats, [subject]: msgs.slice(-80) } })); }
export function recordMistake(m: Omit<Mistake, 'id' | 'date'>) {
  update((s) => ({ ...s, mistakes: [{ ...m, id: `m-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`, date: dayKey() }, ...s.mistakes].slice(0, 300) }));
}
export function setMistakes(mistakes: Mistake[]) { update((s) => ({ ...s, mistakes })); }
export function resetProgress() { state = fresh(); emit(); }

/** Listen for any saved-progress change (used by the save-file auto-saver). */
export function subscribe(fn: () => void) { listeners.add(fn); return () => { listeners.delete(fn); }; }

/** Replace all progress with data loaded from a save file. Throws if it isn't a save file. */
export function replaceState(data: unknown) {
  if (!data || typeof data !== 'object' || (data as AppState).version !== 1 || typeof (data as AppState).xp !== 'number') {
    throw new Error("This doesn't look like a Study + Piano save file.");
  }
  state = normalize(data as AppState);
  emit();
}
