import type { Question, Topic } from './types';
import { ri, pick, shuffle, round, fmt, mc, pickLetters } from './rand';
import { Fig, Polygon, Rays, ParallelLines, Grid, CircleFigL, Solid, NumberLine, T, Seg, AngleMark, ORANGE } from './diagrams';

type P = [number, number];
const num = (value: number, extra: Partial<{ unit: string; mistakes: { value: number; msg: string }[] }> = {}) => ({ kind: 'number' as const, value, ...extra });

// ---------- 1. Foundations ----------
const foundations: Topic = {
  id: 'geo-1', title: 'Foundations: Points, Lines, Segments',
  lesson: `A **point** is an exact location. A **line** goes on forever in both directions. A **plane** is a flat surface that goes on forever.
A **segment** is part of a line with two endpoints. A **ray** starts at one endpoint and goes forever in one direction.
The **Segment Addition Postulate**: if B is between A and C, then AB + BC = AC.
The **midpoint** splits a segment into two equal halves. On a number line, the distance between a and b is |a − b|, and the midpoint is (a + b) ÷ 2.`,
  lessonDiagram: <NumberLine min={-2} max={10} points={[{ v: 0, label: 'A' }, { v: 3, label: 'B' }, { v: 8, label: 'C' }]} />,
  example: {
    problem: 'B is between A and C. AB = 2x + 1, BC = x + 4, and AC = 20. Find x.',
    steps: [
      { step: 'AB + BC = AC', why: 'Segment Addition: the two small parts add up to the whole.' },
      { step: '(2x + 1) + (x + 4) = 20', why: 'Substitute the expressions we were given.' },
      { step: '3x + 5 = 20', why: 'Combine like terms: 2x + x = 3x and 1 + 4 = 5.' },
      { step: '3x = 15 → x = 5', why: 'Subtract 5 from both sides, then divide by 3.' },
    ],
  },
  vocab: [
    { term: 'Point', def: 'An exact location with no size, named with a capital letter.' },
    { term: 'Line', def: 'A straight path that extends forever in both directions.' },
    { term: 'Plane', def: 'A flat surface that extends forever in all directions.' },
    { term: 'Segment', def: 'Part of a line with two endpoints.' },
    { term: 'Ray', def: 'Part of a line with one endpoint that extends forever in one direction.' },
    { term: 'Midpoint', def: 'The point that divides a segment into two congruent segments.' },
    { term: 'Collinear', def: 'Points that lie on the same line.' },
    { term: 'Congruent segments', def: 'Segments with the same length.' },
  ],
  formulas: [{ name: 'Distance on a number line', f: '|a − b|' }, { name: 'Midpoint on a number line', f: '(a + b) / 2' }],
  generate: (d) => {
    const type = d === 0 ? pick(['add', 'dist']) : pick(['add', 'algebra', 'mid', 'dist']);
    if (type === 'dist') {
      const a = ri(-8, 4), b = a + ri(3, 12);
      return {
        prompt: `On a number line, point P is at ${a} and point Q is at ${b}. Find PQ.`,
        diagram: <NumberLine min={Math.min(a, 0) - 1} max={b + 1} points={[{ v: a, label: 'P' }, { v: b, label: 'Q' }]} />,
        answer: num(b - a, { mistakes: [{ value: a + b, msg: 'You added the coordinates. Distance uses subtraction: |a − b|.' }] }),
        hints: ['Distance on a number line is the absolute value of the difference of the coordinates.', `Set it up as |${b} − (${a})|. Be careful with the negative sign.`],
        explanation: `PQ = |${b} − (${a})| = ${b - a}.`,
      };
    }
    if (type === 'mid') {
      const a = ri(-10, 6) * 2, b = a + ri(2, 9) * 2;
      return {
        prompt: `M is the midpoint of AB. A is at ${a} and B is at ${b} on a number line. Where is M?`,
        diagram: <NumberLine min={a - 1} max={b + 1} points={[{ v: a, label: 'A' }, { v: b, label: 'B' }]} />,
        answer: num((a + b) / 2),
        hints: ['The midpoint is exactly halfway: average the two coordinates.', `Add ${a} and ${b}, then divide by 2.`],
        explanation: `M = (${a} + ${b}) ÷ 2 = ${(a + b) / 2}.`,
      };
    }
    if (type === 'add') {
      const ab = ri(3, 15), bc = ri(3, 15);
      return {
        prompt: `B is between A and C. AB = ${ab} and AC = ${ab + bc}. Find BC.`,
        diagram: <Fig h={90}><Seg a={[30, 45]} b={[290, 45]} /><T x={30} y={25} bold>A</T><T x={30 + (ab / (ab + bc)) * 260} y={25} bold>B</T><T x={290} y={25} bold>C</T><T x={30 + (ab / (ab + bc)) * 130} y={65} color={ORANGE}>{ab}</T><T x={160} y={80} color={ORANGE}>{`AC = ${ab + bc}`}</T></Fig>,
        answer: num(bc, { mistakes: [{ value: ab + ab + bc, msg: 'You added. BC is a part of AC, so subtract the known part from the whole.' }] }),
        hints: ['Use the Segment Addition Postulate: AB + BC = AC.', `Substitute: ${ab} + BC = ${ab + bc}. What do you do to both sides?`],
        explanation: `AB + BC = AC → ${ab} + BC = ${ab + bc} → BC = ${bc}.`,
      };
    }
    const x = ri(2, 9), a = ri(1, 4), b = ri(-5, 8), c = ri(1, 3), e = ri(0, 9);
    const total = a * x + b + c * x + e;
    if (a * x + b <= 0) return foundations.generate(d);
    return {
      prompt: `B is between A and C. AB = ${a}x ${b >= 0 ? '+ ' + b : '− ' + -b}, BC = ${c}x + ${e}, and AC = ${total}. Find x${d >= 2 ? ', then find AB' : ''}.${d >= 2 ? ' (Enter AB.)' : ''}`,
      answer: num(d >= 2 ? a * x + b : x),
      hints: ['Segment Addition: AB + BC = AC. Write an equation.', `Combine like terms: (${a} + ${c})x + (${b} + ${e}) = ${total}.`, d >= 2 ? 'After you find x, plug it back into the expression for AB.' : 'Now isolate x: subtract the constant, then divide.'],
      explanation: `${a + c}x + ${b + e} = ${total} → ${a + c}x = ${total - b - e} → x = ${x}.${d >= 2 ? ` AB = ${a}(${x}) ${b >= 0 ? '+' : '−'} ${Math.abs(b)} = ${a * x + b}.` : ''}`,
    };
  },
};

// ---------- 2. Angles ----------
const angles: Topic = {
  id: 'geo-2', title: 'Angles & Angle Pairs',
  lesson: `An **angle** is formed by two rays with a common endpoint (the **vertex**). Angles are measured in degrees.
**Acute**: less than 90°. **Right**: exactly 90°. **Obtuse**: between 90° and 180°. **Straight**: 180°.
**Complementary** angles add to **90°**. **Supplementary** angles add to **180°**.
**Vertical angles** are across from each other when two lines cross — they are always **equal**.
**Adjacent** angles share a vertex and a side. An **angle bisector** cuts an angle into two equal angles.`,
  lessonDiagram: <Fig><Rays dirs={[0, 55, 180]} arcs={[{ from: 0, to: 55, label: '55°' }, { from: 55, to: 180, label: '125°', color: ORANGE }]} /></Fig>,
  example: {
    problem: 'Two angles are supplementary. One angle is 3x° and the other is (x + 20)°. Find both angles.',
    steps: [
      { step: '3x + (x + 20) = 180', why: 'Supplementary angles add up to 180°.' },
      { step: '4x + 20 = 180', why: 'Combine like terms.' },
      { step: '4x = 160 → x = 40', why: 'Subtract 20, then divide by 4.' },
      { step: 'Angles: 3(40) = 120° and 40 + 20 = 60°', why: 'Plug x back in. Check: 120 + 60 = 180 ✓.' },
    ],
  },
  vocab: [
    { term: 'Acute angle', def: 'An angle measuring less than 90°.' }, { term: 'Right angle', def: 'An angle measuring exactly 90°.' },
    { term: 'Obtuse angle', def: 'An angle measuring more than 90° but less than 180°.' }, { term: 'Complementary angles', def: 'Two angles whose measures add to 90°.' },
    { term: 'Supplementary angles', def: 'Two angles whose measures add to 180°.' }, { term: 'Vertical angles', def: 'Opposite angles formed by two intersecting lines; always congruent.' },
    { term: 'Adjacent angles', def: 'Two angles that share a vertex and a side but no interior points.' }, { term: 'Angle bisector', def: 'A ray that divides an angle into two congruent angles.' },
    { term: 'Linear pair', def: 'Adjacent angles whose non-shared sides form a line; they are supplementary.' },
  ],
  formulas: [{ name: 'Complementary', f: '∠1 + ∠2 = 90°' }, { name: 'Supplementary / linear pair', f: '∠1 + ∠2 = 180°' }, { name: 'Vertical angles', f: '∠1 = ∠3' }],
  generate: (d) => {
    const t = d === 0 ? pick(['comp', 'supp', 'vert', 'classify']) : pick(['comp', 'supp', 'vert', 'algebra', 'bisect']);
    if (t === 'classify') {
      const a = pick([ri(10, 85), 90, ri(95, 175), 180]);
      const name = a < 90 ? 'Acute' : a === 90 ? 'Right' : a < 180 ? 'Obtuse' : 'Straight';
      return {
        prompt: `Classify an angle that measures ${a}°.`,
        diagram: <Fig><Rays dirs={[0, a]} arcs={[{ from: 0, to: a, label: `${a}°`, right: a === 90 }]} /></Fig>,
        answer: mc(name, ['Acute', 'Right', 'Obtuse', 'Straight']),
        hints: ['Compare the angle to 90° and 180°.', 'Less than 90° is one type, exactly 90° is another, between 90° and 180° is another.'],
        explanation: `${a}° is ${name.toLowerCase()}.`,
      };
    }
    if (t === 'comp') {
      const a = ri(12, 78);
      return {
        prompt: `∠ABD and ∠DBC are complementary. m∠ABD = ${a}°. Find m∠DBC.`,
        diagram: <Fig><Rays dirs={[0, 90 - a + 0, 90]} labels={['C', 'D', 'A']} arcs={[{ from: 90 - a, to: 90, label: `${a}°` }, { from: 0, to: 90 - a, label: '?', color: ORANGE }]} /></Fig>,
        answer: num(90 - a, { mistakes: [{ value: 180 - a, msg: 'You used 180°. Complementary angles add to 90° (supplementary is 180°).' }] }),
        hints: ['Complementary angles add up to a special number. Which one?', `Set up: ${a} + x = 90.`],
        explanation: `${a} + x = 90 → x = ${90 - a}°.`,
      };
    }
    if (t === 'supp') {
      const a = ri(20, 160);
      return {
        prompt: `∠1 and ∠2 form a linear pair. m∠1 = ${a}°. Find m∠2.`,
        diagram: <Fig><Rays dirs={[0, 180, a]} arcs={[{ from: 0, to: a, label: `${a}°` }, { from: a, to: 180, label: '?', color: ORANGE }]} /></Fig>,
        answer: num(180 - a, { mistakes: [{ value: 90 - a, msg: 'You used 90°. A linear pair makes a straight line, which is 180°.' }] }),
        hints: ['A linear pair forms a straight line. How many degrees is a straight angle?', `Set up: ${a} + x = 180.`],
        explanation: `${a} + x = 180 → x = ${180 - a}°.`,
      };
    }
    if (t === 'vert') {
      const a = ri(25, 155);
      const ask = pick(['vertical', 'adjacent']);
      return {
        prompt: `Two lines intersect. One angle measures ${a}°. Find the measure of the angle that is ${ask === 'vertical' ? 'vertical to it' : 'adjacent to it (forms a linear pair)'}.`,
        diagram: <Fig><Rays dirs={[15, 15 + a]} lines arcs={[{ from: 15, to: 15 + a, label: `${a}°` }, { from: 15 + a, to: 195, label: ask === 'adjacent' ? '?' : '', color: ORANGE }, { from: 195, to: 195 + a, label: ask === 'vertical' ? '?' : '', color: ORANGE }]} /></Fig>,
        answer: num(ask === 'vertical' ? a : 180 - a),
        hints: [ask === 'vertical' ? 'Vertical angles are across from each other. What is always true about them?' : 'Adjacent angles on a line form a straight angle.', ask === 'vertical' ? 'Vertical angles are congruent.' : 'They add to 180°.'],
        explanation: ask === 'vertical' ? `Vertical angles are congruent, so it is ${a}°.` : `180 − ${a} = ${180 - a}°.`,
      };
    }
    if (t === 'bisect') {
      const x = ri(3, 15), a = ri(2, 5), b = ri(1, 20), c = a + ri(1, 3), e = a * x + b - c * x;
      if (e < -30 || a * x + b <= 0) return angles.generate(d);
      return {
        prompt: `Ray BD bisects ∠ABC. m∠ABD = (${a}x + ${b})° and m∠DBC = (${c}x ${e >= 0 ? '+ ' + e : '− ' + -e})°. Find m∠ABC.`,
        answer: num(2 * (a * x + b), { mistakes: [{ value: x, msg: 'That is x. The question asks for the whole angle ABC.' }, { value: a * x + b, msg: 'That is only half of ∠ABC. The bisector splits it into two equal halves.' }] }),
        hints: ['A bisector makes two EQUAL angles. Set the two expressions equal.', 'Solve for x, then find one half-angle.', 'The whole angle is two halves added together.'],
        explanation: `${a}x + ${b} = ${c}x ${e >= 0 ? '+ ' + e : '− ' + -e} → x = ${x}. Each half = ${a * x + b}°, so m∠ABC = ${2 * (a * x + b)}°.`,
      };
    }
    const total = pick([90, 180]), x = ri(5, 25), a = ri(2, 4), b = ri(-10, 20);
    const other = total - (a * x + b);
    if (other <= 5) return angles.generate(d);
    const k = ri(1, 3), m = other - k * x;
    return {
      prompt: `Two angles are ${total === 90 ? 'complementary' : 'supplementary'}. Their measures are (${a}x ${b >= 0 ? '+ ' + b : '− ' + -b})° and (${k}x ${m >= 0 ? '+ ' + m : '− ' + -m})°. Find x.`,
      answer: num(x),
      hints: [`${total === 90 ? 'Complementary' : 'Supplementary'} angles add to ${total}°. Write an equation.`, `(${a}x ${b >= 0 ? '+' : '−'} ${Math.abs(b)}) + (${k}x ${m >= 0 ? '+' : '−'} ${Math.abs(m)}) = ${total}`, 'Combine like terms and solve for x.'],
      explanation: `${a + k}x + ${b + m} = ${total} → x = ${x}.`,
    };
  },
};

// ---------- 3. Parallel lines ----------
const pairInfo: Record<string, { pairs: [number, number][]; rel: 'equal' | 'supp'; name: string }> = {
  corresponding: { pairs: [[1, 5], [2, 6], [3, 7], [4, 8]], rel: 'equal', name: 'Corresponding angles' },
  'alternate interior': { pairs: [[3, 6], [4, 5]], rel: 'equal', name: 'Alternate interior angles' },
  'alternate exterior': { pairs: [[1, 8], [2, 7]], rel: 'equal', name: 'Alternate exterior angles' },
  'same-side interior': { pairs: [[3, 5], [4, 6]], rel: 'supp', name: 'Same-side interior angles' },
};
const parallel: Topic = {
  id: 'geo-3', title: 'Parallel Lines & Transversals',
  lesson: `A **transversal** is a line that crosses two other lines. When the two lines are **parallel**, special angle pairs appear:
• **Corresponding angles** (same position at each crossing) are **equal**.
• **Alternate interior angles** (between the lines, opposite sides of t) are **equal**.
• **Alternate exterior angles** (outside the lines, opposite sides of t) are **equal**.
• **Same-side interior angles** (between the lines, same side of t) are **supplementary** (add to 180°).
Tip: at each crossing there are only two sizes of angle — a small one and a big one, and they add to 180°.`,
  lessonDiagram: <ParallelLines marks={{}} />,
  example: {
    problem: 'Lines m and n are parallel. ∠3 = 115°. Find ∠6.',
    diagram: <ParallelLines marks={{ 3: '115°', 6: '?' }} />,
    steps: [
      { step: '∠3 and ∠6 are alternate interior angles.', why: 'They are between the parallel lines and on opposite sides of the transversal.' },
      { step: 'Alternate interior angles are congruent when lines are parallel.', why: 'This is the Alternate Interior Angles Theorem.' },
      { step: '∠6 = 115°', why: 'Congruent means equal measure.' },
    ],
  },
  vocab: [
    { term: 'Transversal', def: 'A line that intersects two or more lines at different points.' },
    { term: 'Parallel lines', def: 'Coplanar lines that never intersect.' },
    { term: 'Corresponding angles', def: 'Angles in matching positions at each intersection; congruent if lines are parallel.' },
    { term: 'Alternate interior angles', def: 'Between the lines, on opposite sides of the transversal; congruent if lines are parallel.' },
    { term: 'Alternate exterior angles', def: 'Outside the lines, on opposite sides of the transversal; congruent if lines are parallel.' },
    { term: 'Same-side interior angles', def: 'Between the lines, same side of the transversal; supplementary if lines are parallel.' },
    { term: 'Skew lines', def: 'Lines that are not coplanar and never intersect.' },
  ],
  formulas: [{ name: 'Corresponding / alt. interior / alt. exterior', f: 'congruent (equal)' }, { name: 'Same-side interior', f: 'add to 180°' }],
  generate: (d) => {
    const kind = pick(Object.keys(pairInfo));
    const info = pairInfo[kind];
    const [p, q] = shuffle(pick(info.pairs));
    const a = ri(40, 140);
    if (d === 0 && Math.random() < 0.5) {
      return {
        prompt: `Lines m and n are parallel. What kind of angle pair are ∠${p} and ∠${q}?`,
        diagram: <ParallelLines marks={{ [p]: `∠${p}`, [q]: `∠${q}` }} />,
        answer: mc(info.name, Object.values(pairInfo).map((v) => v.name)),
        hints: ['First: are the angles between the parallel lines (interior) or outside (exterior)?', 'Next: are they on the same side of the transversal t, or opposite sides?'],
        explanation: `∠${p} and ∠${q} are ${info.name.toLowerCase()}.`,
      };
    }
    if (d >= 2) {
      const x = ri(5, 20), c1 = ri(2, 5), k1 = ri(-10, 30);
      const v1 = c1 * x + k1;
      if (v1 < 30 || v1 > 150) return parallel.generate(d);
      const v2 = info.rel === 'equal' ? v1 : 180 - v1;
      const c2 = ri(1, 4), k2 = v2 - c2 * x;
      return {
        prompt: `m ∥ n. ∠${p} = (${c1}x ${k1 >= 0 ? '+ ' + k1 : '− ' + -k1})° and ∠${q} = (${c2}x ${k2 >= 0 ? '+ ' + k2 : '− ' + -k2})°. Find x.`,
        diagram: <ParallelLines marks={{ [p]: `∠${p}`, [q]: `∠${q}` }} />,
        answer: num(x),
        hints: [`Name the angle pair: ∠${p} and ∠${q} are ${kind} angles.`, info.rel === 'equal' ? 'That pair is congruent, so set the expressions equal.' : 'That pair is supplementary, so the expressions add to 180.', 'Solve the equation for x.'],
        explanation: `${info.name} are ${info.rel === 'equal' ? 'congruent' : 'supplementary'}. Solving gives x = ${x}.`,
      };
    }
    const ans = info.rel === 'equal' ? a : 180 - a;
    return {
      prompt: `m ∥ n and ∠${p} = ${a}°. Find ∠${q}.`,
      diagram: <ParallelLines marks={{ [p]: `${a}°`, [q]: '?' }} />,
      answer: num(ans, { mistakes: [{ value: info.rel === 'equal' ? 180 - a : a, msg: info.rel === 'equal' ? `These are ${kind} angles, which are congruent — not supplementary.` : 'Same-side interior angles are supplementary, not congruent.' }] }),
      hints: [`What type of angle pair are ∠${p} and ∠${q}? Look at whether they are between the lines and which side of t they are on.`, `They are ${kind} angles. Are those congruent or supplementary?`],
      explanation: `${info.name} are ${info.rel === 'equal' ? 'congruent' : 'supplementary'}, so ∠${q} = ${ans}°.`,
    };
  },
};

// ---------- 4. Logic ----------
const conds = [
  ['an animal is a dog', 'it is a mammal'], ['a number is divisible by 4', 'it is even'], ['a shape is a square', 'it has four sides'],
  ['it is raining', 'the ground is wet'], ['two angles are vertical angles', 'they are congruent'], ['a triangle is equilateral', 'it is isosceles'],
  ['you live in Texas', 'you live in the United States'], ['an angle measures 30°', 'it is acute'], ['a number is prime and greater than 2', 'it is odd'],
];
const neg = (s: string) => s.replace(/^(an?|two|it|you|the|a number) /, (m) => m) .replace(/ is /, ' is not ').replace(/ are /, ' are not ').replace(/ has /, ' does not have ').replace(/ live /, ' do not live ').replace(/ measures /, ' does not measure ');
const logic: Topic = {
  id: 'geo-4', title: 'Logic & Reasoning',
  lesson: `A **conditional statement** has the form "If p, then q." p is the **hypothesis**, q is the **conclusion**.
• **Converse**: If q, then p. (switch)
• **Inverse**: If not p, then not q. (negate)
• **Contrapositive**: If not q, then not p. (switch AND negate) — always has the same truth value as the original!
A **counterexample** is one example that shows a statement is false.
**Inductive reasoning** uses patterns and examples to make a guess (conjecture). **Deductive reasoning** uses facts, definitions, and logic to reach a conclusion that must be true.`,
  example: {
    problem: 'Write the contrapositive of: "If a shape is a square, then it has four sides."',
    steps: [
      { step: 'Hypothesis p: a shape is a square. Conclusion q: it has four sides.', why: 'Split the statement at "if" and "then".' },
      { step: 'Contrapositive = If not q, then not p.', why: 'Switch the parts AND negate both.' },
      { step: '"If a shape does not have four sides, then it is not a square."', why: 'This is true, just like the original — contrapositives always match.' },
    ],
  },
  vocab: [
    { term: 'Conditional', def: 'An if-then statement: If p, then q.' }, { term: 'Hypothesis', def: 'The "if" part of a conditional.' },
    { term: 'Conclusion', def: 'The "then" part of a conditional.' }, { term: 'Converse', def: 'Switch hypothesis and conclusion: If q, then p.' },
    { term: 'Inverse', def: 'Negate both parts: If not p, then not q.' }, { term: 'Contrapositive', def: 'Switch and negate: If not q, then not p.' },
    { term: 'Counterexample', def: 'An example that proves a statement false.' }, { term: 'Inductive reasoning', def: 'Making a conjecture from patterns or examples.' },
    { term: 'Deductive reasoning', def: 'Reaching a conclusion using facts, definitions, and logic.' }, { term: 'Biconditional', def: 'p if and only if q; true when a statement and its converse are both true.' },
  ],
  generate: (d) => {
    const t = d === 0 ? pick(['hyp', 'form']) : pick(['form', 'induct', 'pattern', 'form']);
    const [p, q] = pick(conds);
    if (t === 'hyp') {
      const askH = Math.random() < 0.5;
      return {
        prompt: `"If ${p}, then ${q}." What is the ${askH ? 'hypothesis' : 'conclusion'}?`,
        answer: mc(askH ? p : q, [askH ? q : p, `If ${q}`, `${p} and ${q}`]),
        hints: ['The hypothesis comes right after "if". The conclusion comes after "then".'],
        explanation: `Hypothesis: ${p}. Conclusion: ${q}.`,
      };
    }
    if (t === 'form') {
      const forms = {
        Converse: `If ${q}, then ${p}.`, Inverse: `If ${neg(p)}, then ${neg(q)}.`, Contrapositive: `If ${neg(q)}, then ${neg(p)}.`, Original: `If ${p}, then ${q}.`,
      };
      const which = pick(['Converse', 'Inverse', 'Contrapositive'] as const);
      return {
        prompt: `Original: "If ${p}, then ${q}." Which statement is the ${which.toLowerCase()}?`,
        answer: mc(forms[which], Object.values(forms)),
        hints: ['Converse = switch. Inverse = negate. Contrapositive = switch AND negate.', `For the ${which.toLowerCase()}: ${which === 'Converse' ? 'does the order change? do you add "not"?' : which === 'Inverse' ? 'keep the order, add "not" to both parts.' : 'swap the order and add "not" to both parts.'}`],
        explanation: `The ${which.toLowerCase()} is: "${forms[which]}"`,
      };
    }
    if (t === 'induct') {
      const items = [
        ['Every swan Mia has seen is white, so she concludes all swans are white.', 'Inductive'],
        ['All squares are rectangles. ABCD is a square. So ABCD is a rectangle.', 'Deductive'],
        ['The sun has risen every day in recorded history, so it will rise tomorrow.', 'Inductive'],
        ['Vertical angles are congruent. ∠1 and ∠2 are vertical, so ∠1 ≅ ∠2.', 'Deductive'],
        ['2, 4, 8, 16 … each term doubles, so the next term is probably 32.', 'Inductive'],
        ['If a number is even it is divisible by 2. 14 is even, so 14 is divisible by 2.', 'Deductive'],
      ];
      const [s, a] = pick(items);
      return {
        prompt: `Which type of reasoning is this?\n"${s}"`,
        answer: mc(a, ['Inductive', 'Deductive']),
        hints: ['Is the conclusion based on a pattern/examples, or on a rule/definition applied to a case?'],
        explanation: a === 'Inductive' ? 'It generalizes from examples or a pattern — inductive.' : 'It applies a known rule to a specific case — deductive.',
      };
    }
    const start = ri(1, 9), step = ri(2, 7), mult = d >= 2 && Math.random() < 0.5;
    const seq = Array.from({ length: 5 }, (_, i) => (mult ? start * 2 ** i : start + step * i));
    return {
      prompt: `Use inductive reasoning to find the next number: ${seq.slice(0, 4).join(', ')}, …`,
      answer: num(seq[4]),
      hints: ['Look at how each number changes to get the next one.', mult ? 'Try dividing each term by the one before it.' : 'Try subtracting each term from the next one.'],
      explanation: mult ? `Each term doubles: ${seq[3]} × 2 = ${seq[4]}.` : `Add ${step} each time: ${seq[3]} + ${step} = ${seq[4]}.`,
    };
  },
};

// ---------- 5. Triangles ----------
function triPts(A: number, B: number): P[] {
  // base from (0,0) to (1,0); apex from the two base angles, then fit into the box
  const a = (A * Math.PI) / 180, b = (B * Math.PI) / 180;
  const x = Math.tan(b) / (Math.tan(a) + Math.tan(b)), y = x * Math.tan(a);
  const xs = [0, 1, x], minX = Math.min(...xs), maxX = Math.max(...xs);
  const s = Math.min(240 / (maxX - minX), 150 / y);
  const ox = 160 - ((minX + maxX) / 2) * s;
  return [[ox, 195], [ox + s, 195], [ox + x * s, 195 - y * s]];
}
const triangles: Topic = {
  id: 'geo-5', title: 'Triangles: Angle Sum & Classifying',
  lesson: `The three angles of any triangle add to **180°** (Triangle Sum Theorem).
An **exterior angle** equals the sum of the two **remote interior angles** (the two angles not next to it).
Classify by sides: **scalene** (no equal sides), **isosceles** (at least 2 equal), **equilateral** (all 3 equal).
Classify by angles: **acute**, **right**, **obtuse**, **equiangular**.
In an **isosceles** triangle the **base angles** (across from the equal sides) are equal. An **equilateral** triangle has three 60° angles.`,
  lessonDiagram: <Fig><Polygon pts={[[50, 190], [270, 190], [130, 50]]} vLabels={['A', 'B', 'C']} angles={['a°', 'b°', 'c°']} /><T x={160} y={215} size={12}>a + b + c = 180</T></Fig>,
  example: {
    problem: 'In △ABC, ∠A = 48° and ∠B = 67°. Find ∠C.',
    steps: [
      { step: '∠A + ∠B + ∠C = 180°', why: 'Triangle Sum Theorem.' },
      { step: '48 + 67 + ∠C = 180', why: 'Substitute the known angles.' },
      { step: '115 + ∠C = 180 → ∠C = 65°', why: 'Add the known angles, then subtract from 180.' },
    ],
  },
  vocab: [
    { term: 'Triangle Sum Theorem', def: 'The interior angles of a triangle add to 180°.' }, { term: 'Exterior angle', def: 'Formed by one side of a triangle and the extension of another side.' },
    { term: 'Remote interior angles', def: 'The two interior angles not adjacent to a given exterior angle.' }, { term: 'Scalene', def: 'A triangle with no congruent sides.' },
    { term: 'Isosceles', def: 'A triangle with at least two congruent sides.' }, { term: 'Equilateral', def: 'A triangle with three congruent sides (all angles 60°).' },
    { term: 'Base angles', def: 'The two angles opposite the congruent sides of an isosceles triangle; they are congruent.' },
  ],
  formulas: [{ name: 'Triangle sum', f: '∠A + ∠B + ∠C = 180°' }, { name: 'Exterior angle', f: 'exterior = sum of two remote interior angles' }],
  generate: (d) => {
    const t = d === 0 ? pick(['sum', 'classify']) : pick(['sum', 'ext', 'iso', 'algebra']);
    if (t === 'classify') {
      const kind = pick(['scalene', 'isosceles', 'equilateral']);
      const s = kind === 'equilateral' ? [7, 7, 7] : kind === 'isosceles' ? shuffle([8, 8, 5]) : shuffle([5, 7, 9]);
      return {
        prompt: `A triangle has sides ${s.join(', ')}. Classify it by its sides.`,
        diagram: <Fig><Polygon pts={kind === 'equilateral' ? [[70, 190], [250, 190], [160, 34]] : kind === 'isosceles' ? [[90, 190], [230, 190], [160, 40]] : [[50, 190], [270, 190], [110, 60]]} sides={s.map(String)} /></Fig>,
        answer: mc(kind[0].toUpperCase() + kind.slice(1), ['Scalene', 'Isosceles', 'Equilateral']),
        hints: ['Count how many sides are the same length.'],
        explanation: `The sides ${s.join(', ')} make it ${kind}.`,
      };
    }
    const A = ri(30, 90), B = ri(25, 150 - A), C = 180 - A - B;
    if (t === 'sum') return {
      prompt: `In △ABC, m∠A = ${A}° and m∠B = ${B}°. Find m∠C.`,
      diagram: <Fig><Polygon pts={triPts(A, B)} vLabels={['A', 'B', 'C']} angles={[`${A}°`, `${B}°`, '?']} /></Fig>,
      answer: num(C, { mistakes: [{ value: 360 - A - B, msg: 'A triangle\'s angles add to 180°, not 360°.' }] }),
      hints: ['What do the three angles of a triangle add up to?', `Add the two angles you know: ${A} + ${B}. Then subtract from the total.`],
      explanation: `${A} + ${B} + C = 180 → C = ${C}°.`,
    };
    if (t === 'ext') return {
      prompt: `An exterior angle of a triangle is formed at vertex C. The remote interior angles are ${A}° and ${B}°. Find the exterior angle.`,
      diagram: <Fig><Polygon pts={[[40, 180], [210, 180], [110, 60]]} vLabels={['A', 'C', 'B']} angles={[`${A}°`, undefined, `${B}°`]} /><Seg a={[210, 180]} b={[300, 180]} /><AngleMark v={[210, 180]} p={[300, 180]} q={[110, 60]} label="?" color={ORANGE} /></Fig>,
      answer: num(A + B, { mistakes: [{ value: C, msg: 'That is the interior angle at C. The exterior angle is its supplement.' }] }),
      hints: ['Exterior Angle Theorem: exterior angle = sum of the two remote interior angles.', 'Remote means the two angles NOT touching the exterior angle.'],
      explanation: `Exterior = ${A} + ${B} = ${A + B}°.`,
    };
    if (t === 'iso') {
      const vert = ri(20, 140) & ~1, base = (180 - vert) / 2;
      const askBase = Math.random() < 0.6;
      return {
        prompt: askBase ? `An isosceles triangle has a vertex angle of ${vert}°. Find each base angle.` : `An isosceles triangle has base angles of ${base}° each. Find the vertex angle.`,
        diagram: <Fig><Polygon pts={[[80, 190], [240, 190], [160, 45]]} ticks={[0, 1, 0]} angles={askBase ? ['?', '?', `${vert}°`] : [`${base}°`, `${base}°`, '?']} /></Fig>,
        answer: num(askBase ? base : vert, { mistakes: [{ value: 180 - vert, msg: 'Remember there are TWO base angles sharing what is left. Divide by 2.' }] }),
        hints: ['Base angles of an isosceles triangle are equal, and all three angles add to 180°.', askBase ? `Subtract ${vert} from 180 to get what both base angles share.` : `Add the two base angles, then subtract from 180.`],
        explanation: askBase ? `(180 − ${vert}) ÷ 2 = ${base}°.` : `180 − 2(${base}) = ${vert}°.`,
      };
    }
    const x = ri(8, 25), a = ri(1, 3), b = ri(1, 3), k = 180 - (a + b) * x;
    if (k < 15 || k > 140) return triangles.generate(d);
    return {
      prompt: `The angles of a triangle are ${a}x°, ${b}x°, and ${k}°. Find x${d >= 2 ? ' and then the largest angle. (Enter the largest angle.)' : '.'}`,
      answer: num(d >= 2 ? Math.max(a * x, b * x, k) : x),
      hints: ['All three angles add to 180°.', `${a}x + ${b}x + ${k} = 180. Combine like terms.`],
      explanation: `${a + b}x = ${180 - k} → x = ${x}. Angles: ${a * x}°, ${b * x}°, ${k}°.`,
    };
  },
};

// ---------- 6. Congruent triangles ----------
const congruent: Topic = {
  id: 'geo-6', title: 'Congruent Triangles (SSS, SAS, ASA, AAS, HL)',
  lesson: `Two triangles are **congruent** if all matching sides and angles are equal. You only need certain combinations to prove it:
• **SSS** – three pairs of sides. • **SAS** – two sides and the angle **between** them.
• **ASA** – two angles and the side **between** them. • **AAS** – two angles and a side **not** between them.
• **HL** – hypotenuse and a leg (right triangles only).
**SSA** and **AAA** do NOT prove congruence!
**CPCTC**: Corresponding Parts of Congruent Triangles are Congruent — once triangles are proven congruent, all their matching parts are equal.`,
  example: {
    problem: 'AB ≅ DE, ∠B ≅ ∠E, BC ≅ EF. Which postulate proves △ABC ≅ △DEF?',
    steps: [
      { step: 'List what is given: side, angle, side.', why: 'Match the order around the triangle.' },
      { step: '∠B is between sides AB and BC.', why: 'The angle is formed by those two sides — it is the included angle.' },
      { step: 'SAS', why: 'Two sides and the included angle.' },
    ],
  },
  vocab: [
    { term: 'Congruent triangles', def: 'Triangles whose corresponding sides and angles are all congruent.' }, { term: 'SSS', def: 'Three sides of one triangle congruent to three sides of another.' },
    { term: 'SAS', def: 'Two sides and the included angle congruent.' }, { term: 'ASA', def: 'Two angles and the included side congruent.' },
    { term: 'AAS', def: 'Two angles and a non-included side congruent.' }, { term: 'HL', def: 'Hypotenuse and one leg of right triangles congruent.' },
    { term: 'CPCTC', def: 'Corresponding Parts of Congruent Triangles are Congruent.' }, { term: 'Included angle', def: 'The angle formed between two given sides.' },
  ],
  generate: (d) => {
    const t = d <= 1 ? 'which' : pick(['which', 'cpctc', 'solve']);
    const cases: [string, number[], number[], string][] = [
      ['SSS', [1, 1, 1], [0, 0, 0], 'All three pairs of sides are marked congruent.'],
      ['SAS', [1, 1, 0], [0, 1, 0], 'Two sides and the angle between them.'],
      ['ASA', [1, 0, 0], [1, 1, 0], 'Two angles and the side between them.'],
      ['AAS', [0, 1, 0], [1, 1, 0], 'Two angles and a side that is not between them.'],
      ['Not enough info', [1, 0, 1], [0, 0, 1], 'This is SSA — the angle is not between the two sides, so it does not prove congruence.'],
    ];
    if (t === 'which') {
      const [name, sides, angs, why] = pick(cases);
      const ptsA: P[] = [[20, 180], [140, 180], [60, 70]], ptsB: P[] = [[180, 180], [300, 180], [220, 70]];
      const mk = (pts: P[], lbl: string[]) => (
        <Polygon pts={pts} vLabels={lbl} ticks={sides.map((s, i) => (s ? i + 1 : 0))} angles={angs.map((a, i) => (a ? ['•', '••', '•••'][i] : undefined))} />
      );
      return {
        prompt: 'Matching marks show congruent parts. Which postulate or theorem proves the triangles congruent?',
        diagram: <Fig>{mk(ptsA, ['A', 'B', 'C'])}{mk(ptsB, ['D', 'E', 'F'])}</Fig>,
        answer: mc(name, ['SSS', 'SAS', 'ASA', 'AAS', 'Not enough info']),
        hints: ['Count the marked sides and marked angles.', 'If there is one side and two angles (or two sides and one angle), check whether the side/angle is BETWEEN the others.'],
        explanation: `${name}: ${why}`,
      };
    }
    if (t === 'cpctc') {
      const s = ri(5, 20), ang = ri(30, 100);
      const askSide = Math.random() < 0.5;
      return {
        prompt: `△PQR ≅ △XYZ. PQ = ${s}, m∠R = ${ang}°. Find ${askSide ? 'XY' : 'm∠Z'}.`,
        answer: num(askSide ? s : ang),
        hints: ['The order of letters tells you which parts match: P↔X, Q↔Y, R↔Z.', `Which part of △PQR matches ${askSide ? 'XY' : '∠Z'}?`],
        explanation: askSide ? `PQ ↔ XY by CPCTC, so XY = ${s}.` : `∠R ↔ ∠Z by CPCTC, so m∠Z = ${ang}°.`,
      };
    }
    const x = ri(3, 12), a = ri(2, 5), b = ri(1, 10), c = a + ri(1, 3), e = a * x + b - c * x;
    return {
      prompt: `△ABC ≅ △DEF. AB = ${a}x + ${b} and DE = ${c}x ${e >= 0 ? '+ ' + e : '− ' + -e}. Find AB.`,
      answer: num(a * x + b, { mistakes: [{ value: x, msg: 'That is x. Plug it back in to find AB.' }] }),
      hints: ['AB and DE are corresponding parts, so by CPCTC they are equal.', 'Set the expressions equal and solve for x.', 'Substitute x into AB.'],
      explanation: `${a}x + ${b} = ${c}x ${e >= 0 ? '+' : '−'} ${Math.abs(e)} → x = ${x}, AB = ${a * x + b}.`,
    };
  },
};

// ---------- 7. Proofs ----------
const proofBank = [
  {
    given: ['∠1 and ∠2 are vertical angles'], prove: '∠1 ≅ ∠3 when ∠2 ≅ ∠3',
    steps: [
      { statement: '∠1 and ∠2 are vertical angles', reason: 'Given' },
      { statement: '∠1 ≅ ∠2', reason: 'Vertical Angles Theorem' },
      { statement: '∠2 ≅ ∠3', reason: 'Given' },
      { statement: '∠1 ≅ ∠3', reason: 'Transitive Property' },
    ],
  },
  {
    given: ['M is the midpoint of AB', 'AM = 7'], prove: 'AB = 14',
    steps: [
      { statement: 'M is the midpoint of AB', reason: 'Given' },
      { statement: 'AM = MB', reason: 'Definition of midpoint' },
      { statement: 'AM + MB = AB', reason: 'Segment Addition Postulate' },
      { statement: 'AM + AM = AB', reason: 'Substitution Property' },
      { statement: 'AB = 14', reason: 'Substitution Property' },
    ],
  },
  {
    given: ['AB ≅ CB', 'BD bisects ∠ABC'], prove: '△ABD ≅ △CBD',
    steps: [
      { statement: 'AB ≅ CB', reason: 'Given' },
      { statement: 'BD bisects ∠ABC', reason: 'Given' },
      { statement: '∠ABD ≅ ∠CBD', reason: 'Definition of angle bisector' },
      { statement: 'BD ≅ BD', reason: 'Reflexive Property' },
      { statement: '△ABD ≅ △CBD', reason: 'SAS' },
    ],
  },
  {
    given: ['m ∥ n', '∠1 and ∠5 are corresponding angles', '∠5 and ∠7 form a linear pair'], prove: '∠1 and ∠7 are supplementary',
    steps: [
      { statement: 'm ∥ n', reason: 'Given' },
      { statement: '∠1 ≅ ∠5', reason: 'Corresponding Angles Postulate' },
      { statement: '∠5 and ∠7 form a linear pair', reason: 'Given' },
      { statement: 'm∠5 + m∠7 = 180°', reason: 'Linear Pair Postulate' },
      { statement: 'm∠1 + m∠7 = 180°', reason: 'Substitution Property' },
    ],
  },
  {
    given: ['2x + 5 = 17'], prove: 'x = 6',
    steps: [
      { statement: '2x + 5 = 17', reason: 'Given' },
      { statement: '2x = 12', reason: 'Subtraction Property of Equality' },
      { statement: 'x = 6', reason: 'Division Property of Equality' },
    ],
  },
  {
    given: ['AB ≅ DE', 'BC ≅ EF', 'AC ≅ DF'], prove: '∠A ≅ ∠D',
    steps: [
      { statement: 'AB ≅ DE, BC ≅ EF, AC ≅ DF', reason: 'Given' },
      { statement: '△ABC ≅ △DEF', reason: 'SSS' },
      { statement: '∠A ≅ ∠D', reason: 'CPCTC' },
    ],
  },
  {
    given: ['∠1 and ∠2 are complementary', '∠3 and ∠2 are complementary'], prove: '∠1 ≅ ∠3',
    steps: [
      { statement: '∠1 and ∠2 are complementary; ∠3 and ∠2 are complementary', reason: 'Given' },
      { statement: 'm∠1 + m∠2 = 90°, m∠3 + m∠2 = 90°', reason: 'Definition of complementary angles' },
      { statement: 'm∠1 + m∠2 = m∠3 + m∠2', reason: 'Substitution Property' },
      { statement: 'm∠1 = m∠3', reason: 'Subtraction Property of Equality' },
      { statement: '∠1 ≅ ∠3', reason: 'Definition of congruent angles' },
    ],
  },
  {
    given: ['AC ⟂ BD at C', 'C is the midpoint of BD'], prove: '△ACB ≅ △ACD',
    steps: [
      { statement: 'AC ⟂ BD at C', reason: 'Given' },
      { statement: '∠ACB ≅ ∠ACD', reason: 'All right angles are congruent' },
      { statement: 'C is the midpoint of BD', reason: 'Given' },
      { statement: 'BC ≅ DC', reason: 'Definition of midpoint' },
      { statement: 'AC ≅ AC', reason: 'Reflexive Property' },
      { statement: '△ACB ≅ △ACD', reason: 'SAS' },
    ],
  },
];
const REASONS = ['Given', 'Vertical Angles Theorem', 'Transitive Property', 'Definition of midpoint', 'Segment Addition Postulate', 'Substitution Property', 'Definition of angle bisector', 'Reflexive Property', 'SAS', 'SSS', 'ASA', 'CPCTC', 'Corresponding Angles Postulate', 'Linear Pair Postulate', 'Subtraction Property of Equality', 'Division Property of Equality', 'Definition of complementary angles', 'Definition of congruent angles', 'All right angles are congruent', 'Symmetric Property'];
const proofs: Topic = {
  id: 'geo-7', title: 'Proofs (fill in the reasons)',
  lesson: `A **proof** is a logical argument where every statement is backed by a **reason**.
Reasons can be: **Given** information, **definitions** (midpoint, bisector, congruent…), **postulates** (Segment Addition, Linear Pair…), **properties** (Reflexive, Symmetric, Transitive, Substitution, Addition/Subtraction/Division Property of Equality), and **theorems** (Vertical Angles, SAS, CPCTC…).
In a **two-column proof**, statements go on the left and reasons on the right. A **paragraph proof** says the same thing in sentences. A **flowchart proof** uses boxes and arrows.
Here the app gives you the statements — you choose the reason for each one.`,
  example: {
    problem: 'Given: M is the midpoint of AB. Prove: AM ≅ MB.',
    steps: [
      { step: 'M is the midpoint of AB — Given', why: 'Always start with what you are told.' },
      { step: 'AM ≅ MB — Definition of midpoint', why: 'A midpoint splits a segment into two congruent parts. That definition is exactly the reason.' },
    ],
  },
  vocab: [
    { term: 'Reflexive Property', def: 'Anything is congruent/equal to itself: AB ≅ AB.' }, { term: 'Symmetric Property', def: 'If a = b, then b = a.' },
    { term: 'Transitive Property', def: 'If a = b and b = c, then a = c.' }, { term: 'Substitution Property', def: 'If a = b, you can replace a with b in any expression.' },
    { term: 'Two-column proof', def: 'A proof with statements in one column and reasons in the other.' }, { term: 'Postulate', def: 'A statement accepted as true without proof.' },
    { term: 'Theorem', def: 'A statement that has been proven true.' },
  ],
  generate: (d) => {
    const pr = pick(proofBank.filter((p) => (d === 0 ? p.steps.length <= 4 : d === 1 ? p.steps.length <= 5 : true)));
    const bank = shuffle([...new Set([...pr.steps.map((s) => s.reason), ...shuffle(REASONS).slice(0, 4 + d * 2)])]);
    return {
      prompt: `Given: ${pr.given.join('; ')}.\nProve: ${pr.prove}.\nChoose the reason for each statement.`,
      answer: { kind: 'proof', steps: pr.steps, reasonBank: bank, given: pr.given },
      hints: ['Every statement that is copied from the "Given" list gets the reason "Given".', 'For each other step, ask: what definition, property, or theorem lets me say this?', 'If a step replaces one thing with an equal thing, that is Substitution.'],
      explanation: pr.steps.map((s, i) => `${i + 1}. ${s.statement} — ${s.reason}`).join('\n'),
    };
  },
};

// ---------- 8. Similarity ----------
const similarity: Topic = {
  id: 'geo-8', title: 'Similarity & Proportions',
  lesson: `**Similar** figures have the same shape but maybe a different size. Their angles are equal and their sides are **proportional**.
The **scale factor** is the ratio of matching sides (new ÷ original).
Prove triangles similar with **AA** (two pairs of equal angles), **SSS~** (all sides proportional), or **SAS~** (two sides proportional and the included angle equal).
To find a missing side, set up a **proportion** and **cross-multiply**.`,
  example: {
    problem: '△ABC ~ △DEF. AB = 6, BC = 9, DE = 10. Find EF.',
    steps: [
      { step: 'AB/DE = BC/EF', why: 'Matching sides of similar triangles are proportional (A↔D, B↔E, C↔F).' },
      { step: '6/10 = 9/EF', why: 'Substitute the lengths.' },
      { step: '6 · EF = 90', why: 'Cross-multiply.' },
      { step: 'EF = 15', why: 'Divide both sides by 6.' },
    ],
  },
  vocab: [
    { term: 'Similar figures', def: 'Figures with congruent angles and proportional sides.' }, { term: 'Scale factor', def: 'The ratio of corresponding side lengths of similar figures.' },
    { term: 'Proportion', def: 'An equation stating that two ratios are equal.' }, { term: 'AA Similarity', def: 'Two pairs of congruent angles prove triangles similar.' },
    { term: 'Cross products', def: 'In a/b = c/d, the products ad and bc, which are equal.' },
  ],
  formulas: [{ name: 'Proportion', f: 'a/b = c/d → ad = bc' }, { name: 'Scale factor', f: 'k = new length / original length' }, { name: 'Area ratio', f: 'k²' }],
  generate: (d) => {
    const t = d === 0 ? pick(['side', 'scale']) : d === 1 ? pick(['side', 'solve']) : pick(['side', 'area', 'shadow']);
    const a = ri(2, 9), b = ri(3, 12), k = pick([1.5, 2, 2.5, 3, 4]);
    if (t === 'scale') return {
      prompt: `△ABC ~ △DEF with AB = ${a} and DE = ${a * k}. What is the scale factor from △ABC to △DEF?`,
      answer: num(k, { mistakes: [{ value: 1 / k, msg: 'That is upside down. Scale factor from ABC to DEF is new ÷ original = DE ÷ AB.' }] }),
      hints: ['Scale factor = (new side) ÷ (matching original side).', `Divide DE by AB.`],
      explanation: `${a * k} ÷ ${a} = ${k}.`,
    };
    if (t === 'side') return {
      prompt: `△ABC ~ △DEF. AB = ${a}, BC = ${b}, DE = ${fmt(a * k)}. Find EF.`,
      diagram: <Fig><Polygon pts={[[20, 190], [110, 190], [40, 120]]} vLabels={['A', 'B', 'C']} sides={[String(a), String(b)]} /><Polygon pts={[[140, 200], [300, 200], [176, 70]]} vLabels={['D', 'E', 'F']} sides={[fmt(a * k), '?']} /></Fig>,
      answer: num(b * k),
      hints: ['Write a proportion with matching sides: AB/DE = BC/EF.', `Substitute: ${a}/${fmt(a * k)} = ${b}/EF. Now cross-multiply.`],
      explanation: `${a} · EF = ${fmt(a * k)} · ${b} → EF = ${fmt(b * k)}.`,
    };
    if (t === 'solve') {
      const x = ri(2, 10);
      return {
        prompt: `Solve for n: ${a}/${b} = ${a * x}/n.`,
        answer: num(b * x),
        hints: ['Cross-multiply: the product of the diagonals are equal.', `${a} · n = ${b} · ${a * x}. Now divide.`],
        explanation: `n = ${b} · ${a * x} ÷ ${a} = ${b * x}.`,
      };
    }
    if (t === 'area') {
      const A1 = a * b;
      return {
        prompt: `Two similar figures have a scale factor of ${k}. The smaller has an area of ${A1} square units. Find the area of the larger.`,
        answer: num(A1 * k * k, { mistakes: [{ value: A1 * k, msg: 'Areas scale by the scale factor SQUARED, not just the scale factor.' }] }),
        hints: ['Lengths scale by k, but areas scale by k².', `Compute ${k}² first.`],
        explanation: `${A1} × ${k}² = ${fmt(A1 * k * k)}.`,
      };
    }
    const h = ri(4, 7), s = ri(3, 9), S = ri(15, 40);
    return {
      prompt: `A ${h}-ft person casts a ${s}-ft shadow. At the same time, a tree casts a ${S}-ft shadow. How tall is the tree? (Round to the nearest tenth.)`,
      answer: num(round((h * S) / s, 1)),
      hints: ['The sun makes similar right triangles: height/shadow is the same for both.', `${h}/${s} = tree/${S}. Cross-multiply.`],
      explanation: `tree = ${h} × ${S} ÷ ${s} ≈ ${round((h * S) / s, 1)} ft.`,
    };
  },
};

// ---------- 9. Right triangles ----------
const triples = [[3, 4, 5], [5, 12, 13], [8, 15, 17], [7, 24, 25], [6, 8, 10], [9, 12, 15], [20, 21, 29]];
const rtFig = (a: string, b: string, c: string) => (
  <Fig><Polygon pts={[[60, 190], [260, 190], [60, 60]]} rights={[0]} sides={[a, c, b]} vLabels={['C', 'B', 'A']} /></Fig>
);
const right: Topic = {
  id: 'geo-9', title: 'Right Triangles & Pythagorean Theorem',
  lesson: `In a right triangle, the **legs** (a and b) form the right angle, and the **hypotenuse** (c) is the longest side, across from the right angle.
**Pythagorean Theorem**: a² + b² = c².
**Converse**: if a² + b² = c², the triangle is right. If a² + b² > c² it's acute; if a² + b² < c² it's obtuse.
**45-45-90**: legs x, x; hypotenuse x√2.
**30-60-90**: short leg x, long leg x√3, hypotenuse 2x.`,
  lessonDiagram: rtFig('a', 'b', 'c'),
  example: {
    problem: 'A right triangle has legs 6 and 8. Find the hypotenuse.',
    steps: [
      { step: 'a² + b² = c²', why: 'Pythagorean Theorem.' },
      { step: '6² + 8² = c² → 36 + 64 = c²', why: 'Square each leg.' },
      { step: '100 = c² → c = 10', why: 'Take the square root of both sides (length is positive).' },
    ],
  },
  vocab: [
    { term: 'Hypotenuse', def: 'The side opposite the right angle; the longest side.' }, { term: 'Leg', def: 'Either side that forms the right angle.' },
    { term: 'Pythagorean Theorem', def: 'a² + b² = c² for right triangles.' }, { term: 'Pythagorean triple', def: 'Three whole numbers that satisfy a² + b² = c², like 3-4-5.' },
    { term: '45-45-90 triangle', def: 'Legs equal; hypotenuse = leg × √2.' }, { term: '30-60-90 triangle', def: 'Hypotenuse = 2 × short leg; long leg = short leg × √3.' },
  ],
  formulas: [{ name: 'Pythagorean Theorem', f: 'a² + b² = c²' }, { name: '45-45-90', f: 'x : x : x√2' }, { name: '30-60-90', f: 'x : x√3 : 2x' }],
  generate: (d) => {
    const t = d === 0 ? pick(['hyp', 'leg']) : d === 1 ? pick(['hyp', 'leg', 'converse', 'dec']) : pick(['special45', 'special30', 'dec', 'converse']);
    const [a, b, c] = pick(triples).map((v) => v * (d >= 1 ? ri(1, 2) : 1));
    if (t === 'hyp') return {
      prompt: `A right triangle has legs ${a} and ${b}. Find the hypotenuse.`, diagram: rtFig(String(a), String(b), '?'),
      answer: num(c, { mistakes: [{ value: a + b, msg: 'You added the legs. Square them first: a² + b² = c².' }] }),
      hints: ['Use a² + b² = c² with the legs as a and b.', `Compute ${a}² + ${b}². Then take the square root.`], explanation: `${a}² + ${b}² = ${a * a + b * b}, √${a * a + b * b} = ${c}.`,
    };
    if (t === 'leg') return {
      prompt: `A right triangle has hypotenuse ${c} and one leg ${a}. Find the other leg.`, diagram: rtFig(String(a), '?', String(c)),
      answer: num(b, { mistakes: [{ value: Math.sqrt(a * a + c * c), msg: 'You added the squares. When finding a LEG, subtract: b² = c² − a².' }] }),
      hints: ['The hypotenuse is c. Put the known leg in for a.', `b² = ${c}² − ${a}².`], explanation: `b² = ${c * c} − ${a * a} = ${b * b}, b = ${b}.`,
    };
    if (t === 'dec') {
      const x = ri(2, 12), y = ri(2, 12), h = Math.sqrt(x * x + y * y);
      return {
        prompt: `A right triangle has legs ${x} and ${y}. Find the hypotenuse to the nearest tenth.`, diagram: rtFig(String(x), String(y), '?'),
        answer: num(round(h, 1)), hints: ['a² + b² = c²', `c = √(${x * x} + ${y * y}). Use a calculator for the square root.`], explanation: `c = √${x * x + y * y} ≈ ${round(h, 1)}.`,
      };
    }
    if (t === 'converse') {
      const tri = pick([[a, b, c], [a, b, c + 1], [a, b + 1, c], [5, 6, 7], [4, 5, 8]]).sort((m, n) => m - n);
      const s = tri[0] ** 2 + tri[1] ** 2, h2 = tri[2] ** 2;
      const kind = s === h2 ? 'Right' : s > h2 ? 'Acute' : 'Obtuse';
      return {
        prompt: `A triangle has sides ${tri.join(', ')}. Is it acute, right, or obtuse?`,
        answer: mc(kind, ['Acute', 'Right', 'Obtuse']),
        hints: ['Compare a² + b² (two shorter sides) with c² (longest side).', `${tri[0]}² + ${tri[1]}² = ? and ${tri[2]}² = ?`],
        explanation: `${tri[0]}² + ${tri[1]}² = ${s}, ${tri[2]}² = ${h2}. ${s === h2 ? 'Equal → right.' : s > h2 ? 'Greater → acute.' : 'Less → obtuse.'}`,
      };
    }
    if (t === 'special45') {
      const x = ri(2, 12);
      const askHyp = Math.random() < 0.5;
      return {
        prompt: askHyp ? `A 45-45-90 triangle has legs of ${x}. Find the hypotenuse. (You can type ${x}√2 or a decimal.)` : `A 45-45-90 triangle has a hypotenuse of ${x}√2. Find a leg.`,
        diagram: <Fig><Polygon pts={[[70, 190], [230, 190], [70, 30]]} rights={[0]} angles={[undefined, '45°', '45°']} sides={askHyp ? [String(x), '?', String(x)] : ['?', `${x}√2`, undefined]} /></Fig>,
        answer: num(askHyp ? x * Math.SQRT2 : x),
        hints: ['In a 45-45-90 triangle, the ratio of sides is x : x : x√2.', askHyp ? 'Multiply the leg by √2.' : 'Divide the hypotenuse by √2.'],
        explanation: askHyp ? `Hypotenuse = ${x}√2 ≈ ${round(x * Math.SQRT2, 2)}.` : `Leg = ${x}√2 ÷ √2 = ${x}.`,
      };
    }
    const x = ri(2, 10);
    const ask = pick(['hyp', 'long']);
    return {
      prompt: `A 30-60-90 triangle has a short leg of ${x}. Find the ${ask === 'hyp' ? 'hypotenuse' : 'long leg (type like ' + x + '√3 or a decimal)'}.`,
      diagram: <Fig><Polygon pts={[[60, 190], [270, 190], [60, 70]]} rights={[0]} angles={[undefined, '30°', '60°']} sides={[ask === 'long' ? '?' : undefined, ask === 'hyp' ? '?' : undefined, String(x)]} /></Fig>,
      answer: num(ask === 'hyp' ? 2 * x : x * Math.sqrt(3), { mistakes: [{ value: x * Math.SQRT2, msg: 'That is the 45-45-90 pattern. For 30-60-90 use x, x√3, 2x.' }] }),
      hints: ['30-60-90 ratio: short leg x, long leg x√3, hypotenuse 2x.', ask === 'hyp' ? 'The hypotenuse is double the short leg.' : 'The long leg is the short leg times √3.'],
      explanation: ask === 'hyp' ? `2 × ${x} = ${2 * x}.` : `${x}√3 ≈ ${round(x * Math.sqrt(3), 2)}.`,
    };
  },
};

// ---------- 10. Trigonometry ----------
const trig: Topic = {
  id: 'geo-10', title: 'Trigonometry (SOH-CAH-TOA)',
  lesson: `For an acute angle θ in a right triangle:
**SOH**: sin θ = Opposite / Hypotenuse
**CAH**: cos θ = Adjacent / Hypotenuse
**TOA**: tan θ = Opposite / Adjacent
"Opposite" is the side across from θ. "Adjacent" is the leg touching θ (not the hypotenuse).
To find a missing **side**, use the ratio and solve. To find a missing **angle**, use inverse trig: θ = sin⁻¹(…), cos⁻¹(…), tan⁻¹(…).
Make sure your calculator is in **degree** mode!`,
  example: {
    problem: 'In a right triangle, θ = 35° and the hypotenuse is 12. Find the side opposite θ.',
    steps: [
      { step: 'Opposite & hypotenuse → use sine (SOH).', why: 'We know the hypotenuse and want the opposite.' },
      { step: 'sin 35° = x / 12', why: 'Set up the ratio.' },
      { step: 'x = 12 · sin 35° ≈ 12 · 0.5736 ≈ 6.9', why: 'Multiply both sides by 12.' },
    ],
  },
  vocab: [
    { term: 'Sine', def: 'Opposite ÷ hypotenuse.' }, { term: 'Cosine', def: 'Adjacent ÷ hypotenuse.' }, { term: 'Tangent', def: 'Opposite ÷ adjacent.' },
    { term: 'Inverse trig', def: 'sin⁻¹, cos⁻¹, tan⁻¹ — find an angle from a ratio.' }, { term: 'Angle of elevation', def: 'Angle from horizontal looking UP at an object.' },
    { term: 'Angle of depression', def: 'Angle from horizontal looking DOWN at an object.' },
  ],
  formulas: [{ name: 'SOH', f: 'sin θ = opp / hyp' }, { name: 'CAH', f: 'cos θ = adj / hyp' }, { name: 'TOA', f: 'tan θ = opp / adj' }],
  generate: (d) => {
    const t = d === 0 ? 'ratio' : d === 1 ? 'side' : pick(['side', 'angle', 'elev']);
    const fig = (opp: string, adj: string, hyp: string) => (
      <Fig><Polygon pts={[[50, 190], [270, 190], [270, 60]]} rights={[1]} angles={['θ']} sides={[adj, opp, hyp]} /></Fig>
    );
    if (t === 'ratio') {
      const [a, b, c] = pick(triples);
      const fn = pick(['sin', 'cos', 'tan']);
      const val = fn === 'sin' ? `${a}/${c}` : fn === 'cos' ? `${b}/${c}` : `${a}/${b}`;
      return {
        prompt: `Opposite = ${a}, adjacent = ${b}, hypotenuse = ${c}. What is ${fn} θ?`, diagram: fig(String(a), String(b), String(c)),
        answer: mc(val, [`${a}/${c}`, `${b}/${c}`, `${a}/${b}`, `${b}/${a}`]),
        hints: ['Remember SOH-CAH-TOA.', `${fn} uses ${fn === 'sin' ? 'Opposite over Hypotenuse' : fn === 'cos' ? 'Adjacent over Hypotenuse' : 'Opposite over Adjacent'}.`],
        explanation: `${fn} θ = ${val}.`,
      };
    }
    const th = ri(20, 70), h = ri(8, 30);
    const r = (x: number) => (x * Math.PI) / 180;
    if (t === 'side') {
      const want = pick(['opp', 'adj']);
      const v = want === 'opp' ? h * Math.sin(r(th)) : h * Math.cos(r(th));
      return {
        prompt: `θ = ${th}° and the hypotenuse is ${h}. Find the ${want === 'opp' ? 'opposite side' : 'adjacent side'} to the nearest tenth.`,
        diagram: fig(want === 'opp' ? 'x' : '', want === 'adj' ? 'x' : '', String(h)),
        answer: num(round(v, 1)),
        hints: ['Which two sides are involved: the one you know and the one you want?', want === 'opp' ? 'Opposite and hypotenuse → sine.' : 'Adjacent and hypotenuse → cosine.', `Write ${want === 'opp' ? 'sin' : 'cos'} ${th}° = x / ${h}, then multiply both sides by ${h}.`],
        explanation: `x = ${h} · ${want === 'opp' ? 'sin' : 'cos'} ${th}° ≈ ${round(v, 1)}.`,
      };
    }
    if (t === 'angle') {
      const o = ri(3, 15), a = ri(3, 15);
      const ang = (Math.atan(o / a) * 180) / Math.PI;
      return {
        prompt: `The opposite side is ${o} and the adjacent side is ${a}. Find θ to the nearest tenth of a degree.`, diagram: fig(String(o), String(a), ''),
        answer: num(round(ang, 1)),
        hints: ['Opposite and adjacent → tangent.', `tan θ = ${o}/${a}. To get θ, use the inverse: θ = tan⁻¹(${o}/${a}).`],
        explanation: `θ = tan⁻¹(${o}/${a}) ≈ ${round(ang, 1)}°.`,
      };
    }
    const dist = ri(20, 100);
    const v = dist * Math.tan(r(th));
    return {
      prompt: `You stand ${dist} ft from a building. The angle of elevation to the top is ${th}°. How tall is the building (nearest tenth)?`,
      answer: num(round(v, 1)),
      hints: ['Draw it: the distance is the side adjacent to the angle, the height is opposite.', 'Opposite and adjacent → tangent.', `tan ${th}° = h / ${dist}.`],
      explanation: `h = ${dist} · tan ${th}° ≈ ${round(v, 1)} ft.`,
    };
  },
};

// ---------- 11. Polygons ----------
const regPts = (n: number, r = 85): P[] => Array.from({ length: n }, (_, i) => [160 + r * Math.cos(-Math.PI / 2 + (2 * Math.PI * i) / n), 115 + r * Math.sin(-Math.PI / 2 + (2 * Math.PI * i) / n)] as P);
const polyNames: Record<number, string> = { 5: 'pentagon', 6: 'hexagon', 7: 'heptagon', 8: 'octagon', 9: 'nonagon', 10: 'decagon', 12: 'dodecagon' };
const quadProps: [string, string][] = [
  ['Parallelogram', 'Both pairs of opposite sides are parallel, opposite angles are equal, and diagonals bisect each other.'],
  ['Rectangle', 'A parallelogram with four right angles; its diagonals are congruent.'],
  ['Rhombus', 'A parallelogram with four congruent sides; its diagonals are perpendicular.'],
  ['Square', 'Four congruent sides AND four right angles.'],
  ['Trapezoid', 'Exactly one pair of parallel sides.'],
  ['Kite', 'Two pairs of consecutive congruent sides; diagonals are perpendicular.'],
];
const polygons: Topic = {
  id: 'geo-11', title: 'Polygons & Quadrilaterals',
  lesson: `**Interior angle sum** of an n-sided polygon: (n − 2) · 180°.
Each interior angle of a **regular** polygon: (n − 2) · 180° ÷ n.
**Exterior angles** of any convex polygon always add to **360°**, so each exterior angle of a regular polygon = 360° ÷ n.
Quadrilaterals: **parallelogram** (opposite sides parallel & equal, opposite angles equal, consecutive angles supplementary), **rectangle** (4 right angles), **rhombus** (4 equal sides), **square** (both!), **trapezoid** (exactly one pair of parallel sides), **kite** (two pairs of adjacent equal sides).`,
  lessonDiagram: <Fig><Polygon pts={regPts(6)} /><T x={160} y={115} size={12}>(6−2)·180 = 720°</T></Fig>,
  example: {
    problem: 'Find each interior angle of a regular octagon.',
    steps: [
      { step: 'n = 8', why: 'An octagon has 8 sides.' },
      { step: 'Sum = (8 − 2) · 180 = 1080°', why: 'Interior angle sum formula.' },
      { step: 'Each = 1080 ÷ 8 = 135°', why: 'Regular means all angles are equal, so divide evenly.' },
    ],
  },
  vocab: [
    { term: 'Regular polygon', def: 'All sides congruent and all angles congruent.' }, { term: 'Convex polygon', def: 'No interior angle greater than 180°.' },
    ...quadProps.map(([term, def]) => ({ term, def })),
    { term: 'Diagonal', def: 'A segment joining two non-consecutive vertices.' },
  ],
  formulas: [{ name: 'Interior angle sum', f: '(n − 2) · 180°' }, { name: 'Each interior (regular)', f: '(n − 2) · 180° / n' }, { name: 'Exterior sum', f: '360°' }, { name: 'Each exterior (regular)', f: '360° / n' }],
  generate: (d) => {
    const t = d === 0 ? pick(['sum', 'each']) : d === 1 ? pick(['sum', 'each', 'ext', 'para']) : pick(['sides', 'para', 'quad', 'ext', 'missing']);
    const n = pick([5, 6, 8, 9, 10, 12]);
    if (t === 'sum') return {
      prompt: `Find the sum of the interior angles of a ${polyNames[n]} (${n} sides).`, diagram: <Fig><Polygon pts={regPts(n)} /></Fig>,
      answer: num((n - 2) * 180, { mistakes: [{ value: n * 180, msg: 'Use (n − 2), not n. A polygon splits into n − 2 triangles.' }] }),
      hints: ['A polygon can be split into triangles from one vertex. How many triangles?', `Use (n − 2) · 180 with n = ${n}.`], explanation: `(${n} − 2) · 180 = ${(n - 2) * 180}°.`,
    };
    if (t === 'each') return {
      prompt: `Find the measure of each interior angle of a regular ${polyNames[n]}.`, diagram: <Fig><Polygon pts={regPts(n)} /></Fig>,
      answer: num(((n - 2) * 180) / n),
      hints: ['First find the total interior angle sum.', `Then divide by ${n} because every angle is the same.`], explanation: `${(n - 2) * 180} ÷ ${n} = ${fmt(((n - 2) * 180) / n)}°.`,
    };
    if (t === 'ext') return {
      prompt: `Find each exterior angle of a regular ${polyNames[n]}.`, answer: num(360 / n),
      hints: ['The exterior angles of any convex polygon add up to the same number every time.', `Divide 360° by ${n}.`], explanation: `360 ÷ ${n} = ${fmt(360 / n)}°.`,
    };
    if (t === 'sides') {
      const m = pick([5, 6, 8, 9, 10, 12, 15, 18, 20]);
      return {
        prompt: `Each interior angle of a regular polygon measures ${fmt(((m - 2) * 180) / m)}°. How many sides does it have?`, answer: num(m),
        hints: ['Find the exterior angle first: 180° − interior.', 'Exterior angles sum to 360°, so n = 360 ÷ (exterior angle).'], explanation: `Exterior = ${fmt(360 / m)}°, n = 360 ÷ ${fmt(360 / m)} = ${m}.`,
      };
    }
    if (t === 'para') {
      const a = ri(50, 130);
      const ask = pick(['opp', 'cons']);
      return {
        prompt: `ABCD is a parallelogram and m∠A = ${a}°. Find m∠${ask === 'opp' ? 'C' : 'B'}.`,
        diagram: <Fig><Polygon pts={[[40, 170], [220, 170], [280, 60], [100, 60]]} vLabels={['A', 'B', 'C', 'D']} angles={[`${a}°`, ask === 'cons' ? '?' : undefined, ask === 'opp' ? '?' : undefined]} /></Fig>,
        answer: num(ask === 'opp' ? a : 180 - a),
        hints: ['Opposite angles of a parallelogram are congruent; consecutive angles are supplementary.', `Is ∠${ask === 'opp' ? 'C' : 'B'} opposite ∠A or next to it?`],
        explanation: ask === 'opp' ? `Opposite angles are equal: ${a}°.` : `Consecutive angles are supplementary: 180 − ${a} = ${180 - a}°.`,
      };
    }
    if (t === 'missing') {
      const angs = [ri(70, 110), ri(70, 110), ri(70, 110)];
      const miss = 360 - angs.reduce((s, v) => s + v, 0);
      return {
        prompt: `A quadrilateral has angles ${angs.join('°, ')}°, and x°. Find x.`, answer: num(miss),
        hints: ['A quadrilateral\'s angles add up to (4 − 2) · 180°.', 'Subtract the known angles from 360°.'], explanation: `360 − ${angs.reduce((s, v) => s + v, 0)} = ${miss}°.`,
      };
    }
    const [name, desc] = pick(quadProps);
    return {
      prompt: `Which quadrilateral is described? "${desc}"`, answer: mc(name, quadProps.map((q) => q[0])),
      hints: ['Look for key words: "exactly one pair", "four right angles", "four congruent sides", "consecutive".'], explanation: `${name}: ${desc}`,
    };
  },
};

// ---------- 12. Transformations ----------
const transforms: Topic = {
  id: 'geo-12', title: 'Transformations',
  lesson: `A **transformation** moves or changes a figure. The original is the **pre-image**; the result is the **image** (labeled A′).
• **Translation** (slide): (x, y) → (x + a, y + b)
• **Reflection** over x-axis: (x, y) → (x, −y). Over y-axis: (x, y) → (−x, y). Over y = x: (x, y) → (y, x).
• **Rotation** about the origin: 90° counterclockwise (x, y) → (−y, x); 180° (x, y) → (−x, −y); 270° ccw (x, y) → (y, −x).
• **Dilation** by scale factor k from the origin: (x, y) → (kx, ky).
Translations, reflections, and rotations are **rigid motions** (they keep size and shape). Dilations change size.
A **composition** is doing one transformation after another.`,
  lessonDiagram: <Grid min={-6} max={6} polys={[{ pts: [[1, 1], [4, 1], [1, 3]] }, { pts: [[-1, 1], [-4, 1], [-1, 3]], color: '#FF9F43' }]} />,
  example: {
    problem: 'Rotate A(3, −2) 90° counterclockwise about the origin.',
    steps: [
      { step: 'Rule: (x, y) → (−y, x)', why: 'This is the rule for a 90° counterclockwise rotation.' },
      { step: 'x = 3, y = −2', why: 'Identify the coordinates.' },
      { step: "A′ = (−(−2), 3) = (2, 3)", why: 'Plug in. The negative of −2 is 2.' },
    ],
  },
  vocab: [
    { term: 'Pre-image', def: 'The original figure before a transformation.' }, { term: 'Image', def: 'The figure after a transformation.' },
    { term: 'Translation', def: 'A slide of every point the same distance and direction.' }, { term: 'Reflection', def: 'A flip over a line of reflection.' },
    { term: 'Rotation', def: 'A turn around a fixed point.' }, { term: 'Dilation', def: 'Enlarging or shrinking by a scale factor from a center.' },
    { term: 'Rigid motion', def: 'A transformation that preserves distance and angle measure.' }, { term: 'Line symmetry', def: 'A figure can be folded onto itself along a line.' },
    { term: 'Rotational symmetry', def: 'A figure matches itself after a rotation of less than 360°.' },
  ],
  formulas: [
    { name: 'Reflect x-axis', f: '(x, y) → (x, −y)' }, { name: 'Reflect y-axis', f: '(x, y) → (−x, y)' }, { name: 'Reflect y = x', f: '(x, y) → (y, x)' },
    { name: 'Rotate 90° ccw', f: '(x, y) → (−y, x)' }, { name: 'Rotate 180°', f: '(x, y) → (−x, −y)' }, { name: 'Rotate 270° ccw', f: '(x, y) → (y, −x)' }, { name: 'Dilation', f: '(x, y) → (kx, ky)' },
  ],
  generate: (d) => {
    const x = ri(-5, 5) || 2, y = ri(-5, 5) || -3;
    const opts = [
      { name: 'translate', f: () => { const a = ri(-3, 3) || 1, b = ri(-3, 3) || -2; return { desc: `translated by (x ${a >= 0 ? '+' : '−'} ${Math.abs(a)}, y ${b >= 0 ? '+' : '−'} ${Math.abs(b)})`, p: [x + a, y + b], hint: 'Add the shift to each coordinate.' }; } },
      { name: 'rx', f: () => ({ desc: 'reflected over the x-axis', p: [x, -y], hint: 'Reflecting over the x-axis flips up/down. Which coordinate changes sign?' }) },
      { name: 'ry', f: () => ({ desc: 'reflected over the y-axis', p: [-x, y], hint: 'Reflecting over the y-axis flips left/right. Which coordinate changes sign?' }) },
      { name: 'ryx', f: () => ({ desc: 'reflected over the line y = x', p: [y, x], hint: 'Reflecting over y = x swaps something about the coordinates.' }) },
      { name: 'r90', f: () => ({ desc: 'rotated 90° counterclockwise about the origin', p: [-y, x], hint: 'Rule for 90° ccw: (x, y) → (−y, x).' }) },
      { name: 'r180', f: () => ({ desc: 'rotated 180° about the origin', p: [-x, -y], hint: '180° rotation: both coordinates change sign.' }) },
      { name: 'r270', f: () => ({ desc: 'rotated 270° counterclockwise about the origin', p: [y, -x], hint: 'Rule for 270° ccw (same as 90° clockwise): (x, y) → (y, −x).' }) },
      { name: 'dil', f: () => { const k = pick([2, 0.5, 3]); return { desc: `dilated by scale factor ${k} centered at the origin`, p: [x * k, y * k], hint: 'Multiply each coordinate by the scale factor.' }; } },
    ];
    const pool = d === 0 ? opts.slice(0, 3) : d === 1 ? opts.slice(0, 6) : opts;
    if (d >= 3 && Math.random() < 0.6) {
      const a = pick(opts.slice(1, 6)).f();
      const [x2, y2] = a.p as number[];
      const b = pick(['rx', 'ry']);
      const final = b === 'rx' ? [x2, -y2] : [-x2, y2];
      if (Math.abs(final[0]) > 6 || Math.abs(final[1]) > 6) return transforms.generate(d);
      return {
        prompt: `Composition: A(${x}, ${y}) is ${a.desc}, then reflected over the ${b === 'rx' ? 'x' : 'y'}-axis. Click A″ on the grid (or type it).`,
        answer: { kind: 'point', x: final[0], y: final[1], shown: [{ x, y, label: 'A' }] },
        hints: ['Do the transformations one at a time, in order.', a.hint, `After the first move you get A′. Now reflect A′ over the ${b === 'rx' ? 'x' : 'y'}-axis.`],
        explanation: `First: A′ = (${x2}, ${y2}). Then A″ = (${final[0]}, ${final[1]}).`,
      };
    }
    const o = pick(pool).f();
    const [px, py] = o.p as number[];
    if (!Number.isInteger(px) || !Number.isInteger(py) || Math.abs(px) > 6 || Math.abs(py) > 6) return transforms.generate(d);
    return {
      prompt: `A(${x}, ${y}) is ${o.desc}. Click where A′ lands on the grid (or type it like (2, -3)).`,
      answer: { kind: 'point', x: px, y: py, shown: [{ x, y, label: 'A' }] },
      hints: [o.hint, 'Write the rule, then plug in x and y carefully — watch the signs.'],
      explanation: `A′ = (${px}, ${py}).`,
    };
  },
};

// ---------- 13. Coordinate geometry ----------
const coordinate: Topic = {
  id: 'geo-13', title: 'Coordinate Geometry',
  lesson: `**Slope** m = (y₂ − y₁) / (x₂ − x₁) — "rise over run."
**Parallel** lines have **equal** slopes. **Perpendicular** lines have slopes that are **negative reciprocals** (multiply to −1).
**Distance formula**: d = √((x₂ − x₁)² + (y₂ − y₁)²) — it's the Pythagorean Theorem on a grid!
**Midpoint formula**: M = ((x₁ + x₂)/2, (y₁ + y₂)/2).
You can **prove** shapes on a grid: e.g., a parallelogram has opposite sides with equal slopes; a right angle has perpendicular slopes.`,
  lessonDiagram: <Grid points={[{ x: -3, y: -2, label: 'A' }, { x: 3, y: 2, label: 'B' }]} segments={[[-3, -2, 3, 2]]} />,
  example: {
    problem: 'Find the distance between A(1, 2) and B(4, 6).',
    steps: [
      { step: 'Δx = 4 − 1 = 3, Δy = 6 − 2 = 4', why: 'Find how far apart the points are horizontally and vertically.' },
      { step: 'd = √(3² + 4²) = √(9 + 16)', why: 'Distance formula (Pythagorean Theorem with legs Δx and Δy).' },
      { step: 'd = √25 = 5', why: 'Simplify.' },
    ],
  },
  vocab: [
    { term: 'Slope', def: 'Steepness of a line: rise ÷ run.' }, { term: 'Parallel slopes', def: 'Equal slopes.' },
    { term: 'Perpendicular slopes', def: 'Negative reciprocals; their product is −1.' }, { term: 'Distance formula', def: '√((x₂−x₁)² + (y₂−y₁)²)' },
    { term: 'Midpoint formula', def: '((x₁+x₂)/2, (y₁+y₂)/2)' },
  ],
  formulas: [{ name: 'Slope', f: 'm = (y₂ − y₁)/(x₂ − x₁)' }, { name: 'Distance', f: 'd = √((x₂−x₁)² + (y₂−y₁)²)' }, { name: 'Midpoint', f: '((x₁+x₂)/2, (y₁+y₂)/2)' }, { name: 'Point-slope', f: 'y − y₁ = m(x − x₁)' }],
  generate: (d) => {
    const t = d === 0 ? pick(['slope', 'mid']) : d === 1 ? pick(['slope', 'dist', 'mid']) : pick(['dist', 'perp', 'para', 'endpoint', 'dist']);
    const x1 = ri(-5, 3), y1 = ri(-5, 3);
    const [dx, dy] = d <= 1 && t === 'dist' ? pick([[3, 4], [4, 3], [6, 8], [5, 12].map((v) => v / 2)]) : [ri(1, 4), ri(-4, 4) || 2];
    const x2 = x1 + dx, y2 = y1 + dy;
    const pts = [{ x: x1, y: y1, label: `A(${x1},${y1})` }, { x: x2, y: y2, label: `B(${x2},${y2})` }];
    const g = <Grid points={pts} segments={[[x1, y1, x2, y2]]} />;
    if (t === 'slope') return {
      prompt: `Find the slope of the line through A(${x1}, ${y1}) and B(${x2}, ${y2}).`, diagram: g,
      answer: num(dy / dx, { mistakes: [{ value: dx / dy, msg: 'That is run over rise. Slope is RISE (change in y) over RUN (change in x).' }] }),
      hints: ['Slope = (change in y) ÷ (change in x).', `Change in y: ${y2} − (${y1}). Change in x: ${x2} − (${x1}).`], explanation: `m = ${dy}/${dx} = ${fmt(dy / dx)}.`,
    };
    if (t === 'mid') {
      const mx = (x1 + x2) / 2, my = (y1 + y2) / 2;
      if (!Number.isInteger(mx) || !Number.isInteger(my)) return coordinate.generate(d);
      return {
        prompt: `Find the midpoint of A(${x1}, ${y1}) and B(${x2}, ${y2}). Click it on the grid or type it.`,
        answer: { kind: 'point', x: mx, y: my, shown: [{ x: x1, y: y1, label: 'A' }, { x: x2, y: y2, label: 'B' }] },
        hints: ['The midpoint is the average of the x\'s and the average of the y\'s.', `x: (${x1} + ${x2}) ÷ 2. y: (${y1} + ${y2}) ÷ 2.`], explanation: `M = (${mx}, ${my}).`,
      };
    }
    if (t === 'dist') {
      const dist = Math.hypot(dx, dy);
      return {
        prompt: `Find the distance between A(${x1}, ${y1}) and B(${x2}, ${y2})${Number.isInteger(dist) ? '' : ' to the nearest tenth'}.`, diagram: g,
        answer: num(round(dist, 1)), hints: ['Find Δx and Δy first.', `d = √(${dx}² + ${dy}²).`], explanation: `d = √(${dx * dx} + ${dy * dy}) ≈ ${round(dist, 2)}.`,
      };
    }
    if (t === 'endpoint') {
      const mx = x1 + dx, my = y1 + dy;
      return {
        prompt: `M(${mx}, ${my}) is the midpoint of AB, and A = (${x1}, ${y1}). Find B. Click it or type it.`,
        answer: { kind: 'point', x: 2 * mx - x1, y: 2 * my - y1, grid: { min: -10, max: 10 }, shown: [{ x: x1, y: y1, label: 'A' }, { x: mx, y: my, label: 'M' }] },
        hints: ['The midpoint is halfway, so B is the same "step" past M as M is past A.', `From A to M you move (${dx}, ${dy}). Do it again from M.`], explanation: `B = (${2 * mx - x1}, ${2 * my - y1}).`,
      };
    }
    const m = pick([2, -3, 0.5, -0.25, 4, -2]);
    const perp = t === 'perp';
    return {
      prompt: `Line ℓ has slope ${fmt(m)}. What is the slope of a line ${perp ? 'perpendicular' : 'parallel'} to ℓ?`,
      answer: num(perp ? -1 / m : m, { mistakes: perp ? [{ value: 1 / m, msg: 'Close! Reciprocal is right, but you also need to change the sign (negative reciprocal).' }, { value: -m, msg: 'Just changing the sign isn\'t enough. Flip it too (negative reciprocal).' }] : [] }),
      hints: [perp ? 'Perpendicular slopes are negative reciprocals: flip the fraction and change the sign.' : 'Parallel lines never meet — they must have the same steepness.'],
      explanation: perp ? `−1 ÷ ${fmt(m)} = ${fmt(-1 / m)}.` : `Parallel → same slope: ${fmt(m)}.`,
    };
  },
};

// ---------- 14. Circles ----------
const circles: Topic = {
  id: 'geo-14', title: 'Circles',
  lesson: `**Radius** r: center to edge. **Diameter** d = 2r. **Chord**: segment with both endpoints on the circle.
**Circumference** C = 2πr = πd.
A **central angle** has its vertex at the center; it equals its **intercepted arc**.
An **inscribed angle** has its vertex on the circle; it is **half** its intercepted arc.
A **tangent** touches the circle at exactly one point and is **perpendicular** to the radius at that point.
**Arc length** = (central angle ÷ 360) × 2πr.`,
  lessonDiagram: <CircleFigL parts={['central', 'inscribed']} labels={{ central: '2x', inscribed: 'x' }} />,
  example: {
    problem: 'An inscribed angle intercepts an arc of 110°. Find the inscribed angle.',
    diagram: <CircleFigL parts={['inscribed', 'arc']} labels={{ arc: '110°', inscribed: '?' }} />,
    steps: [
      { step: 'Inscribed angle = ½ · intercepted arc', why: 'Inscribed Angle Theorem.' },
      { step: '½ · 110° = 55°', why: 'Take half of the arc.' },
    ],
  },
  vocab: [
    { term: 'Radius', def: 'A segment from the center to a point on the circle.' }, { term: 'Diameter', def: 'A chord through the center; twice the radius.' },
    { term: 'Chord', def: 'A segment whose endpoints are on the circle.' }, { term: 'Central angle', def: 'Angle with vertex at the center; equals its arc.' },
    { term: 'Inscribed angle', def: 'Angle with vertex on the circle; half its intercepted arc.' }, { term: 'Tangent', def: 'A line that touches a circle at exactly one point; ⟂ to the radius there.' },
    { term: 'Arc', def: 'Part of a circle between two points.' }, { term: 'Circumference', def: 'The distance around a circle: 2πr.' },
  ],
  formulas: [{ name: 'Circumference', f: 'C = 2πr = πd' }, { name: 'Arc length', f: '(θ/360) · 2πr' }, { name: 'Inscribed angle', f: '½ · intercepted arc' }, { name: 'Central angle', f: '= intercepted arc' }],
  generate: (d) => {
    const t = d === 0 ? pick(['diam', 'circ', 'central']) : d === 1 ? pick(['circ', 'inscribed', 'central', 'arcFromInscribed']) : pick(['arclen', 'tangent', 'inscribed', 'arclen', 'semicircle']);
    const r = ri(2, 15);
    if (t === 'diam') {
      const askR = Math.random() < 0.5;
      return {
        prompt: askR ? `A circle has diameter ${2 * r}. Find the radius.` : `A circle has radius ${r}. Find the diameter.`, diagram: <CircleFigL parts={[askR ? 'diameter' : 'radius']} labels={{ diameter: String(2 * r), radius: String(r) }} />,
        answer: num(askR ? r : 2 * r), hints: ['The diameter goes all the way across; the radius goes halfway.'], explanation: askR ? `${2 * r} ÷ 2 = ${r}.` : `2 × ${r} = ${2 * r}.`,
      };
    }
    if (t === 'circ') return {
      prompt: `Find the circumference of a circle with radius ${r}. Round to the nearest tenth (or type ${2 * r}π).`, diagram: <CircleFigL parts={['radius']} labels={{ radius: String(r) }} />,
      answer: num(2 * Math.PI * r, { mistakes: [{ value: Math.PI * r * r, msg: 'That is the AREA (πr²). Circumference is 2πr.' }, { value: Math.PI * r, msg: 'You used πr. Circumference is 2πr (or π times the diameter).' }] }),
      hints: ['Circumference = 2πr.', `Multiply 2 × π × ${r}.`], explanation: `C = 2π(${r}) = ${2 * r}π ≈ ${round(2 * Math.PI * r, 1)}.`,
    };
    const arc = ri(20, 170) & ~1;
    if (t === 'central') return {
      prompt: `Central angle ∠AOB intercepts arc AB, which measures ${arc}°. Find m∠AOB.`, diagram: <CircleFigL parts={['central', 'arc']} labels={{ central: '?', arc: `${arc}°` }} />,
      answer: num(arc, { mistakes: [{ value: arc / 2, msg: 'Halving is for INSCRIBED angles. A central angle equals its arc.' }] }),
      hints: ['Where is the vertex — at the center or on the circle?', 'A central angle has the same measure as its intercepted arc.'], explanation: `m∠AOB = ${arc}°.`,
    };
    if (t === 'inscribed') return {
      prompt: `Inscribed angle ∠ACB intercepts arc AB = ${arc}°. Find m∠ACB.`, diagram: <CircleFigL parts={['inscribed', 'arc']} labels={{ inscribed: '?', arc: `${arc}°` }} />,
      answer: num(arc / 2, { mistakes: [{ value: arc, msg: 'That would be a central angle. The vertex is ON the circle, so it is half the arc.' }] }),
      hints: ['The vertex C is on the circle, so this is an inscribed angle.', 'Inscribed angle = half the intercepted arc.'], explanation: `½ · ${arc} = ${arc / 2}°.`,
    };
    if (t === 'arcFromInscribed') {
      const ang = ri(15, 85);
      return {
        prompt: `Inscribed angle ∠ACB measures ${ang}°. Find the intercepted arc AB.`, diagram: <CircleFigL parts={['inscribed', 'arc']} labels={{ inscribed: `${ang}°`, arc: '?' }} />,
        answer: num(2 * ang), hints: ['Inscribed angle = ½ arc, so arc = ?', 'Undo the "half" by doubling.'], explanation: `arc = 2 × ${ang} = ${2 * ang}°.`,
      };
    }
    if (t === 'semicircle') return {
      prompt: 'An inscribed angle intercepts a semicircle (its sides go through the ends of a diameter). What is its measure?',
      answer: num(90), hints: ['A semicircle is an arc of 180°.', 'Inscribed angle = half its arc.'], explanation: 'Half of 180° = 90°. It is always a right angle.',
    };
    if (t === 'tangent') {
      const [a, , c] = pick(triples);
      return {
        prompt: `Segment PT is tangent to circle O at T. The radius OT = ${a} and OP = ${c}. Find PT.`,
        diagram: <CircleFigL parts={['tangent']} labels={{}} />,
        answer: num(Math.sqrt(c * c - a * a)),
        hints: ['A tangent is perpendicular to the radius at the point of tangency — so there is a right triangle.', `OP is the hypotenuse. Use a² + b² = c².`], explanation: `PT = √(${c}² − ${a}²) = ${Math.sqrt(c * c - a * a)}.`,
      };
    }
    const ang = pick([30, 45, 60, 72, 90, 120, 135, 150]);
    const L = (ang / 360) * 2 * Math.PI * r;
    return {
      prompt: `A circle has radius ${r}. Find the length of an arc with a central angle of ${ang}°, to the nearest tenth.`, diagram: <CircleFigL parts={['central', 'radius']} labels={{ central: `${ang}°`, radius: String(r) }} />,
      answer: num(round(L, 1)), hints: ['Arc length is a fraction of the whole circumference.', `Fraction = ${ang}/360. Circumference = 2π(${r}).`], explanation: `(${ang}/360) · 2π(${r}) ≈ ${round(L, 2)}.`,
    };
  },
};

// ---------- 15. Area & perimeter ----------
const area: Topic = {
  id: 'geo-15', title: 'Area & Perimeter',
  lesson: `**Perimeter** is the distance around. **Area** is the space inside (square units).
• Rectangle: A = lw • Parallelogram: A = bh • Triangle: A = ½bh
• Trapezoid: A = ½(b₁ + b₂)h • Rhombus/kite: A = ½d₁d₂
• Circle: A = πr² • Sector: A = (θ/360) · πr²
**Height** is always perpendicular to the base — not the slanted side!
**Composite shapes**: split into simple shapes, find each area, then add (or subtract holes).`,
  example: {
    problem: 'A trapezoid has bases 6 and 10 and height 4. Find its area.',
    steps: [
      { step: 'A = ½(b₁ + b₂)h', why: 'Trapezoid area formula — average the bases, multiply by height.' },
      { step: 'A = ½(6 + 10)(4)', why: 'Substitute.' },
      { step: 'A = ½(16)(4) = 32', why: 'Add, then multiply. Units are square units.' },
    ],
  },
  vocab: [
    { term: 'Area', def: 'The number of square units inside a figure.' }, { term: 'Perimeter', def: 'The distance around a figure.' },
    { term: 'Height (altitude)', def: 'Perpendicular distance from the base to the opposite side/vertex.' }, { term: 'Sector', def: 'A "pizza slice" of a circle bounded by two radii and an arc.' },
    { term: 'Composite figure', def: 'A figure made of two or more simple shapes.' },
  ],
  formulas: [
    { name: 'Rectangle', f: 'A = lw' }, { name: 'Triangle', f: 'A = ½bh' }, { name: 'Parallelogram', f: 'A = bh' }, { name: 'Trapezoid', f: 'A = ½(b₁ + b₂)h' },
    { name: 'Rhombus / kite', f: 'A = ½d₁d₂' }, { name: 'Circle', f: 'A = πr²' }, { name: 'Sector', f: 'A = (θ/360)πr²' },
  ],
  generate: (d) => {
    const t = d === 0 ? pick(['rect', 'tri', 'perim']) : d === 1 ? pick(['tri', 'trap', 'para', 'circle']) : pick(['composite', 'sector', 'trap', 'kite', 'hole']);
    const a = ri(3, 14), b = ri(3, 14), h = ri(2, 10);
    if (t === 'rect') return {
      prompt: `Find the area of a rectangle ${a} by ${b}.`, diagram: <Fig><Polygon pts={[[60, 170], [260, 170], [260, 60], [60, 60]]} sides={[String(a), String(b)]} rights={[0, 1, 2, 3]} /></Fig>,
      answer: num(a * b, { mistakes: [{ value: 2 * (a + b), msg: 'That is the perimeter. Area multiplies length × width.' }] }), hints: ['Area of a rectangle = length × width.'], explanation: `${a} × ${b} = ${a * b}.`,
    };
    if (t === 'perim') return {
      prompt: `Find the perimeter of a rectangle ${a} by ${b}.`, diagram: <Fig><Polygon pts={[[60, 170], [260, 170], [260, 60], [60, 60]]} sides={[String(a), String(b)]} /></Fig>,
      answer: num(2 * (a + b), { mistakes: [{ value: a * b, msg: 'That is the area. Perimeter adds up all four sides.' }, { value: a + b, msg: 'A rectangle has FOUR sides — two of each length.' }] }), hints: ['Perimeter = add all the sides.', 'There are two sides of each length.'], explanation: `2(${a} + ${b}) = ${2 * (a + b)}.`,
    };
    if (t === 'tri') return {
      prompt: `A triangle has base ${a} and height ${h}. Find its area.`, diagram: <Fig><Polygon pts={[[50, 180], [270, 180], [120, 50]]} sides={[String(a)]} /><Seg a={[120, 50]} b={[120, 180]} dash color={ORANGE} /><T x={132} y={115} color={ORANGE}>{h}</T></Fig>,
      answer: num((a * h) / 2, { mistakes: [{ value: a * h, msg: 'Don\'t forget the ½! A triangle is half of a parallelogram.' }] }), hints: ['A = ½ · base · height.', `½ × ${a} × ${h}`], explanation: `½(${a})(${h}) = ${fmt((a * h) / 2)}.`,
    };
    if (t === 'para') return {
      prompt: `A parallelogram has base ${a}, slanted side ${h + 2}, and height ${h}. Find its area.`, diagram: <Fig><Polygon pts={[[40, 170], [220, 170], [280, 70], [100, 70]]} sides={[String(a), String(h + 2)]} /><Seg a={[100, 70]} b={[100, 170]} dash color={ORANGE} /><T x={112} y={120} color={ORANGE}>{h}</T></Fig>,
      answer: num(a * h, { mistakes: [{ value: a * (h + 2), msg: 'You used the slanted side. Area uses the perpendicular HEIGHT.' }] }), hints: ['A = base × height.', 'The height must be perpendicular to the base (the dashed line), not the slanted side.'], explanation: `${a} × ${h} = ${a * h}.`,
    };
    if (t === 'trap') return {
      prompt: `A trapezoid has bases ${a} and ${b} and height ${h}. Find its area.`, diagram: <Fig><Polygon pts={[[40, 180], [280, 180], [220, 70], [90, 70]]} sides={[String(Math.max(a, b)), undefined, String(Math.min(a, b))]} /><Seg a={[150, 70]} b={[150, 180]} dash color={ORANGE} /><T x={162} y={125} color={ORANGE}>{h}</T></Fig>,
      answer: num(((a + b) * h) / 2), hints: ['A = ½(b₁ + b₂)h.', `Add the bases (${a} + ${b}), multiply by ${h}, then halve.`], explanation: `½(${a + b})(${h}) = ${fmt(((a + b) * h) / 2)}.`,
    };
    if (t === 'kite') return {
      prompt: `A kite has diagonals ${a} and ${b}. Find its area.`, answer: num((a * b) / 2),
      hints: ['For kites and rhombuses: A = ½ d₁ d₂.'], explanation: `½(${a})(${b}) = ${fmt((a * b) / 2)}.`,
    };
    if (t === 'circle') {
      const r = ri(2, 12);
      return {
        prompt: `Find the area of a circle with radius ${r}. Nearest tenth (or type ${r * r}π).`, diagram: <CircleFigL parts={['radius']} labels={{ radius: String(r) }} />,
        answer: num(Math.PI * r * r, { mistakes: [{ value: 2 * Math.PI * r, msg: 'That is the circumference. Area is πr².' }] }), hints: ['A = πr².', `Square the radius first: ${r}² = ${r * r}.`], explanation: `π(${r})² = ${r * r}π ≈ ${round(Math.PI * r * r, 1)}.`,
      };
    }
    if (t === 'sector') {
      const r = ri(3, 12), ang = pick([45, 60, 90, 120, 150]);
      const v = (ang / 360) * Math.PI * r * r;
      return {
        prompt: `Find the area of a sector with radius ${r} and central angle ${ang}°. Nearest tenth.`, diagram: <CircleFigL parts={['sector']} labels={{ central: `${ang}°` }} />,
        answer: num(round(v, 1)), hints: ['A sector is a fraction of the whole circle\'s area.', `Fraction = ${ang}/360; whole area = π(${r})².`], explanation: `(${ang}/360)π(${r})² ≈ ${round(v, 2)}.`,
      };
    }
    if (t === 'hole') {
      const s = ri(8, 16), r = ri(1, Math.floor(s / 3));
      const v = s * s - Math.PI * r * r;
      return {
        prompt: `A square ${s} by ${s} has a circular hole of radius ${r} cut out. Find the remaining area to the nearest tenth.`,
        answer: num(round(v, 1)), hints: ['Area remaining = big shape − hole.', `Square: ${s}². Hole: π(${r})².`], explanation: `${s * s} − ${r * r}π ≈ ${round(v, 2)}.`,
      };
    }
    // composite: rectangle + triangle on top (house)
    return {
      prompt: `A "house" shape is a ${a}-by-${b} rectangle with a triangle on top (same base ${a}, height ${h}). Find the total area.`,
      diagram: <Fig><Polygon pts={[[80, 200], [240, 200], [240, 110], [160, 40], [80, 110]]} /><Seg a={[80, 110]} b={[240, 110]} dash /><T x={160} y={215} color={ORANGE}>{a}</T><T x={255} y={155} color={ORANGE}>{b}</T><T x={168} y={80} color={ORANGE}>{h}</T></Fig>,
      answer: num(a * b + (a * h) / 2), hints: ['Split the shape into a rectangle and a triangle.', `Rectangle: ${a} × ${b}. Triangle: ½ × ${a} × ${h}.`, 'Add the two areas.'], explanation: `${a * b} + ${fmt((a * h) / 2)} = ${fmt(a * b + (a * h) / 2)}.`,
    };
  },
};

// ---------- 16. Surface area & volume ----------
const volume: Topic = {
  id: 'geo-16', title: 'Surface Area & Volume',
  lesson: `**Volume** is the space inside a 3-D figure (cubic units). **Surface area** is the total area of all the faces (square units).
• Prism: V = Bh (B = area of the base) • Cylinder: V = πr²h
• Pyramid: V = ⅓Bh • Cone: V = ⅓πr²h (a cone is ⅓ of a cylinder!)
• Sphere: V = (4/3)πr³
Surface area: rectangular prism SA = 2(lw + lh + wh); cylinder SA = 2πr² + 2πrh; sphere SA = 4πr².`,
  lessonDiagram: <Solid kind="cylinder" labels={{ r: 'r', h: 'h' }} />,
  example: {
    problem: 'Find the volume of a cone with radius 3 and height 8.',
    steps: [
      { step: 'V = ⅓πr²h', why: 'A cone holds one third of a cylinder with the same base and height.' },
      { step: 'V = ⅓π(3²)(8) = ⅓π(9)(8)', why: 'Square the radius first.' },
      { step: 'V = 24π ≈ 75.4', why: '⅓ × 72 = 24. Units are cubic.' },
    ],
  },
  vocab: [
    { term: 'Volume', def: 'The amount of space inside a solid, in cubic units.' }, { term: 'Surface area', def: 'The total area of all surfaces of a solid.' },
    { term: 'Prism', def: 'A solid with two congruent parallel bases and rectangular sides.' }, { term: 'Pyramid', def: 'A solid with a polygon base and triangular faces meeting at a point.' },
    { term: 'Cylinder', def: 'A solid with two congruent parallel circular bases.' }, { term: 'Cone', def: 'A solid with a circular base and one vertex.' },
    { term: 'Sphere', def: 'All points in space the same distance from a center.' }, { term: 'Slant height', def: 'The height of a lateral face of a pyramid or cone, measured along the surface.' },
  ],
  formulas: [
    { name: 'Prism volume', f: 'V = Bh' }, { name: 'Cylinder volume', f: 'V = πr²h' }, { name: 'Pyramid volume', f: 'V = ⅓Bh' }, { name: 'Cone volume', f: 'V = ⅓πr²h' },
    { name: 'Sphere volume', f: 'V = (4/3)πr³' }, { name: 'Rect. prism SA', f: '2(lw + lh + wh)' }, { name: 'Cylinder SA', f: '2πr² + 2πrh' }, { name: 'Sphere SA', f: '4πr²' },
  ],
  generate: (d) => {
    const t = d === 0 ? pick(['prismV', 'cubeV']) : d === 1 ? pick(['prismV', 'cylV', 'prismSA', 'pyrV']) : pick(['coneV', 'sphereV', 'cylSA', 'sphereSA', 'pyrV', 'backsolve']);
    const l = ri(2, 12), w = ri(2, 10), h = ri(2, 12), r = ri(2, 9);
    const q = (prompt: string, kind: 'prism' | 'cylinder' | 'cone' | 'sphere' | 'pyramid' | 'cube', labels: Record<string, string>, v: number, hints: string[], expl: string, mistakes: { value: number; msg: string }[] = []): Question => ({
      prompt: prompt + (Number.isInteger(v) ? '' : ' Round to the nearest tenth (or type an answer with π).'), diagram: <Solid kind={kind} labels={labels} />, answer: num(v, { mistakes }), hints, explanation: expl,
    });
    switch (t) {
      case 'cubeV': return q(`Find the volume of a cube with edge ${l}.`, 'cube', { l: String(l), w: String(l), h: String(l) }, l ** 3, ['V = s³ (side × side × side).'], `${l}³ = ${l ** 3}.`, [{ value: 6 * l * l, msg: 'That is the surface area. Volume is s³.' }]);
      case 'prismV': return q(`Find the volume of a rectangular prism ${l} × ${w} × ${h}.`, 'prism', { l: String(l), w: String(w), h: String(h) }, l * w * h, ['V = length × width × height.'], `${l} × ${w} × ${h} = ${l * w * h}.`);
      case 'prismSA': return q(`Find the surface area of a rectangular prism ${l} × ${w} × ${h}.`, 'prism', { l: String(l), w: String(w), h: String(h) }, 2 * (l * w + l * h + w * h), ['A box has 6 faces in 3 matching pairs.', `Find lw, lh, wh, add them, then double.`], `2(${l * w} + ${l * h} + ${w * h}) = ${2 * (l * w + l * h + w * h)}.`, [{ value: l * w * h, msg: 'That is the volume. Surface area adds the areas of the 6 faces.' }]);
      case 'cylV': return q(`Find the volume of a cylinder with radius ${r} and height ${h}.`, 'cylinder', { r: String(r), h: String(h) }, Math.PI * r * r * h, ['V = πr²h — base area times height.', `Base area = π(${r})².`], `π(${r * r})(${h}) = ${r * r * h}π ≈ ${round(Math.PI * r * r * h, 1)}.`, [{ value: 2 * Math.PI * r * h, msg: 'Square the radius: V = πr²h.' }]);
      case 'cylSA': return q(`Find the surface area of a cylinder with radius ${r} and height ${h}.`, 'cylinder', { r: String(r), h: String(h) }, 2 * Math.PI * r * r + 2 * Math.PI * r * h, ['Two circles (top and bottom) + the curved side (a rectangle when unrolled).', 'Circles: 2πr². Side: circumference × height = 2πrh.'], `2π(${r * r}) + 2π(${r})(${h}) = ${2 * r * r + 2 * r * h}π ≈ ${round(2 * Math.PI * r * (r + h), 1)}.`);
      case 'pyrV': { const s = ri(3, 10); return q(`A square pyramid has base side ${s} and height ${h}. Find its volume.`, 'pyramid', { s: String(s), h: String(h) }, (s * s * h) / 3, ['V = ⅓ × (base area) × height.', `Base area = ${s}².`], `⅓(${s * s})(${h}) = ${fmt((s * s * h) / 3)}.`, [{ value: s * s * h, msg: 'Don\'t forget the ⅓ for pyramids.' }]); }
      case 'coneV': return q(`Find the volume of a cone with radius ${r} and height ${h}.`, 'cone', { r: String(r), h: String(h) }, (Math.PI * r * r * h) / 3, ['A cone is ⅓ of a cylinder: V = ⅓πr²h.'], `⅓π(${r * r})(${h}) ≈ ${round((Math.PI * r * r * h) / 3, 1)}.`, [{ value: Math.PI * r * r * h, msg: 'That is the cylinder volume. A cone is ⅓ of it.' }]);
      case 'sphereV': return q(`Find the volume of a sphere with radius ${r}.`, 'sphere', { r: String(r) }, (4 / 3) * Math.PI * r ** 3, ['V = (4/3)πr³.', `Cube the radius first: ${r}³ = ${r ** 3}.`], `(4/3)π(${r ** 3}) ≈ ${round((4 / 3) * Math.PI * r ** 3, 1)}.`);
      case 'sphereSA': return q(`Find the surface area of a sphere with radius ${r}.`, 'sphere', { r: String(r) }, 4 * Math.PI * r * r, ['SA = 4πr².'], `4π(${r * r}) = ${4 * r * r}π ≈ ${round(4 * Math.PI * r * r, 1)}.`);
      default: {
        const V = l * w * h;
        return {
          prompt: `A rectangular prism has volume ${V}, length ${l}, and width ${w}. Find its height.`, diagram: <Solid kind="prism" labels={{ l: String(l), w: String(w), h: '?' }} />,
          answer: num(h), hints: ['V = lwh. Plug in what you know.', `${V} = ${l} × ${w} × h. What do you divide by?`], explanation: `h = ${V} ÷ ${l * w} = ${h}.`,
        };
      }
    }
  },
};

export const GEOMETRY_TOPICS: Topic[] = [foundations, angles, parallel, logic, triangles, congruent, proofs, similarity, right, trig, polygons, transforms, coordinate, circles, area, volume];
export const GEOMETRY_IDS = GEOMETRY_TOPICS.map((t) => t.id);
export { pickLetters };
