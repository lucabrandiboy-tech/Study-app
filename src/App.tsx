import { useApp, getState } from './lib/store';
import { useEffect } from 'react';
import { BrowserRouter, HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { MobileTopBar, Sidebar, Celebrations } from './components/Shell';
import { ChatButton, ChatPanel } from './components/ChatPanel';
import { useUi } from './lib/store';
import { Home, Progress, Settings } from './pages/pages';
import { StudyHome, SubjectPage, TopicPage, AdvancedHome } from './study/pages';
import { FlashcardsPage, QuizPage, FocusTimer, NotesPage, EssayCoach, MistakesPage } from './study/tools';
import { PianoCourse, UnitPage, LessonPage, FreePlay } from './piano/pages';
import { SongPlayer } from './piano/SongLibrary';
import { PracticeGames } from './piano/games';
import { SheetLibrary } from './piano/SheetLibrary';
import { installComputerKeyboard, connectMidi, setMidiAppSound } from './piano/input';
import { initFileSave, dailyAutoBackup } from './lib/filesave';
import { initCloud } from './lib/cloud';
import { JazzController } from './components/Jazz';
import { GeoTools } from './components/GeoTools';
import { Reminders } from './components/Reminders';
import { APlanPage } from './study/APlan';
import { HomeworkCalendar } from './study/Calendar';
import { SearchPage, SearchHotkey } from './pages/Search';

const Router = import.meta.env.VITE_SINGLEFILE ? HashRouter : BrowserRouter;

export default function App() {
  const chatOpen = useUi((u) => u.chatOpen);
  useEffect(() => installComputerKeyboard(), []);
  useEffect(() => { void connectMidi(); void initFileSave(); void initCloud(); dailyAutoBackup(getState().settings.autoBackup); }, []);
  const midiAppSound = useApp((s) => s.settings.midiAppSound);
  useEffect(() => setMidiAppSound(midiAppSound), [midiAppSound]);
  return (
    <Router>
      <div className="flex min-h-screen">
        <Sidebar />
        <main className={`flex-1 p-3 pb-28 md:p-8 min-w-0 transition-[margin] ${chatOpen ? 'md:mr-[420px]' : ''}`}>
          <MobileTopBar />
          <div className="max-w-[1400px] mx-auto">
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/search" element={<SearchPage />} />
              <Route path="/study" element={<StudyHome />} />
              <Route path="/plan" element={<APlanPage />} />
              <Route path="/calendar" element={<HomeworkCalendar />} />
              <Route path="/advanced" element={<AdvancedHome />} />
              <Route path="/study/mistakes" element={<MistakesPage />} />
              <Route path="/study/flashcards" element={<FlashcardsPage />} />
              <Route path="/study/quiz" element={<QuizPage />} />
              <Route path="/study/timer" element={<FocusTimer />} />
              <Route path="/study/notes" element={<NotesPage />} />
              <Route path="/study/essay" element={<EssayCoach />} />
              <Route path="/study/:subjectId" element={<SubjectPage />} />
              <Route path="/study/:subjectId/:topicId" element={<TopicPage />} />
              <Route path="/piano" element={<PianoCourse />} />
              <Route path="/piano/unit/:n" element={<UnitPage />} />
              <Route path="/piano/lesson/:id" element={<LessonPage />} />
              <Route path="/practice" element={<PracticeGames />} />
              <Route path="/songs" element={<SongPlayer />} />
              <Route path="/sheets" element={<SheetLibrary />} />
              <Route path="/free" element={<FreePlay />} />
              <Route path="/progress" element={<Progress />} />
              <Route path="/settings" element={<Settings />} />
              <Route path="*" element={<Navigate to="/" />} />
            </Routes>
          </div>
        </main>
      </div>
      <SearchHotkey />
      <JazzController />
      <GeoTools />
      <Reminders />
      <ChatButton />
      <ChatPanel />
      <Celebrations />
    </Router>
  );
}
