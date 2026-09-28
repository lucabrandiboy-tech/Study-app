import type { Difficulty } from '../lib/store';
import type { Question, Topic } from './types';
import { mc, pick, shuffle } from './rand';

/** [question, correct answer, wrong answers, hint, difficulty 0-2 (optional)] */
export type BankQ = [string, string, string[], string, number?];

export function bankTopic(t: {
  id: string; title: string; lesson: string; example: Topic['example']; vocab: Topic['vocab'];
  questions: BankQ[]; gens?: ((d: Difficulty) => Question)[];
}): Topic {
  return {
    id: t.id, title: t.title, lesson: t.lesson, example: t.example, vocab: t.vocab,
    generate: (d) => {
      if (t.gens?.length && Math.random() < 0.45) return pick(t.gens)(d);
      const pool = t.questions.filter((q) => (q[4] ?? 0) <= d + 1);
      // Mix vocab-definition questions in, so every topic has plenty of variety.
      if (t.vocab.length >= 4 && Math.random() < 0.3) {
        const v = pick(t.vocab);
        const flip = Math.random() < 0.5;
        const others = shuffle(t.vocab.filter((x) => x !== v)).slice(0, d === 0 ? 2 : 3);
        return {
          prompt: flip ? `Which term matches this definition?\n"${v.def}"` : `What does "${v.term}" mean?`,
          answer: mc(flip ? v.term : v.def, others.map((o) => (flip ? o.term : o.def))),
          hints: ['Think back to the lesson vocabulary for this topic.', `Key idea: ${flip ? 'look for a word in the definition that connects to the term.' : 'break the term into parts you know.'}`],
          explanation: `${v.term}: ${v.def}`,
        };
      }
      const [q, a, wrong, hint] = pick(pool.length ? pool : t.questions);
      const choice = mc(a, shuffle(wrong).slice(0, d === 0 ? 2 : 3));
      const elim = choice.choices.find((c, i) => i !== choice.correct);
      return {
        prompt: q,
        answer: choice,
        hints: [hint, `You can rule out "${elim}".`],
        explanation: `Answer: ${a}. ${hint}`,
      };
    },
  };
}
