import { useEffect } from 'react';
import { BrowserRouter, HashRouter, Routes, Route, Navigate } from 'react-router-dom';
import { Sidebar, Celebrations } from './components/Shell';
import { ChatButton, ChatPanel } from './components/ChatPanel';
import { useUi } from './lib/store';
import { Home, Progress, Settings } from './pages/pages';
import { StudyHome, SubjectPage, TopicPage, AdvancedHome } from './study/pages';
import { FlashcardsPage, QuizPage, FocusTimer, NotesPage, EssayCoach, MistakesPage } from './study/tools';
import { PianoCourse, UnitPage, LessonPage, FreePlay } from './piano/pages';
import { SongPlayer } from './piano/SongLibrary';
import { PracticeGames } from './piano/games';
import { SheetLibrary } from './piano/SheetLibrary';
import { installComputerKeyboard, connectMidi } from './piano/input';
import { initFileSave } from './lib/filesave';
import { JazzController } from './components/Jazz';

const Router = import.meta.env.VITE_SINGLEFILE ? HashRouter : BrowserRouter;

export default function App() {
  const chatOpen = useUi((u) => u.chatOpen);
  useEffect(() => installComputerKeyboard(), []);
  useEffect(() => { void connectMidi(); void initFileSave(); }, []);
  return (
    <Router>
      <div className="flex min-h-screen">
        <Sidebar />
        <main className={`flex-1 p-8 min-w-0 transition-[margin] ${chatOpen ? 'mr-[420px]' : ''}`}>
          <div className="max-w-[1400px] mx-auto">
            <Routes>
              <Route path="/" element={<Home />} />
              <Route path="/study" element={<StudyHome />} />
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
      <JazzController />
      <ChatButton />
      <ChatPanel />
      <Celebrations />
    </Router>
  );
}
