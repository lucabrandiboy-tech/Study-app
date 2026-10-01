# Study + Piano

A study and piano-practice app for an 8th grader: full courses for each subject, a homework calendar, an A+ Plan, a search bar (app content, Wikipedia look-ups and picture search, all shown inside the app), and a piano course with a 1,000+ song library. Dark UI. It runs in the browser, on phones and computers, with no API key needed.

## Stack
React 18 + Vite + TypeScript (strict, `noUnusedLocals`), Tailwind 3, react-router (HashRouter in the single-file build), Tone.js (piano sound + generated smooth-jazz menu music), OpenSheetMusicDisplay (sheet music), @tonejs/midi, Recharts, qrcode-generator.

## Commands
- `npm run dev:web`: dev server (port 5173)
- `npx tsc -b`: typecheck (run before every commit)
- `npm run check`: validates every study question generator, every song's bar lengths, and a few search results (run before every commit)
- `npm run build:single`: one self-contained HTML file in `dist-single/index.html`. Copy it to `study-piano-app.html` (gitignored) to hand to the user.
- `npm run build:pwa`: installable phone app in `dist-pwa/` (manifest, icons, offline service worker)

## Where things live
- `src/lib/store.ts`: all app state (localStorage, `useSyncExternalStore`). Includes settings, topics, streak, homework, grades, tests, spaced review, piano progress. `normalize()` fills new fields on load, so add defaults to `fresh()`.
- `src/study/`: subjects and topics. `subjects.ts` builds each course from units; `bank.ts` (`bankTopic`) turns question banks into generators; `QuestionView.tsx` renders questions; `APlan.tsx` (daily review, tests, grades); `Calendar.tsx` (homework; unfinished homework due tomorrow or earlier blocks that day's streak).
- `src/piano/`: `course.ts` (units and lessons), `songs.ts` and `library/*` (song library; `make.ts` holds the compact song format, `popGen.ts` generates the original pop songs, `preAdvanced.ts` holds the hand-written pieces), `engine.ts` (play modes), `SimplyPlayer.tsx` (full-screen player), `importer.ts` (MusicXML/.mxl/MIDI import), `input.ts` (MIDI keyboard + computer keys).
- `src/pages/Search.tsx`: Search page (`/search`, sidebar box, 🔍 on phones, Ctrl+K). Results open in a preview on the page instead of navigating away. `src/lib/search.ts` indexes the app's content and the student's own homework/notes/decks (local only); `src/lib/wiki.ts` is the "Look it up" and "Pictures" part: Wikipedia's and Wikimedia Commons' public APIs (no key, `origin=*` CORS); article text and pictures are shown inside the app, never linked out. Commons has no safe-search, so `unsafeText()` filters picture searches, file names, descriptions and categories (keep it strict: the user is an 8th grader). The teacher can turn it off in Settings with a PIN (`settings.picFilter`, `settings.picPin` hashed). The service worker never caches Wikipedia requests.
- `src/lib/localTutor.ts`: offline rule-based Study Buddy (teaches from lesson data, never gives homework answers).
- `src/lib/cloud.ts`: cloud save when the app runs as a claude.ai artifact (per-user private `db` storage, chunked, newest copy wins). `src/lib/filesave.ts`: save files, downloads, daily auto-backup.
- `src/components/`: Shell (sidebar + phone top bar/drawer), GeoTools (notepad + TI-84 style calculator on study pages), Reminders (popup on open), PhoneSetup (install QR code).
- `src/index.css`: phone layout rules (`@media (max-width: 767px)`).

## Conventions
- Use `ask()` from `src/lib/embed.ts` instead of `confirm()`. Pop-up dialogs are blocked when the app runs inside the claude.ai viewer.
- Songs use the token format `C4:q`, `[C4 E4]:h`, `r:e`, `~` ties, `.` dots, `t` triplets; `|` separates bars. `npm run check` catches bad bar lengths.
- Study never solves homework: lessons, worked examples, generated practice, hints.

## Publishing
- Pushing to `claude/vigilant-tesla-ja2vck` (the default branch) runs `.github/workflows/pages.yml`, which publishes the installable app to https://lucabrandiboy-tech.github.io/Study-app/
- The claude.ai version (with cloud sync) is https://claude.ai/artifact/SKAmGM25aGdXY6jge3JyGg. It was published from an earlier session, so a new session publishes a new artifact from `study-piano-app.html` with capabilities `{db: {}, user: {}, downloads: true}`.
