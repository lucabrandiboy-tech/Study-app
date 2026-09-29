import type { Subject, Topic } from './types';
import { GEOMETRY_TOPICS } from './geometry';
import { SCIENCE } from './science';
import { ENGLISH, HISTORY, SPANISH, FRENCH } from './humanities';
import { SCIENCE2 } from './science2';
import { ENGLISH2 } from './english2';
import { HISTORY2 } from './history2';
import { language2 } from './language2';
import { ADV_MATH, ADV_SCI, ADV_ENG, ADV_HIST, advLanguage } from './advanced';

type U = [string, string[]];
/** Order a subject's topics into named units (topics not listed are appended as a final unit). */
function course(topics: Topic[], units: U[]): { topics: Topic[]; units: Subject['units'] } {
  const byId = new Map(topics.map((t) => [t.id, t]));
  const used = new Set(units.flatMap(([, ids]) => ids));
  const rest = topics.filter((t) => !used.has(t.id)).map((t) => t.id);
  const all: U[] = rest.length ? [...units, ['More Topics', rest]] : units;
  const clean = all.map(([title, ids]) => ({ title, topicIds: ids.filter((id) => byId.has(id)) }));
  return { topics: clean.flatMap((u) => u.topicIds.map((id) => byId.get(id)!)), units: clean };
}

export function getSubjects(language: 'spanish' | 'french'): Subject[] {
  const L = (x: string) => `lang-${language}-${x}`;
  const geo = course(GEOMETRY_TOPICS, [
    ['Foundations & Angles', ['geo-1', 'geo-2', 'geo-3']], ['Logic, Triangles & Proofs', ['geo-4', 'geo-5', 'geo-6', 'geo-7']],
    ['Similarity & Right Triangles', ['geo-8', 'geo-9', 'geo-10']], ['Polygons, Transformations & Coordinates', ['geo-11', 'geo-12', 'geo-13']],
    ['Circles, Area & Volume', ['geo-14', 'geo-15', 'geo-16']]]);
  const sci = course([...SCIENCE, ...SCIENCE2], [
    ['Science Skills & Matter', ['sci-method', 'sci-matter', 'sci-periodic', 'sci-reactions']], ['Physical Science', ['sci-forces', 'sci-energy', 'sci-waves', 'sci-electricity']],
    ['Earth & Space', ['sci-earth', 'sci-weather', 'sci-space', 'sci-resources']], ['Life Science', ['sci-cells', 'sci-body', 'sci-genetics', 'sci-ecosystems']]]);
  const eng = course([...ENGLISH, ...ENGLISH2], [
    ['Grammar & Mechanics', ['eng-grammar', 'eng-speech', 'eng-punctuation', 'eng-vocab']], ['Reading & Literature', ['eng-reading', 'eng-figurative', 'eng-poetry']],
    ['Writing', ['eng-narrative', 'eng-essay', 'eng-argument', 'eng-research']]]);
  const his = course([...HISTORY, ...HISTORY2], [
    ['Before the United States', ['his-native', 'his-exploration', 'his-colonies', 'his-french-indian']], ['A New Nation', ['his-revolution', 'his-constitution', 'his-civics', 'his-courts']],
    ['A Growing Nation', ['his-expansion', 'his-jackson', 'his-industry', 'his-geography']], ['Division & Reunion', ['his-road-war', 'his-civilwar']]]);
  const lang = course([...(language === 'spanish' ? SPANISH : FRENCH), ...language2(language)], [
    ['First Words', [L('greet'), L('numbers'), L('family')]], ['Daily Life', [L('food'), L('colors'), L('weather'), L('body')]],
    ['Around Town', [L('home'), L('town')]], ['Grammar', [L('grammar'), L('verbs')]]]);
  return [
    { id: 'geometry', name: 'Geometry', icon: '📐', color: '#A970FF', blurb: 'High school geometry — 5 units, 16 topics', ...geo },
    { id: 'science', name: 'Science', icon: '🔬', color: '#4ADE80', blurb: '8th grade physical, earth & life science', ...sci },
    { id: 'english', name: 'English / ELA', icon: '📚', color: '#7FD3FF', blurb: 'Grammar, vocab, reading, writing', ...eng },
    { id: 'history', name: 'History & Civics', icon: '🏛️', color: '#FF9F43', blurb: 'Colonies through Reconstruction, civics, geography', ...his },
    { id: 'language', name: language === 'spanish' ? 'Spanish' : 'French', icon: language === 'spanish' ? '🇪🇸' : '🇫🇷', color: '#F472B6', blurb: 'Vocab, phrases, grammar (change language in Settings)', ...lang },
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
