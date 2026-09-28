export interface BadgeDef { id: string; name: string; icon: string; desc: string }
export const BADGES: BadgeDef[] = [
  { id: 'first-lesson', name: 'First Lesson', icon: '🎓', desc: 'Finish your first study session or piano lesson' },
  { id: 'streak-7', name: '7-Day Streak', icon: '🔥', desc: 'Practice 7 days in a row' },
  { id: 'streak-30', name: '30-Day Streak', icon: '🌋', desc: 'Practice 30 days in a row' },
  { id: 'streak-100', name: '100-Day Streak', icon: '☄️', desc: 'Practice 100 days in a row' },
  { id: 'geometry-master', name: 'Geometry Master', icon: '📐', desc: 'Master all 16 geometry units' },
  { id: 'perfect-quiz', name: 'Perfect Quiz', icon: '💯', desc: 'Score 100% on a quiz' },
  { id: 'first-mastery', name: 'Topic Tamer', icon: '🏅', desc: 'Master your first topic' },
  { id: 'both-hands', name: 'Both Hands Hero', icon: '🙌', desc: 'Pass the Both Hands Together unit test' },
  { id: 'scale-master', name: 'Scale Master', icon: '🎼', desc: 'Pass the Major Scales and Minor Scales unit tests' },
  { id: 'chord-crusher', name: 'Chord Crusher', icon: '🎹', desc: 'Pass the Chords and Chord Progressions unit tests' },
  { id: 'first-song', name: 'First Song', icon: '🎵', desc: 'Finish a song in the Song Player' },
  { id: 'focus-4', name: 'Deep Focus', icon: '🍅', desc: 'Finish 4 focus rounds in one day' },
  { id: 'advanced-pianist', name: 'Advanced Pianist', icon: '👑', desc: 'Pass the Final Exam of the piano course' },
];
