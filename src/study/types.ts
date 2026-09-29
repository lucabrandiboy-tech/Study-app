import type { ReactNode } from 'react';
import type { Difficulty } from '../lib/store';

export type Answer =
  | { kind: 'number'; value: number; unit?: string; mistakes?: { value: number; msg: string }[] }
  | { kind: 'choice'; choices: string[]; correct: number; why?: string[] }
  | { kind: 'point'; x: number; y: number; grid?: { min: number; max: number }; shown?: { x: number; y: number; label: string }[] }
  | { kind: 'proof'; steps: { statement: string; reason: string }[]; reasonBank: string[]; given?: string[] }
  | { kind: 'text'; accept: string[] };

export interface Question {
  prompt: string;
  diagram?: ReactNode;
  answer: Answer;
  hints: string[]; // one step at a time, never the final answer
  explanation: string; // shown after answering (full solution)
}

export interface WorkedExample {
  problem: string;
  diagram?: ReactNode;
  steps: { step: string; why: string }[];
}

export interface Topic {
  id: string;
  title: string;
  lesson: string; // **bold** + newlines
  lessonDiagram?: ReactNode;
  example: WorkedExample;
  generate: (d: Difficulty) => Question;
  vocab: { term: string; def: string }[];
  formulas?: { name: string; f: string }[];
}

export interface Subject {
  id: string;
  name: string;
  icon: string;
  color: string;
  blurb: string;
  topics: Topic[];
  advanced?: boolean; // Super Advanced section (one grade ahead)
  units?: { title: string; topicIds: string[] }[]; // course units, in order
}
