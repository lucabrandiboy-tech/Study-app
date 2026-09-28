import type { Subject, Topic } from './types';
import { GEOMETRY_TOPICS } from './geometry';
import { SCIENCE } from './science';
import { ENGLISH, HISTORY, SPANISH, FRENCH } from './humanities';
import { ADV_MATH, ADV_SCI, ADV_ENG, ADV_HIST, advLanguage } from './advanced';

export function getSubjects(language: 'spanish' | 'french'): Subject[] {
  return [
    { id: 'geometry', name: 'Geometry', icon: '📐', color: '#A970FF', blurb: 'High school geometry — 16 units', topics: GEOMETRY_TOPICS },
    { id: 'science', name: 'Science', icon: '🔬', color: '#4ADE80', blurb: '8th grade physical, earth & life science', topics: SCIENCE },
    { id: 'english', name: 'English / ELA', icon: '📚', color: '#7FD3FF', blurb: 'Grammar, vocab, reading, writing', topics: ENGLISH },
    { id: 'history', name: 'History & Civics', icon: '🏛️', color: '#FF9F43', blurb: 'Colonies through Reconstruction, civics, geography', topics: HISTORY },
    { id: 'language', name: language === 'spanish' ? 'Spanish' : 'French', icon: language === 'spanish' ? '🇪🇸' : '🇫🇷', color: '#F472B6', blurb: 'Vocab, phrases, grammar (change language in Settings)', topics: language === 'spanish' ? SPANISH : FRENCH },
    // ---- Super Advanced: one grade ahead ----
    { id: 'adv-math', name: 'Algebra II & Pre-Calc', icon: '🧮', color: '#A970FF', blurb: 'Quadratics, systems, exponentials, logs, functions, sequences', topics: ADV_MATH, advanced: true },
    { id: 'adv-science', name: 'Biology, Chemistry & Physics', icon: '🧬', color: '#4ADE80', blurb: '9th grade lab sciences with real calculations', topics: ADV_SCI, advanced: true },
    { id: 'adv-english', name: 'English 9', icon: '🖋️', color: '#7FD3FF', blurb: 'Rhetoric, literary analysis, advanced grammar', topics: ADV_ENG, advanced: true },
    { id: 'adv-history', name: 'World History & Economics', icon: '🌍', color: '#FF9F43', blurb: 'Ancient civilizations to the Cold War, plus economics', topics: ADV_HIST, advanced: true },
    { id: 'adv-language', name: language === 'spanish' ? 'Spanish II' : 'French II', icon: language === 'spanish' ? '🇪🇸' : '🇫🇷', color: '#F472B6', blurb: 'Past, future and conditional tenses', topics: advLanguage(language), advanced: true },
  ];
}

export function findTopic(subjects: Subject[], topicId: string): { subject: Subject; topic: Topic; index: number } | null {
  for (const s of subjects) {
    const i = s.topics.findIndex((t) => t.id === topicId);
    if (i >= 0) return { subject: s, topic: s.topics[i], index: i };
  }
  return null;
}
