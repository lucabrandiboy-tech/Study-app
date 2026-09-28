import type { Subject, Topic } from './types';
import { GEOMETRY_TOPICS } from './geometry';
import { SCIENCE } from './science';
import { ENGLISH, HISTORY, SPANISH, FRENCH } from './humanities';

export function getSubjects(language: 'spanish' | 'french'): Subject[] {
  return [
    { id: 'geometry', name: 'Geometry', icon: '📐', color: '#A970FF', blurb: 'High school geometry — 16 units', topics: GEOMETRY_TOPICS },
    { id: 'science', name: 'Science', icon: '🔬', color: '#4ADE80', blurb: '8th grade physical, earth & life science', topics: SCIENCE },
    { id: 'english', name: 'English / ELA', icon: '📚', color: '#7FD3FF', blurb: 'Grammar, vocab, reading, writing', topics: ENGLISH },
    { id: 'history', name: 'History & Civics', icon: '🏛️', color: '#FF9F43', blurb: 'Colonies through Reconstruction, civics, geography', topics: HISTORY },
    { id: 'language', name: language === 'spanish' ? 'Spanish' : 'French', icon: language === 'spanish' ? '🇪🇸' : '🇫🇷', color: '#F472B6', blurb: 'Vocab, phrases, grammar (change language in Settings)', topics: language === 'spanish' ? SPANISH : FRENCH },
  ];
}

export function findTopic(subjects: Subject[], topicId: string): { subject: Subject; topic: Topic; index: number } | null {
  for (const s of subjects) {
    const i = s.topics.findIndex((t) => t.id === topicId);
    if (i >= 0) return { subject: s, topic: s.topics[i], index: i };
  }
  return null;
}
