# Study + Piano

A dark-mode desktop web app for an 8th grader with two zones — **Study Zone** (learn, never cheat) and **Piano Zone** — plus streaks, XP, badges, a progress page, and an AI tutor ("Study Buddy") that gives hints instead of answers.

## Run it

```bash
npm install
npm run dev
```

For a single downloadable `index.html` (no install needed), run `npm run build:single`. The file is written to `dist-single/`.

Open **http://localhost:5173** in **Chrome or Edge** on a PC. `npm run dev` starts both the website (Vite) and the small AI helper server (`server/`).

### Turn on Study Buddy (optional)

1. Get a Claude API key at console.anthropic.com.
2. Copy `.env.example` to `.env` and paste the key after `ANTHROPIC_API_KEY=`.
3. Restart `npm run dev`.

The key stays in the local server. It's never sent to the browser, and `.env` is git-ignored. Without a key, the chat panel shows setup steps and the rest of the app still works.

### Connect a piano

Plug in a USB MIDI keyboard, then open **Settings → Detect keyboard**. The sustain pedal works too. If you don't have a keyboard, you can play with the computer keys:

- **A S D F G H J K** play the white keys.
- **W E T Y U O P** play the black keys.
- **Z / X** move down or up an octave.
- Hold **Shift** for the sustain pedal.

## What's inside

| Area | Features |
| --- | --- |
| Home | Greeting, streak flame + freezes, daily goal ring, continue cards, weekly XP chart, recent badges, quick-start buttons |
| Streaks & XP | Daily streak (study session OR piano lesson), streak freezes (1 per 7 days, max 2), calendar, XP levels, 13 badges, celebration animations |
| Study Zone | Geometry (16 units), Science (8), English (5 + Essay Coach), History/Civics (7), Spanish or French (5). Every topic follows lesson → worked example → practice with random new questions → one-at-a-time hints → check with mistake explanations → mastery (5 in a row unlocks the next difficulty: Easy/Medium/Hard/Challenge) |
| Geometry tools | Labeled SVG diagrams, clickable coordinate grid, proof reason-picker, formula sheet, vocab flashcards |
| Study tools | Flashcards (auto + custom decks, flip, shuffle, missed cards repeat), 10-question quizzes (timed/untimed, full review), Pomodoro focus timer, notes per subject/unit |
| Piano course | 24 units (Beginner → Advanced) + Final Exam, 4–7 lessons each, practice song, unit test (2★ to unlock), placement test, course map. Each lesson has 5 steps: learn (keyboard animation), demo, wait-for-me practice, scored performance (1–3★), and feedback on missed notes and timing |
| Extra practice | Daily warm-ups, note-reading speed game (notes/min), rhythm tapping, ear training (intervals & chords) |
| Song Player | Public-domain library by level, MusicXML/MIDI import, sheet view with a moving cursor or falling-notes view, colored left/right-hand keys, tempo 25–150%, metronome, loop measures, wait-for-me, RH/LH/both, score screen |
| Free Play | Sampled grand piano (Salamander), recording + playback, metronome, pedal |
| Progress | Streak calendar, XP history, accuracy by subject and topic, weak spots, minutes per week, piano stars per lesson, songs, note-game speed, badge collection |
| Settings | MIDI setup/test, daily goals, timer lengths, volume, metronome sound, language, AI status, reset (type RESET to confirm) |

### Saving progress to a file

Progress is always kept in the browser, and it can also be saved to a real file on your computer. Go to **Settings → Save file** and choose one:

- **Create new save file** (Chrome/Edge): pick where to save it, for example Documents or a USB stick. Every change after that is saved to the file automatically. After you restart the browser, click **Reconnect save file** in the sidebar and the app loads your progress from the file.
- **Open existing save file**: load progress from a file you saved earlier, for example on another computer, and keep saving to it.
- **Export / Import backup** (any browser): download a `.json` copy of your progress, or load one back in.

## Project layout

```
src/lib/         store (progress, streaks, XP, badges), dates, hooks
src/study/       subjects, question generators, diagrams, study pages & tools
src/piano/       audio (Tone.js), MIDI + computer-keys input, notation → MusicXML,
                 play-along engine, sheet/falling views, course, songs, games
src/pages/       Home, Progress, Settings
src/components/  UI kit, sidebar, celebrations, Study Buddy chat panel
server/          Express + Claude API (no-cheating system prompt)
scripts/         check-content.ts: checks every question generator and piano piece
```

`npm run typecheck` type-checks the project. `npx tsx scripts/check-content.ts` generates thousands of questions and checks every course piece's bar lengths.
