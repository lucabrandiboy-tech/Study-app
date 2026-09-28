/**
 * Super Advanced: everything one grade ahead (9th grade / Algebra II level) of the normal Study Zone.
 */
import type { Difficulty } from '../lib/store';
import type { Question, Topic } from './types';
import { bankTopic } from './bank';
import { ri, pick, round, fmt, mc } from './rand';

const num = (value: number, prompt: string, hints: string[], explanation: string, mistakes: { value: number; msg: string }[] = []): Question =>
  ({ prompt, answer: { kind: 'number', value, mistakes }, hints, explanation });
const sgn = (n: number) => (n < 0 ? `− ${-n}` : `+ ${n}`);
const nz = (a: number, b: number) => { let v = 0; while (v === 0) v = ri(a, b); return v; };

// ======================= ALGEBRA II / PRE-CALC =======================
const gen = (id: string, title: string, lesson: string, example: Topic['example'], vocab: Topic['vocab'], formulas: Topic['formulas'], g: (d: Difficulty) => Question): Topic =>
  ({ id, title, lesson, example, vocab, formulas, generate: g });

export const ADV_MATH: Topic[] = [
  gen('adv-quad', 'Quadratic Equations',
    `A **quadratic** has the form ax² + bx + c = 0. Ways to solve it:
• **Factor**: x² − 5x + 6 = (x − 2)(x − 3) = 0, so x = 2 or x = 3 (Zero Product Property).
• **Quadratic formula**: x = (−b ± √(b² − 4ac)) / 2a.
The **discriminant** D = b² − 4ac tells how many real solutions: D > 0 → two, D = 0 → one, D < 0 → none (two complex).
The **vertex** of y = ax² + bx + c is at x = −b / 2a.`,
    { problem: 'Solve x² + 2x − 15 = 0.', steps: [
      { step: 'Find two numbers that multiply to −15 and add to 2: 5 and −3.', why: 'That lets us factor the trinomial.' },
      { step: '(x + 5)(x − 3) = 0', why: 'Write the factored form.' },
      { step: 'x = −5 or x = 3', why: 'Zero Product Property: set each factor equal to 0.' }] },
    [{ term: 'Quadratic', def: 'A polynomial of degree 2: ax² + bx + c.' }, { term: 'Discriminant', def: 'b² − 4ac; tells the number of real roots.' }, { term: 'Vertex', def: 'The turning point of a parabola, at x = −b/2a.' }, { term: 'Root / zero', def: 'An x-value that makes the expression equal 0.' }, { term: 'Zero Product Property', def: 'If ab = 0, then a = 0 or b = 0.' }],
    [{ name: 'Quadratic formula', f: 'x = (−b ± √(b² − 4ac)) / 2a' }, { name: 'Discriminant', f: 'D = b² − 4ac' }, { name: 'Vertex x', f: 'x = −b / 2a' }],
    (d) => {
      const t = d === 0 ? 'factor' : d === 1 ? pick(['factor', 'disc', 'vertex']) : pick(['formula', 'disc', 'vertex', 'factorA']);
      if (t === 'factor' || t === 'factorA') {
        const a = t === 'factorA' ? pick([2, 3]) : 1;
        const r1 = nz(-9, 9), r2 = nz(-9, 9);
        const b = -a * (r1 + r2), c = a * r1 * r2;
        return num(Math.max(r1, r2), `Solve ${a === 1 ? '' : a}x² ${sgn(b)}x ${sgn(c)} = 0. Enter the LARGER solution.`,
          [a > 1 ? `First divide every term by ${a}.` : 'Look for two numbers that multiply to c and add to b.', `They multiply to ${r1 * r2} and add to ${-(r1 + r2)}.`, 'Set each factor equal to zero.'],
          `${a === 1 ? '' : `${a}`}(x ${sgn(-r1)})(x ${sgn(-r2)}) = 0 → x = ${r1} or x = ${r2}. Larger: ${Math.max(r1, r2)}.`,
          [{ value: -Math.max(r1, r2), msg: 'Sign flip! If the factor is (x − 3), the solution is +3.' }]);
      }
      if (t === 'disc') {
        const a = nz(-3, 4), b = ri(-8, 8), c = ri(-9, 9);
        const D = b * b - 4 * a * c;
        return { prompt: `How many real solutions does ${a}x² ${sgn(b)}x ${sgn(c)} = 0 have?`, answer: mc(D > 0 ? 'Two' : D === 0 ? 'One' : 'None', ['Two', 'One', 'None']),
          hints: ['Compute the discriminant D = b² − 4ac.', `D = (${b})² − 4(${a})(${c}).`], explanation: `D = ${D}. ${D > 0 ? 'Positive → two real solutions.' : D === 0 ? 'Zero → one real solution.' : 'Negative → no real solutions.'}` };
      }
      if (t === 'vertex') {
        const a = nz(-3, 3), h = ri(-6, 6), k = ri(-9, 9);
        const b = -2 * a * h, c = a * h * h + k;
        const askY = d >= 2 && Math.random() < 0.5;
        return num(askY ? k : h, `Find the ${askY ? 'y' : 'x'}-coordinate of the vertex of y = ${a}x² ${sgn(b)}x ${sgn(c)}.`,
          ['The vertex x-value is −b / (2a).', `−(${b}) / (2 · ${a})`, ...(askY ? ['Plug that x back into the equation to get y.'] : [])],
          `x = ${h}, y = ${k}. Vertex (${h}, ${k}).`, [{ value: -h, msg: 'Watch the sign: x = −b / 2a.' }]);
      }
      const a = nz(1, 3), b = ri(-9, 9), c = ri(-9, 2);
      const D = b * b - 4 * a * c;
      if (D <= 0 || Number.isInteger(Math.sqrt(D))) return ADV_MATH[0].generate(d);
      const x = (-b + Math.sqrt(D)) / (2 * a);
      return num(round(x, 2), `Use the quadratic formula to solve ${a}x² ${sgn(b)}x ${sgn(c)} = 0. Enter the solution using "+" (larger one), rounded to 2 decimals.`,
        ['a, b, c = ' + `${a}, ${b}, ${c}.`, `Discriminant: b² − 4ac = ${b * b} − ${4 * a * c}.`, 'x = (−b + √D) / 2a.'], `D = ${D}; x = (${-b} + √${D}) / ${2 * a} ≈ ${round(x, 2)}.`);
    }),
  gen('adv-systems', 'Systems of Equations',
    `A **system** is two or more equations that share variables. The solution is where the lines cross.
• **Substitution**: solve one equation for a variable, plug it into the other.
• **Elimination**: add or subtract equations to cancel a variable.
If the lines are parallel there is **no solution**; if they're the same line there are **infinitely many**.`,
    { problem: 'Solve: x + y = 10 and x − y = 4.', steps: [
      { step: 'Add the equations: 2x = 14', why: 'The y terms cancel (elimination).' },
      { step: 'x = 7', why: 'Divide by 2.' },
      { step: '7 + y = 10 → y = 3', why: 'Substitute back into the first equation.' }] },
    [{ term: 'System of equations', def: 'Two or more equations with the same variables.' }, { term: 'Substitution', def: 'Replace a variable with an equivalent expression.' }, { term: 'Elimination', def: 'Add/subtract equations to cancel a variable.' }, { term: 'Inconsistent system', def: 'A system with no solution (parallel lines).' }],
    [{ name: 'Elimination', f: 'Multiply so coefficients cancel, then add' }],
    (d) => {
      const x = ri(-8, 8), y = ri(-8, 8);
      const a1 = d === 0 ? 1 : nz(-4, 4), b1 = d === 0 ? 1 : nz(-4, 4), a2 = d === 0 ? 1 : nz(-4, 4), b2 = d === 0 ? -1 : nz(-4, 4);
      if (a1 * b2 - a2 * b1 === 0) return ADV_MATH[1].generate(d);
      const askY = Math.random() < 0.5;
      const eq = (a: number, b: number) => `${a === 1 ? '' : a === -1 ? '−' : a}x ${b < 0 ? '−' : '+'} ${Math.abs(b) === 1 ? '' : Math.abs(b)}y = ${a * x + b * y}`;
      return num(askY ? y : x, `Solve the system and enter ${askY ? 'y' : 'x'}:\n${eq(a1, b1)}\n${eq(a2, b2)}`,
        ['Pick a variable to eliminate. Can you multiply one equation so the x (or y) coefficients are opposites?', 'Add the equations to cancel that variable, then solve for the other.', 'Substitute back to find the second variable.'],
        `x = ${x}, y = ${y}.`, [{ value: askY ? x : y, msg: `That's ${askY ? 'x' : 'y'} — the question asks for ${askY ? 'y' : 'x'}.` }]);
    }),
  gen('adv-exp', 'Exponents & Exponential Functions',
    `**Exponent rules**: aᵐ · aⁿ = aᵐ⁺ⁿ, aᵐ / aⁿ = aᵐ⁻ⁿ, (aᵐ)ⁿ = aᵐⁿ, a⁰ = 1, a⁻ⁿ = 1/aⁿ.
**Exponential growth/decay**: y = a · bˣ. If b > 1 it grows; if 0 < b < 1 it decays.
Percent growth: y = P(1 + r)ᵗ. Percent decay: y = P(1 − r)ᵗ.`,
    { problem: 'A town of 5,000 grows 4% per year. Population after 3 years?', steps: [
      { step: 'y = 5000(1.04)³', why: 'Growth factor is 1 + 0.04.' },
      { step: '1.04³ ≈ 1.1249', why: 'Compute the power first.' },
      { step: 'y ≈ 5624', why: 'Multiply.' }] },
    [{ term: 'Exponential growth', def: 'Increase by the same factor each period (b > 1).' }, { term: 'Exponential decay', def: 'Decrease by the same factor each period (0 < b < 1).' }, { term: 'Growth factor', def: 'The base b; equals 1 + r for r% growth.' }, { term: 'Negative exponent', def: 'a⁻ⁿ = 1 / aⁿ.' }],
    [{ name: 'Product rule', f: 'aᵐ·aⁿ = aᵐ⁺ⁿ' }, { name: 'Quotient rule', f: 'aᵐ/aⁿ = aᵐ⁻ⁿ' }, { name: 'Power rule', f: '(aᵐ)ⁿ = aᵐⁿ' }, { name: 'Growth', f: 'y = P(1 + r)ᵗ' }],
    (d) => {
      const t = d === 0 ? 'rules' : pick(['rules', 'growth', 'neg']);
      if (t === 'rules') {
        const m = ri(2, 9), n = ri(2, 9), k = pick(['mul', 'div', 'pow'] as const);
        const v = k === 'mul' ? m + n : k === 'div' ? m - n : m * n;
        return num(v, `Simplify ${k === 'mul' ? `x^${m} · x^${n}` : k === 'div' ? `x^${m} / x^${n}` : `(x^${m})^${n}`} = x^?. Enter the exponent.`,
          [k === 'mul' ? 'Same base multiplied → add exponents.' : k === 'div' ? 'Same base divided → subtract exponents.' : 'Power to a power → multiply exponents.'], `x^${v}.`,
          [{ value: k === 'mul' ? m * n : m + n, msg: 'Check which rule applies: multiplying powers ADDS exponents; a power of a power MULTIPLIES them.' }]);
      }
      if (t === 'neg') { const b = ri(2, 5), n = ri(1, 3); return num(1 / b ** n, `Evaluate ${b}^(−${n}). (A fraction like 1/8 is fine.)`, ['A negative exponent means reciprocal.', `${b}^(−${n}) = 1 / ${b}^${n}.`], `1/${b ** n}.`, [{ value: -(b ** n), msg: 'A negative exponent does not make the answer negative — it flips it into a fraction.' }]); }
      const P = ri(10, 90) * 100, r = ri(2, 12), yrs = ri(2, 8), decay = Math.random() < 0.4;
      const v = P * (decay ? 1 - r / 100 : 1 + r / 100) ** yrs;
      return num(Math.round(v), `A ${decay ? 'car worth $' + P + ' loses' : 'savings account with $' + P + ' grows'} ${r}% per year. What is it worth after ${yrs} years? (Round to the nearest whole number.)`,
        [`Growth factor = ${decay ? `1 − ${r / 100}` : `1 + ${r / 100}`}.`, `Value = ${P} × (factor)^${yrs}.`], `${P}(${decay ? 1 - r / 100 : 1 + r / 100})^${yrs} ≈ ${Math.round(v)}.`,
        [{ value: Math.round(P * (1 + (decay ? -1 : 1) * (r / 100) * yrs)), msg: 'That is simple (linear) change. Exponential change multiplies by the factor every year.' }]);
    }),
  gen('adv-log', 'Logarithms',
    `A **logarithm** answers "what exponent?": log_b(x) = y means bʸ = x.
Examples: log₂(8) = 3 because 2³ = 8. log₁₀(1000) = 3.
Rules: log(xy) = log x + log y, log(x/y) = log x − log y, log(xⁿ) = n·log x.
Use logs to solve for exponents: 2ˣ = 20 → x = log(20)/log(2) ≈ 4.32.`,
    { problem: 'Evaluate log₃(81).', steps: [{ step: 'Ask: 3 to what power is 81?', why: 'That is what a logarithm means.' }, { step: '3⁴ = 81, so log₃(81) = 4', why: '3·3·3·3 = 81.' }] },
    [{ term: 'Logarithm', def: 'The exponent you raise the base to, to get a number.' }, { term: 'Common log', def: 'Base-10 logarithm, written log x.' }, { term: 'Natural log', def: 'Base-e logarithm, written ln x.' }, { term: 'Change of base', def: 'log_b(x) = log(x) / log(b).' }],
    [{ name: 'Definition', f: 'log_b(x) = y ⇔ bʸ = x' }, { name: 'Change of base', f: 'log_b(x) = log x / log b' }, { name: 'Power rule', f: 'log(xⁿ) = n log x' }],
    (d) => {
      if (d <= 1 || Math.random() < 0.5) { const b = pick([2, 3, 4, 5, 10]), y = ri(d === 0 ? 1 : -2, 5); const x = b ** y; return num(y, `Evaluate log${b === 10 ? '' : `_${b}`}(${y < 0 ? `1/${b ** -y}` : x}).`, [`Ask: ${b} to what power equals ${y < 0 ? `1/${b ** -y}` : x}?`, y < 0 ? 'A fraction means a negative exponent.' : `Try multiplying ${b} by itself.`], `${b}^${y} = ${y < 0 ? `1/${b ** -y}` : x}, so the answer is ${y}.`); }
      const b = pick([2, 3, 5]), T = ri(6, 90);
      const v = Math.log(T) / Math.log(b);
      if (Number.isInteger(round(v, 6))) return ADV_MATH[3].generate(d);
      return num(round(v, 2), `Solve ${b}^x = ${T}. Round to 2 decimals.`, ['Take the log of both sides.', `x · log(${b}) = log(${T}).`, 'Divide both sides by log(' + b + ').'], `x = log ${T} / log ${b} ≈ ${round(v, 2)}.`);
    }),
  gen('adv-func', 'Functions: Composition & Inverses',
    `A **function** f(x) gives exactly one output for each input.
**Composition**: (f ∘ g)(x) = f(g(x)) — do g first, then f.
**Inverse** f⁻¹ undoes f: if f(x) = 2x + 3, swap x and y and solve: f⁻¹(x) = (x − 3)/2.
**Domain** = allowed inputs; **range** = possible outputs.`,
    { problem: 'f(x) = 3x − 1, g(x) = x². Find f(g(2)).', steps: [{ step: 'g(2) = 4', why: 'Work from the inside out.' }, { step: 'f(4) = 3(4) − 1 = 11', why: 'Plug the inside result into f.' }] },
    [{ term: 'Function', def: 'A rule assigning exactly one output to each input.' }, { term: 'Composition', def: 'Using one function\'s output as another\'s input: f(g(x)).' }, { term: 'Inverse function', def: 'A function that undoes the original.' }, { term: 'Domain', def: 'The set of allowed input values.' }, { term: 'Range', def: 'The set of output values.' }],
    [{ name: 'Composition', f: '(f∘g)(x) = f(g(x))' }, { name: 'Inverse check', f: 'f(f⁻¹(x)) = x' }],
    (d) => {
      const a = nz(-4, 5), b = ri(-9, 9), c = nz(-3, 3), e = ri(-5, 5), x = nz(-4, 4);
      const fx = `${a === 1 ? '' : a === -1 ? '−' : a}x${b ? ` ${sgn(b)}` : ''}`;
      const gx = `${c === 1 ? '' : c === -1 ? '−' : c}x²${e ? ` ${sgn(e)}` : ''}`;
      const f = (v: number) => a * v + b, g = (v: number) => c * v * v + e;
      const t = d === 0 ? 'eval' : pick(['comp', 'comp2', 'inv']);
      if (t === 'eval') return num(f(x), `f(x) = ${fx}. Find f(${x}).`, ['Replace every x with the input.', `${a}(${x}) ${sgn(b)}`], `f(${x}) = ${f(x)}.`);
      if (t === 'comp') return num(f(g(x)), `f(x) = ${fx} and g(x) = ${gx}. Find f(g(${x})).`, ['Work from the inside out: find g first.', `g(${x}) = ${c}(${x})² ${sgn(e)}. Then put that into f.`], `g(${x}) = ${g(x)}, f(${g(x)}) = ${f(g(x))}.`, [{ value: g(f(x)), msg: 'That is g(f(x)) — the order matters. Do g first.' }]);
      if (t === 'comp2') return num(g(f(x)), `f(x) = ${fx} and g(x) = ${gx}. Find g(f(${x})).`, ['Inside first: find f(' + x + ').', 'Then plug that result into g.'], `g(${f(x)}) = ${g(f(x))}.`, [{ value: f(g(x)), msg: 'That is f(g(x)). Here f goes first.' }]);
      const y = ri(-10, 20);
      if (y === b || (y - b) / a === y) return ADV_MATH[4].generate(d);
      return num((y - b) / a, `f(x) = ${fx}. Find f⁻¹(${y}).`, ['f⁻¹(y) asks: which input to f gives this output?', `Solve ${fx} = ${y}.`], `x = (${y} ${sgn(-b)}) / ${a} = ${fmt((y - b) / a)}.`);
    }),
  gen('adv-seq', 'Sequences & Series',
    `An **arithmetic sequence** adds the same **common difference** d: aₙ = a₁ + (n − 1)d.
A **geometric sequence** multiplies by the same **common ratio** r: aₙ = a₁ · rⁿ⁻¹.
Sum of the first n terms of an arithmetic series: Sₙ = n(a₁ + aₙ)/2.`,
    { problem: 'Find the 20th term of 3, 7, 11, 15, …', steps: [{ step: 'd = 4, a₁ = 3', why: 'Each term adds 4.' }, { step: 'a₂₀ = 3 + 19·4 = 79', why: 'Use aₙ = a₁ + (n − 1)d.' }] },
    [{ term: 'Arithmetic sequence', def: 'Each term adds a constant difference.' }, { term: 'Geometric sequence', def: 'Each term is multiplied by a constant ratio.' }, { term: 'Series', def: 'The sum of the terms of a sequence.' }],
    [{ name: 'Arithmetic nth term', f: 'aₙ = a₁ + (n−1)d' }, { name: 'Geometric nth term', f: 'aₙ = a₁·rⁿ⁻¹' }, { name: 'Arithmetic sum', f: 'Sₙ = n(a₁ + aₙ)/2' }],
    (d) => {
      const t = d === 0 ? 'arith' : pick(['arith', 'geo', 'sum']);
      const a1 = ri(-5, 12), dd = nz(-6, 8), n = ri(8, 30);
      if (t === 'arith') return num(a1 + (n - 1) * dd, `Find term #${n} of ${[0, 1, 2, 3].map((i) => a1 + i * dd).join(', ')}, …`, ['Find the common difference d.', 'Use aₙ = a₁ + (n − 1)d.'], `${a1} + ${n - 1}(${dd}) = ${a1 + (n - 1) * dd}.`, [{ value: a1 + n * dd, msg: 'Off by one! Use (n − 1), since the first term already counts.' }]);
      if (t === 'sum') { const an = a1 + (n - 1) * dd; return num((n * (a1 + an)) / 2, `Find the sum of the first ${n} terms of ${[0, 1, 2].map((i) => a1 + i * dd).join(', ')}, …`, ['First find the last term aₙ.', `Then Sₙ = n(a₁ + aₙ)/2.`], `a${n} = ${an}; S = ${n}(${a1} + ${an})/2 = ${(n * (a1 + an)) / 2}.`); }
      const g1 = ri(1, 5), r = pick([2, 3, -2]), m = ri(5, 8);
      return num(g1 * r ** (m - 1), `Find term #${m} of ${[0, 1, 2, 3].map((i) => g1 * r ** i).join(', ')}, …`, ['Divide a term by the one before it to find r.', `aₙ = a₁ · r^(n−1).`], `${g1} · ${r}^${m - 1} = ${g1 * r ** (m - 1)}.`);
    }),
];

// ======================= PHYSICS & CHEMISTRY =======================
const molar = { H: 1.008, C: 12.01, N: 14.01, O: 16.0, Na: 22.99, Cl: 35.45, Ca: 40.08 };
const COMPOUNDS: [string, [keyof typeof molar, number][]][] = [['H₂O', [['H', 2], ['O', 1]]], ['CO₂', [['C', 1], ['O', 2]]], ['NaCl', [['Na', 1], ['Cl', 1]]], ['CH₄', [['C', 1], ['H', 4]]], ['NH₃', [['N', 1], ['H', 3]]], ['CaCO₃', [['Ca', 1], ['C', 1], ['O', 3]]], ['C₆H₁₂O₆', [['C', 6], ['H', 12], ['O', 6]]]];
const mm = (f: [keyof typeof molar, number][]) => f.reduce((s, [e, n]) => s + molar[e] * n, 0);

export const ADV_SCI: Topic[] = [
  bankTopic({
    id: 'adv-chem', title: 'Chemistry: Moles & Molar Mass',
    lesson: `A **mole** is 6.022 × 10²³ particles (Avogadro's number) — a "chemist's dozen."
**Molar mass** (g/mol) = sum of the atomic masses in a formula. H₂O = 2(1.008) + 16.00 = 18.02 g/mol.
Convert: **moles = grams ÷ molar mass**; grams = moles × molar mass.
The **periodic table** is arranged by atomic number; columns (groups) share properties because they have the same number of **valence electrons**.
Atomic masses used here: H 1.008, C 12.01, N 14.01, O 16.00, Na 22.99, Cl 35.45, Ca 40.08.`,
    example: { problem: 'How many moles are in 36.04 g of water?', steps: [{ step: 'Molar mass H₂O = 18.02 g/mol', why: '2(1.008) + 16.00.' }, { step: '36.04 ÷ 18.02 = 2.00 mol', why: 'moles = grams ÷ molar mass.' }] },
    vocab: [{ term: 'Mole', def: '6.022 × 10²³ particles of a substance.' }, { term: 'Molar mass', def: 'Mass of one mole of a substance, in g/mol.' }, { term: 'Valence electrons', def: 'Electrons in the outermost shell; determine bonding.' }, { term: 'Ionic bond', def: 'Attraction between oppositely charged ions (metal + nonmetal).' }, { term: 'Covalent bond', def: 'Atoms sharing electrons (nonmetals).' }],
    questions: [
      ['Which bond forms in NaCl?', 'Ionic', ['Covalent', 'Metallic', 'Hydrogen'], 'Na is a metal, Cl is a nonmetal — electrons are transferred.'],
      ['Elements in the same group have the same number of…', 'Valence electrons', ['Neutrons', 'Protons', 'Isotopes'], 'That is why they react similarly.'],
      ['Noble gases are unreactive because…', 'Their outer shell is full', ['They are metals', 'They have no electrons', 'They are radioactive'], 'A full valence shell is stable.'],
      ['A catalyst…', 'Speeds up a reaction without being used up', ['Is a product', 'Slows a reaction', 'Is always a liquid'], 'It lowers the activation energy.', 1],
    ],
    gens: [
      () => { const [n, f] = pick(COMPOUNDS); return num(round(mm(f), 2), `Find the molar mass of ${n} in g/mol (2 decimals).`, ['Add the atomic mass of every atom in the formula.', 'Multiply each element\'s mass by its subscript.'], `${f.map(([e, k]) => `${k}(${molar[e]})`).join(' + ')} = ${round(mm(f), 2)} g/mol.`); },
      () => { const [n, f] = pick(COMPOUNDS); const mol = ri(1, 12) / 2; const g = round(mol * mm(f), 2); return num(mol, `How many moles are in ${g} g of ${n}?`, ['First find the molar mass of the compound.', 'moles = grams ÷ molar mass.'], `${g} ÷ ${round(mm(f), 2)} = ${mol} mol.`, [{ value: round(g * mm(f), 1), msg: 'You multiplied. To go from grams to moles, divide by molar mass.' }]); },
    ],
  }),
  bankTopic({
    id: 'adv-phys', title: 'Physics: Momentum, Work & Power',
    lesson: `**Momentum** p = m × v (kg·m/s). In a collision, total momentum is **conserved**.
**Work** W = F × d (joules) — force times distance moved in the force's direction.
**Power** P = W ÷ t (watts) — how fast work is done.
**Acceleration** a = (v_final − v_initial) ÷ t.
**Ohm's law** for circuits: V = I × R (volts = amps × ohms).`,
    example: { problem: 'A 60 kg skater moving 3 m/s: momentum?', steps: [{ step: 'p = m·v', why: 'Momentum formula.' }, { step: 'p = 60 × 3 = 180 kg·m/s', why: 'Multiply.' }] },
    vocab: [{ term: 'Momentum', def: 'Mass times velocity.' }, { term: 'Work', def: 'Force times distance in the direction of the force.' }, { term: 'Power', def: 'Rate of doing work (W ÷ t).' }, { term: 'Resistance', def: 'Opposition to electric current, in ohms.' }, { term: 'Current', def: 'Flow of electric charge, in amperes.' }],
    questions: [
      ['In a closed system, total momentum…', 'Stays the same', ['Always increases', 'Always decreases', 'Becomes zero'], 'Conservation of momentum.'],
      ['Which unit measures power?', 'Watt', ['Joule', 'Newton', 'Ohm'], 'Joules per second.'],
      ['If resistance goes up and voltage stays the same, current…', 'Decreases', ['Increases', 'Stays the same', 'Doubles'], 'I = V ÷ R.', 1],
      ['Carrying a box across a room at constant height does how much work on the box (physics)?', 'Zero', ['A lot', 'Depends on speed', 'Negative'], 'The force (up) is perpendicular to the motion (sideways).', 2],
    ],
    gens: [
      () => { const m = ri(2, 90), v = ri(1, 20); return num(m * v, `Find the momentum of a ${m} kg object moving at ${v} m/s.`, ['p = m × v.'], `${m} × ${v} = ${m * v} kg·m/s.`); },
      () => { const F = ri(5, 200), dd = ri(2, 30), t = ri(2, 20); return num(round((F * dd) / t, 2), `A ${F} N force pushes a crate ${dd} m in ${t} s. What is the power (W)? (2 decimals)`, ['First find work: W = F × d.', 'Then power = work ÷ time.'], `W = ${F * dd} J; P = ${F * dd}/${t} ≈ ${round((F * dd) / t, 2)} W.`, [{ value: F * dd, msg: 'That is the work. Power divides work by time.' }]); },
      () => { const I = ri(1, 10), R = ri(2, 50); return num(I * R, `A current of ${I} A flows through a ${R} Ω resistor. What is the voltage?`, ["Ohm's law: V = I × R."], `${I} × ${R} = ${I * R} V.`); },
      () => { const m1 = ri(1, 10), v1 = ri(2, 12), m2 = ri(1, 10); return num(round((m1 * v1) / (m1 + m2), 2), `A ${m1} kg cart moving ${v1} m/s hits a still ${m2} kg cart and they stick together. Their speed after? (2 decimals)`, ['Momentum before = momentum after.', `Before: ${m1} × ${v1}. After: (${m1} + ${m2}) × v.`], `v = ${m1 * v1} / ${m1 + m2} ≈ ${round((m1 * v1) / (m1 + m2), 2)} m/s.`); },
    ],
  }),
  bankTopic({
    id: 'adv-bio-cells', title: 'Biology: Cells & Energy',
    lesson: `**Prokaryotic** cells (bacteria) have no nucleus. **Eukaryotic** cells (plants, animals, fungi) have a nucleus and organelles.
Key organelles: **nucleus** (DNA), **mitochondria** (cellular respiration), **chloroplasts** (photosynthesis, plants only), **ribosomes** (make proteins), **cell membrane** (controls what enters/leaves).
**Photosynthesis**: 6CO₂ + 6H₂O + light → C₆H₁₂O₆ + 6O₂.
**Cellular respiration**: C₆H₁₂O₆ + 6O₂ → 6CO₂ + 6H₂O + ATP (energy).
**Diffusion** moves particles from high to low concentration; **osmosis** is diffusion of water.`,
    example: { problem: 'Why do plant cells need both chloroplasts AND mitochondria?', steps: [{ step: 'Chloroplasts make glucose (store energy).', why: 'Photosynthesis builds food from light.' }, { step: 'Mitochondria break glucose down into ATP.', why: 'Cells can only use energy as ATP.' }] },
    vocab: [{ term: 'Mitochondria', def: 'Organelle that makes ATP through cellular respiration.' }, { term: 'Chloroplast', def: 'Organelle where photosynthesis occurs.' }, { term: 'Ribosome', def: 'Organelle that builds proteins.' }, { term: 'ATP', def: 'The energy molecule cells use.' }, { term: 'Osmosis', def: 'Diffusion of water across a membrane.' }, { term: 'Prokaryote', def: 'A cell without a nucleus.' }],
    questions: [
      ['Which organelle is found in plant cells but NOT animal cells?', 'Chloroplast', ['Mitochondria', 'Ribosome', 'Nucleus'], 'Animals don\'t photosynthesize.'],
      ['A product of photosynthesis is…', 'Oxygen', ['Carbon dioxide', 'ATP only', 'Nitrogen'], 'Look at the right side of the equation.'],
      ['A cell placed in very salty water will…', 'Shrink as water leaves', ['Swell and burst', 'Stay the same', 'Divide faster'], 'Water moves toward higher solute concentration.', 1],
      ['Bacteria are…', 'Prokaryotes', ['Eukaryotes', 'Viruses', 'Plants'], 'They have no nucleus.'],
      ['Where does cellular respiration mostly happen?', 'Mitochondria', ['Chloroplasts', 'Nucleus', 'Cell wall'], 'The "powerhouse."'],
    ],
  }),
  bankTopic({
    id: 'adv-bio-dna', title: 'Biology: DNA, Genetics & Evolution',
    lesson: `**DNA** is a double helix. Base pairs: **A–T** and **C–G**. In RNA, T is replaced by **U**.
**Protein synthesis**: DNA → (transcription) → mRNA → (translation at ribosomes) → protein. Every 3 bases (a **codon**) code for one amino acid.
**Mitosis** makes 2 identical body cells; **meiosis** makes 4 unique sex cells with half the chromosomes.
A **dihybrid cross** (AaBb × AaBb) gives a 9:3:3:1 ratio.
**Natural selection**: individuals with helpful traits survive and reproduce more, so the traits become common over generations.`,
    example: { problem: 'DNA strand: TAC GGA. Write the mRNA.', steps: [{ step: 'Pair each base: T→A, A→U, C→G, G→C.', why: 'mRNA uses U instead of T.' }, { step: 'AUG CCU', why: 'Transcribe each base.' }] },
    vocab: [{ term: 'Codon', def: 'Three mRNA bases that code for one amino acid.' }, { term: 'Transcription', def: 'Copying DNA into mRNA.' }, { term: 'Translation', def: 'Building a protein from mRNA at a ribosome.' }, { term: 'Meiosis', def: 'Cell division that makes gametes with half the chromosomes.' }, { term: 'Natural selection', def: 'Organisms better suited to their environment survive and reproduce more.' }, { term: 'Adaptation', def: 'An inherited trait that helps survival.' }],
    questions: [
      ['The DNA base A pairs with ___ in mRNA.', 'U', ['T', 'G', 'C'], 'RNA has uracil instead of thymine.'],
      ['Meiosis produces…', '4 unique cells with half the chromosomes', ['2 identical cells', '4 identical cells', '1 large cell'], 'It makes eggs and sperm.'],
      ['How many bases make one codon?', '3', ['2', '4', '1'], 'Think "triplet."'],
      ['In AaBb × AaBb, what fraction shows both recessive traits (aabb)?', '1/16', ['1/4', '9/16', '3/16'], 'The 9:3:3:1 ratio out of 16.', 1],
      ['Antibiotic resistance in bacteria is an example of…', 'Natural selection', ['Photosynthesis', 'Mitosis', 'Transcription'], 'Resistant bacteria survive and multiply.'],
      ['Complementary DNA strand for ATGC?', 'TACG', ['UACG', 'ATGC', 'GCAT'], 'A↔T and C↔G.', 1],
    ],
  }),
];

// ======================= ENGLISH 9 =======================
export const ADV_ENG: Topic[] = [
  bankTopic({
    id: 'adv-rhetoric', title: 'Rhetoric & Argument',
    lesson: `Persuasive writers use three appeals:
**Ethos** — credibility ("As a doctor with 20 years of experience…").
**Pathos** — emotion ("Imagine a child going to bed hungry…").
**Logos** — logic and evidence ("Studies show a 40% drop…").
Watch for **logical fallacies**: **ad hominem** (attacking the person), **straw man** (distorting the other side), **bandwagon** ("everyone does it"), **false dilemma** (only two choices), **slippery slope** (one step leads to disaster).`,
    example: { problem: '"Nine out of ten dentists recommend Brite toothpaste." Which appeal?', steps: [{ step: 'It cites experts and a statistic.', why: 'Experts suggest credibility; numbers suggest logic.' }, { step: 'Mainly ethos (authority of dentists), supported by logos.', why: 'Ads often mix appeals.' }] },
    vocab: [{ term: 'Ethos', def: 'Appeal to credibility or character.' }, { term: 'Pathos', def: 'Appeal to emotion.' }, { term: 'Logos', def: 'Appeal to logic and evidence.' }, { term: 'Ad hominem', def: 'Attacking the person instead of the argument.' }, { term: 'Straw man', def: 'Misrepresenting an argument to make it easier to attack.' }, { term: 'False dilemma', def: 'Presenting only two options when more exist.' }, { term: 'Bandwagon', def: 'Arguing something is right because many people do it.' }],
    questions: [
      ['"You can\'t trust her plan — she failed math in 5th grade."', 'Ad hominem', ['Straw man', 'Bandwagon', 'Logos'], 'It attacks the person, not the plan.'],
      ['"Either we ban phones in school or grades will never improve."', 'False dilemma', ['Slippery slope', 'Ethos', 'Ad hominem'], 'Only two options are offered.'],
      ['"Picture the lonely puppy shivering in the rain…"', 'Pathos', ['Ethos', 'Logos', 'Straw man'], 'It targets feelings.'],
      ['"Crime fell 12% in cities that added streetlights."', 'Logos', ['Pathos', 'Ethos', 'Bandwagon'], 'It uses data.'],
      ['"If we let students redo one test, soon nobody will study at all."', 'Slippery slope', ['False dilemma', 'Straw man', 'Ethos'], 'One small step supposedly leads to disaster.', 1],
    ],
  }),
  bankTopic({
    id: 'adv-litanalysis', title: 'Literary Analysis',
    lesson: `Analyze HOW an author creates meaning, not just WHAT happens.
**Characterization**: direct (the narrator tells us) vs. indirect (speech, thoughts, actions, looks — STEAL).
**Irony**: verbal (saying the opposite), situational (the unexpected happens), dramatic (the reader knows what a character doesn't).
**Symbolism**: an object standing for an idea. **Foreshadowing**: hints of what's coming. **Tone** = author's attitude; **mood** = reader's feeling.
Write analysis with **claim → evidence (quote) → commentary** explaining how the quote supports the claim.`,
    example: { problem: 'A fire station burns down. What kind of irony?', steps: [{ step: 'Is it a statement? No.', why: 'Rules out verbal irony.' }, { step: 'It is the opposite of what we\'d expect to happen.', why: 'That\'s situational irony.' }] },
    vocab: [{ term: 'Dramatic irony', def: 'The audience knows something a character does not.' }, { term: 'Situational irony', def: 'The outcome is the opposite of what is expected.' }, { term: 'Verbal irony', def: 'Saying the opposite of what is meant.' }, { term: 'Indirect characterization', def: 'Revealing character through speech, thoughts, effect on others, actions, looks.' }, { term: 'Tone', def: 'The author\'s attitude toward the subject.' }, { term: 'Mood', def: 'The feeling the text creates in the reader.' }, { term: 'Symbol', def: 'Something that represents a bigger idea.' }],
    questions: [
      ['In Romeo and Juliet, the audience knows Juliet is only asleep, but Romeo doesn\'t. This is…', 'Dramatic irony', ['Verbal irony', 'Situational irony', 'Foreshadowing'], 'The audience knows more than the character.'],
      ['"What lovely weather," she said as the hail pounded the car.', 'Verbal irony', ['Dramatic irony', 'Symbolism', 'Mood'], 'She says the opposite of what she means.'],
      ['"His hands shook as he reached for the letter." This shows character through…', 'Actions (indirect)', ['Direct statement', 'Setting', 'Theme'], 'The narrator doesn\'t tell us he is nervous — we infer it.'],
      ['Good commentary in an analysis paragraph…', 'Explains how the evidence supports the claim', ['Repeats the quote', 'Summarizes the plot', 'Introduces a new topic'], 'It connects evidence to the claim.', 1],
      ['A dark, stormy opening scene mostly creates…', 'Mood', ['Theme', 'Characterization', 'Point of view'], 'It is about how the reader feels.'],
    ],
  }),
  bankTopic({
    id: 'adv-grammar', title: 'Advanced Grammar & Style',
    lesson: `**Clauses**: independent (stands alone) vs. dependent (starts with although, because, when, which…).
**Semicolons** join two independent clauses: "I studied; I passed." Use a comma before a conjunction instead: "I studied, so I passed."
**Colons** introduce a list or explanation after a complete sentence.
**Parallel structure**: items in a list use the same form ("hiking, swimming, and biking" — not "to bike").
**Active voice** is usually stronger than passive. Avoid **misplaced modifiers** ("Running to class, my backpack broke" — the backpack wasn't running!).`,
    example: { problem: 'Fix: "She likes reading, to swim, and biking."', steps: [{ step: 'Items: reading, to swim, biking.', why: 'Check the form of each item.' }, { step: '"She likes reading, swimming, and biking."', why: 'Make all items -ing forms (parallel).' }] },
    vocab: [{ term: 'Independent clause', def: 'A clause that can stand alone as a sentence.' }, { term: 'Dependent clause', def: 'A clause that cannot stand alone.' }, { term: 'Semicolon', def: 'Joins two closely related independent clauses.' }, { term: 'Parallel structure', def: 'Using the same grammatical form for items in a series.' }, { term: 'Misplaced modifier', def: 'A describing phrase placed next to the wrong word.' }],
    questions: [
      ['Which uses a semicolon correctly?', 'The storm hit; the power went out.', ['The storm; hit the town.', 'Because the storm hit; the power went out.', 'The storm hit; and the power went out.'], 'Both sides must be complete sentences, with no conjunction.'],
      ['Which has parallel structure?', 'I like to read, to draw, and to paint.', ['I like to read, drawing, and paint.', 'I like reading, to draw, and painting.', 'I like read, drawing, and to paint.'], 'All items should match.'],
      ['Which has a misplaced modifier?', 'Covered in mud, Mom washed the dog.', ['Covered in mud, the dog needed a bath.', 'Mom washed the muddy dog.', 'The dog, covered in mud, needed a bath.'], 'Who is covered in mud?'],
      ['"Although it rained" is a…', 'Dependent clause', ['Independent clause', 'Complete sentence', 'Prepositional phrase'], 'It cannot stand alone.'],
      ['Which correctly uses a colon?', 'Bring three things: a pen, a notebook, and water.', ['Bring: a pen and water.', 'I need: to go home.', 'The answer is: yes because.'], 'A complete sentence must come before the colon.', 1],
    ],
  }),
];

// ======================= WORLD HISTORY & ECONOMICS =======================
export const ADV_HIST: Topic[] = [
  bankTopic({
    id: 'adv-ancient', title: 'Ancient Civilizations',
    lesson: `Civilizations began in river valleys: **Mesopotamia** (Tigris & Euphrates — cuneiform, Code of Hammurabi), **Egypt** (Nile — pyramids, hieroglyphics), **Indus Valley**, and **China** (Yellow River — dynasties, Mandate of Heaven).
**Greece** gave us **democracy** (Athens), philosophy (Socrates, Plato, Aristotle), and the Olympics.
**Rome** became a **republic** (elected senators), then an empire; Roman law and engineering (roads, aqueducts) shaped the West. The Western Roman Empire fell in 476 CE.`,
    example: { problem: 'Why did early civilizations start near rivers?', steps: [{ step: 'Rivers gave water and fertile soil.', why: 'Farming surpluses let people settle.' }, { step: 'Surpluses allowed cities, trade and specialized jobs.', why: 'Those are the features of civilization.' }] },
    vocab: [{ term: 'Cuneiform', def: 'Wedge-shaped writing from Mesopotamia.' }, { term: 'Code of Hammurabi', def: 'One of the earliest written law codes (Babylon).' }, { term: 'Democracy', def: 'Rule by the people (developed in Athens).' }, { term: 'Republic', def: 'Government where citizens elect representatives (Rome).' }, { term: 'Mandate of Heaven', def: 'Chinese belief that rulers had divine approval that could be lost.' }],
    questions: [
      ['The Code of Hammurabi came from…', 'Mesopotamia', ['Egypt', 'Greece', 'China'], 'Babylon, between the Tigris and Euphrates.'],
      ['Which city-state is known for early democracy?', 'Athens', ['Sparta', 'Rome', 'Babylon'], 'Citizens voted directly on laws.'],
      ['Rome\'s government where citizens elected senators was a…', 'Republic', ['Monarchy', 'Theocracy', 'Direct democracy'], 'The U.S. borrowed this idea.'],
      ['The Mandate of Heaven explained…', 'Why Chinese dynasties rose and fell', ['How pyramids were built', 'Greek philosophy', 'Roman law'], 'Disasters meant a ruler lost heaven\'s approval.', 1],
    ],
  }),
  bankTopic({
    id: 'adv-modern', title: 'Revolutions & World Wars',
    lesson: `The **Enlightenment** (1700s) — thinkers like **Locke** (natural rights), **Montesquieu** (separation of powers), and **Rousseau** (social contract) — inspired the American and **French Revolutions** (1789).
The **Industrial Revolution** began in Britain (~1760): factories, steam power, urbanization.
**WWI** (1914–1918): causes = **MAIN** (Militarism, Alliances, Imperialism, Nationalism); spark = assassination of Archduke Franz Ferdinand.
**WWII** (1939–1945): Germany invades Poland; the Holocaust; U.S. enters after **Pearl Harbor** (1941); ends after atomic bombs on Japan.
The **Cold War** (1947–1991): U.S. vs. U.S.S.R., capitalism vs. communism.`,
    example: { problem: 'How did Locke influence the Declaration of Independence?', steps: [{ step: 'Locke: natural rights to life, liberty, property.', why: 'Key Enlightenment idea.' }, { step: 'Jefferson wrote "life, liberty, and the pursuit of happiness."', why: 'He adapted Locke\'s idea.' }] },
    vocab: [{ term: 'Enlightenment', def: '1700s movement emphasizing reason and individual rights.' }, { term: 'Industrial Revolution', def: 'Shift to machine and factory production, starting in Britain.' }, { term: 'Imperialism', def: 'A nation extending control over other lands.' }, { term: 'Holocaust', def: 'Nazi Germany\'s genocide of six million Jews and millions of others.' }, { term: 'Cold War', def: 'Tension between the U.S. and U.S.S.R. without direct large-scale war.' }],
    questions: [
      ['Which event sparked WWI?', 'Assassination of Archduke Franz Ferdinand', ['Invasion of Poland', 'Pearl Harbor', 'The French Revolution'], '1914, Sarajevo.'],
      ['The U.S. entered WWII after…', 'The attack on Pearl Harbor', ['The sinking of the Lusitania', 'D-Day', 'The fall of Berlin'], 'December 7, 1941.'],
      ['Who argued for separation of powers?', 'Montesquieu', ['Locke', 'Rousseau', 'Napoleon'], 'It became the three branches.'],
      ['The Industrial Revolution began in…', 'Great Britain', ['France', 'The United States', 'Japan'], 'Coal, iron, and textile mills.'],
      ['The Cold War mainly opposed…', 'The U.S. and the Soviet Union', ['Britain and France', 'Germany and Japan', 'China and India'], 'Capitalism vs. communism.', 1],
    ],
  }),
  bankTopic({
    id: 'adv-econ', title: 'Economics',
    lesson: `**Scarcity**: resources are limited, so we make choices. The **opportunity cost** is the next-best thing you give up.
**Supply and demand**: when demand rises (and supply stays the same), prices rise. When supply rises, prices fall.
**Market economy** (prices set by buyers & sellers), **command economy** (government decides), **mixed economy** (most countries, including the U.S.).
**Inflation** is a general rise in prices. **GDP** measures the total value of goods and services a country produces.`,
    example: { problem: 'You skip a $20 concert to study for a test. What is the opportunity cost?', steps: [{ step: 'What did you give up?', why: 'Opportunity cost = next-best alternative.' }, { step: 'The concert (and the fun of going).', why: 'Not the $20 — you kept that.' }] },
    vocab: [{ term: 'Scarcity', def: 'Limited resources compared to wants.' }, { term: 'Opportunity cost', def: 'The value of the next-best alternative given up.' }, { term: 'Supply', def: 'How much producers are willing to sell at each price.' }, { term: 'Demand', def: 'How much consumers want to buy at each price.' }, { term: 'Inflation', def: 'A general increase in prices over time.' }, { term: 'GDP', def: 'Total value of goods and services produced in a country.' }],
    questions: [
      ['A hot new video game is sold out everywhere. Its price will likely…', 'Rise', ['Fall', 'Stay exactly the same', 'Drop to zero'], 'High demand, low supply.'],
      ['The U.S. economy is best described as…', 'Mixed', ['Command', 'Traditional', 'Pure market'], 'Mostly markets, with some government role.'],
      ['A farmer grows corn instead of wheat. The wheat is the…', 'Opportunity cost', ['Profit', 'GDP', 'Supply'], 'The next-best option given up.'],
      ['If a factory makes far more toys than people want, toy prices will likely…', 'Fall', ['Rise', 'Stay the same', 'Double'], 'Surplus pushes prices down.', 1],
    ],
  }),
];

// ======================= LANGUAGE LEVEL 2 =======================
export function advLanguage(lang: 'spanish' | 'french'): Topic[] {
  const S = lang === 'spanish';
  return [
    bankTopic({
      id: `adv-${lang}-past`, title: S ? 'Past Tense: Preterite vs. Imperfect' : 'Past Tense: Passé Composé vs. Imparfait',
      lesson: S ? `**Preterite** = completed actions at a specific time: **hablé, comí, viví** (yo). "Ayer **comí** pizza."
**Imperfect** = ongoing, habitual, or background past: **hablaba, comía, vivía**. "Cuando era niño, **comía** pizza."
Clues: *ayer, anoche, una vez* → preterite. *siempre, todos los días, mientras* → imperfect.
Irregular preterite: ir/ser → **fui**, tener → **tuve**, hacer → **hice**.`
        : `**Passé composé** = completed actions: avoir/être + past participle. "J'**ai mangé**." "Je **suis allé(e)**."
**Imparfait** = ongoing, habitual, or background past: "Je **mangeais** tous les jours."
Clues: *hier, une fois, soudain* → passé composé. *toujours, chaque jour, pendant que* → imparfait.
**DR MRS VANDERTRAMP** verbs (aller, venir, partir…) use **être**.`,
      example: { problem: S ? 'Ayer yo ___ (ir) al cine.' : 'Hier, je ___ (aller) au cinéma.', steps: [{ step: S ? '"Ayer" = specific completed time → preterite.' : '"Hier" = completed action → passé composé.', why: 'Time clues tell you the tense.' }, { step: S ? 'ir → fui: "Ayer yo fui al cine."' : 'aller uses être: "Hier, je suis allé(e) au cinéma."', why: S ? 'Ir is irregular.' : 'Aller is a DR MRS VANDERTRAMP verb.' }] },
      vocab: S ? [{ term: 'ayer', def: 'yesterday' }, { term: 'anoche', def: 'last night' }, { term: 'siempre', def: 'always' }, { term: 'mientras', def: 'while' }, { term: 'fui', def: 'I went / I was (preterite)' }, { term: 'tuve', def: 'I had (preterite)' }]
        : [{ term: 'hier', def: 'yesterday' }, { term: 'soudain', def: 'suddenly' }, { term: 'toujours', def: 'always' }, { term: 'pendant que', def: 'while' }, { term: 'je suis allé(e)', def: 'I went' }, { term: "j'avais", def: 'I had / used to have' }],
      questions: S ? [
        ['Ayer yo ___ (comer) tacos.', 'comí', ['comía', 'como', 'comeré'], '"Ayer" → preterite.'],
        ['Cuando era niña, siempre ___ (jugar) en el parque.', 'jugaba', ['jugué', 'juego', 'jugará'], '"Siempre" → habitual → imperfect.'],
        ['Anoche nosotros ___ (ir) a la fiesta.', 'fuimos', ['íbamos', 'vamos', 'fueron'], 'Ir is irregular in the preterite.'],
        ['Mientras yo ___ (leer), sonó el teléfono.', 'leía', ['leí', 'leo', 'leeré'], 'Background action → imperfect.', 1],
        ['Ella ___ (tener) un accidente el lunes.', 'tuvo', ['tenía', 'tiene', 'tuve'], 'Specific event → preterite; "ella" form.', 1],
      ] : [
        ['Hier, j\'___ (manger) une pizza.', 'ai mangé', ['mangeais', 'mange', 'mangerai'], '"Hier" → passé composé.'],
        ['Quand j\'étais petit, je ___ (jouer) au parc chaque jour.', 'jouais', ['ai joué', 'joue', 'jouerai'], '"Chaque jour" → habit → imparfait.'],
        ['Elle ___ (aller) à Paris l\'année dernière.', 'est allée', ['a allé', 'allait', 'va'], 'Aller uses être, and agrees with "elle".'],
        ['Pendant que je ___ (lire), le téléphone a sonné.', 'lisais', ['ai lu', 'lis', 'lirai'], 'Background action → imparfait.', 1],
        ['Nous ___ (finir) nos devoirs à 8 heures.', 'avons fini', ['finissions', 'sommes finis', 'finissons'], 'Completed action with avoir.', 1],
      ],
    }),
    bankTopic({
      id: `adv-${lang}-future`, title: S ? 'Future & Conditional' : 'Futur & Conditionnel',
      lesson: S ? `**Future**: add endings to the infinitive: -é, -ás, -á, -emos, -éis, -án. "Mañana **hablaré**."
**Conditional** ("would"): add -ía, -ías, -ía, -íamos, -íais, -ían. "Yo **hablaría**."
Irregular stems: tener → **tendr-**, hacer → **har-**, poder → **podr-**, salir → **saldr-**, decir → **dir-**.
Near future: **ir a + infinitive**: "Voy a estudiar."`
        : `**Futur simple**: infinitive + -ai, -as, -a, -ons, -ez, -ont. "Demain, je **parlerai**."
**Conditionnel** ("would"): future stem + -ais, -ais, -ait, -ions, -iez, -aient. "Je **parlerais**."
Irregular stems: être → **ser-**, avoir → **aur-**, aller → **ir-**, faire → **fer-**, pouvoir → **pourr-**.
Near future: **aller + infinitive**: "Je vais étudier."`,
      example: { problem: S ? 'Translate: "I would travel to Spain."' : 'Translate: "I would travel to France."', steps: [{ step: '"Would" → conditional.', why: 'Conditional expresses hypotheticals.' }, { step: S ? 'viajar + ía → "Yo viajaría a España."' : 'voyager + ais → "Je voyagerais en France."', why: 'Regular verbs keep the full infinitive.' }] },
      vocab: S ? [{ term: 'mañana', def: 'tomorrow' }, { term: 'tendré', def: 'I will have' }, { term: 'haría', def: 'I would do/make' }, { term: 'podría', def: 'I could / would be able to' }, { term: 'voy a', def: 'I am going to' }]
        : [{ term: 'demain', def: 'tomorrow' }, { term: "j'aurai", def: 'I will have' }, { term: 'je ferais', def: 'I would do/make' }, { term: 'je pourrais', def: 'I could' }, { term: 'je vais', def: 'I am going to' }],
      questions: S ? [
        ['Mañana yo ___ (hablar) con el profesor.', 'hablaré', ['hablaría', 'hablé', 'hablaba'], 'Future: infinitive + é.'],
        ['Si tuviera dinero, ___ (comprar) un carro.', 'compraría', ['compraré', 'compré', 'compro'], '"Would" → conditional.'],
        ['Nosotros ___ (tener) un examen el viernes.', 'tendremos', ['teneremos', 'tendríamos', 'tenemos'], 'Tener has an irregular future stem.', 1],
        ['"I am going to study" =', 'Voy a estudiar', ['Estudiaré a ir', 'Iba estudiar', 'Voy estudiar'], 'ir a + infinitive.'],
      ] : [
        ['Demain, je ___ (parler) avec le professeur.', 'parlerai', ['parlerais', 'ai parlé', 'parlais'], 'Futur: infinitive + ai.'],
        ['Si j\'avais de l\'argent, j\'___ (acheter) une voiture.', 'achèterais', ['achèterai', 'ai acheté', 'achète'], '"Would" → conditionnel.'],
        ['Nous ___ (avoir) un examen vendredi.', 'aurons', ['avrons', 'aurions', 'avons'], 'Avoir has the irregular stem aur-.', 1],
        ['"I am going to study" =', 'Je vais étudier', ['J\'étudierai aller', 'J\'allais étudier', 'Je vais étudie'], 'aller + infinitive.'],
      ],
    }),
  ];
}
