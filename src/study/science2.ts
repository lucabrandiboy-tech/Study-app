import type { Question } from './types';
import { bankTopic } from './bank';
import { ri, round, pick } from './rand';

const n = (value: number, prompt: string, hints: string[], explanation: string, mistakes: { value: number; msg: string }[] = []): Question => ({ prompt, answer: { kind: 'number', value, mistakes }, hints, explanation });

export const SCIENCE2 = [
  bankTopic({
    id: 'sci-method', title: 'Scientific Method & Lab Skills',
    lesson: `Scientists answer questions with the **scientific method**: ask a question → research → form a **hypothesis** (a testable prediction) → run an **experiment** → analyze data → draw a conclusion → share results.
The **independent variable** is what YOU change. The **dependent variable** is what you measure. **Controlled variables** stay the same so the test is fair. A **control group** gets no treatment, for comparison.
Repeat trials and take the **mean** (average) to make results more reliable. Scientists use **metric units**: meters, grams, liters, °C.`,
    example: { problem: 'You test whether fertilizer amount affects plant height. Name the variables.', steps: [{ step: 'Independent: amount of fertilizer', why: 'That is what you change on purpose.' }, { step: 'Dependent: plant height', why: 'That is what you measure.' }, { step: 'Controlled: water, light, soil, pot size, plant type', why: 'Keeping these the same makes it a fair test.' }] },
    vocab: [{ term: 'Hypothesis', def: 'A testable prediction about how variables are related.' }, { term: 'Independent variable', def: 'The factor the scientist changes on purpose.' }, { term: 'Dependent variable', def: 'The factor that is measured in an experiment.' }, { term: 'Controlled variable', def: 'A factor kept the same during an experiment.' }, { term: 'Control group', def: 'The group that does not receive the treatment, used for comparison.' }, { term: 'Mean', def: 'The average: add all values and divide by how many there are.' }],
    questions: [
      ['Which is a testable hypothesis?', 'If plants get more light, then they will grow taller.', ['Plants are pretty.', 'Plants should be watered.', 'I like tall plants.'], 'A hypothesis predicts a relationship you can measure.'],
      ['In a test of how temperature affects dissolving time, temperature is the…', 'Independent variable', ['Dependent variable', 'Controlled variable', 'Conclusion'], 'It is what the experimenter changes.'],
      ['Why do scientists repeat trials?', 'To make results more reliable', ['To use more supplies', 'To change the hypothesis', 'To avoid measuring'], 'One trial could be a fluke.'],
      ['The metric unit for mass is the…', 'Gram', ['Liter', 'Meter', 'Degree'], 'Liters measure volume, meters measure length.'],
      ['Data that does NOT support your hypothesis means…', 'You revise the hypothesis and test again', ['The experiment was useless', 'You should hide the data', 'Science failed'], 'Unexpected results are still valuable.', 1],
    ],
    gens: [() => { const d = Array.from({ length: 4 }, () => ri(8, 30)); const m = d.reduce((a, b) => a + b, 0) / 4; return n(m, `Four trials measured ${d.join(' cm, ')} cm. What is the mean?`, ['Add the four values.', 'Divide the total by 4.'], `(${d.join(' + ')}) ÷ 4 = ${m} cm.`); }],
  }),
  bankTopic({
    id: 'sci-cells', title: 'Cells & Life',
    lesson: `All living things are made of **cells** — the basic unit of life (**cell theory**). Cells come only from other cells.
**Unicellular** organisms are one cell (bacteria); **multicellular** organisms have many (you have trillions).
Levels of organization: **cell → tissue → organ → organ system → organism**.
Plant cells have a **cell wall** and **chloroplasts**; animal cells don't. Both have a **nucleus**, **cytoplasm**, **cell membrane**, and **mitochondria**.
Cells divide by **mitosis** to grow and repair.`,
    example: { problem: 'Put in order from smallest to largest: organ, cell, organism, tissue, organ system.', steps: [{ step: 'Cell is the smallest living unit.', why: 'Everything is built from cells.' }, { step: 'cell → tissue → organ → organ system → organism', why: 'Similar cells form tissues, tissues form organs, organs work together in systems.' }] },
    vocab: [{ term: 'Cell theory', def: 'All living things are made of cells, cells are the basic unit of life, and cells come from other cells.' }, { term: 'Tissue', def: 'A group of similar cells working together.' }, { term: 'Organ', def: 'A structure made of tissues that does a specific job.' }, { term: 'Cell wall', def: 'A rigid layer outside the membrane of plant cells.' }, { term: 'Cytoplasm', def: 'The jelly-like fluid inside a cell.' }, { term: 'Mitosis', def: 'Cell division that makes two identical cells.' }],
    questions: [
      ['Which structure do plant cells have that animal cells do not?', 'Cell wall', ['Nucleus', 'Cell membrane', 'Mitochondria'], 'It gives plants their stiffness.'],
      ['The heart is an example of a(n)…', 'Organ', ['Tissue', 'Cell', 'Organ system'], 'It is made of several tissues working together.'],
      ['Bacteria are…', 'Unicellular', ['Multicellular', 'Not living', 'Made of tissues'], 'A bacterium is one cell.'],
      ['What controls what enters and leaves a cell?', 'Cell membrane', ['Cell wall', 'Nucleus', 'Chloroplast'], 'It is selectively permeable.'],
      ['According to cell theory, new cells come from…', 'Existing cells', ['Nonliving matter', 'Sunlight', 'Water'], 'Life comes from life.'],
    ],
  }),
  bankTopic({
    id: 'sci-body', title: 'Human Body Systems',
    lesson: `Body systems work together to keep you alive (**homeostasis** = keeping conditions stable).
• **Circulatory**: heart and blood vessels carry oxygen and nutrients. • **Respiratory**: lungs take in O₂ and release CO₂.
• **Digestive**: breaks food into nutrients. • **Nervous**: brain, spinal cord, nerves send signals.
• **Skeletal**: support and protection; **Muscular**: movement. • **Immune**: fights germs. • **Endocrine**: hormones.
Example teamwork: lungs load oxygen into blood, the heart pumps it to muscles.`,
    example: { problem: 'When you run, why do you breathe faster AND your heart beats faster?', steps: [{ step: 'Muscles need more oxygen for energy.', why: 'Cellular respiration uses O₂.' }, { step: 'Respiratory system brings in more O₂; circulatory system delivers it faster.', why: 'Systems work together to keep homeostasis.' }] },
    vocab: [{ term: 'Homeostasis', def: 'Keeping the body\'s internal conditions stable.' }, { term: 'Circulatory system', def: 'Heart and blood vessels that move blood through the body.' }, { term: 'Respiratory system', def: 'Organs that exchange oxygen and carbon dioxide.' }, { term: 'Nervous system', def: 'Brain, spinal cord and nerves that carry signals.' }, { term: 'Endocrine system', def: 'Glands that release hormones.' }, { term: 'Immune system', def: 'Defends the body against disease.' }],
    questions: [
      ['Which system breaks food into nutrients?', 'Digestive', ['Respiratory', 'Skeletal', 'Endocrine'], 'It starts in the mouth.'],
      ['Gas exchange happens in the…', 'Lungs', ['Stomach', 'Heart', 'Kidneys'], 'Tiny air sacs called alveoli.'],
      ['Sweating to cool down is an example of…', 'Homeostasis', ['Digestion', 'Mitosis', 'Photosynthesis'], 'Keeping body temperature stable.'],
      ['Which system sends electrical signals?', 'Nervous', ['Muscular', 'Circulatory', 'Digestive'], 'Neurons carry messages.'],
      ['White blood cells are part of the…', 'Immune system', ['Skeletal system', 'Digestive system', 'Respiratory system'], 'They fight germs.'],
    ],
  }),
  bankTopic({
    id: 'sci-weather', title: 'Weather & Climate',
    lesson: `**Weather** is the day-to-day condition of the atmosphere; **climate** is the average weather over many years.
The Sun heats Earth unevenly, causing **air pressure** differences and **wind** (air moves from high to low pressure).
**Air masses** meet at **fronts**: cold fronts bring quick storms; warm fronts bring steady rain.
The **greenhouse effect** traps heat; extra CO₂ from burning fossil fuels strengthens it (**climate change**).
Temperature: °C = (°F − 32) × 5⁄9.`,
    example: { problem: 'Convert 68°F to Celsius.', steps: [{ step: '68 − 32 = 36', why: 'Subtract 32 first.' }, { step: '36 × 5/9 = 20°C', why: 'Then multiply by 5/9.' }] },
    vocab: [{ term: 'Weather', def: 'Short-term condition of the atmosphere.' }, { term: 'Climate', def: 'Average weather of a place over many years.' }, { term: 'Air pressure', def: 'The weight of air pushing on a surface.' }, { term: 'Front', def: 'The boundary where two air masses meet.' }, { term: 'Humidity', def: 'The amount of water vapor in the air.' }, { term: 'Greenhouse effect', def: 'Gases in the atmosphere trapping heat near Earth\'s surface.' }],
    questions: [
      ['"It is raining in Chicago today" describes…', 'Weather', ['Climate', 'Latitude', 'Season'], 'Short-term.'],
      ['Wind blows from areas of…', 'High pressure to low pressure', ['Low pressure to high pressure', 'Cold to colder', 'Ocean only'], 'Air flows "downhill" in pressure.'],
      ['A cold front usually brings…', 'Sudden storms, then cooler weather', ['Weeks of fog', 'Hotter weather', 'No change'], 'Dense cold air shoves warm air up fast.'],
      ['Which gas is most linked to human-caused climate change?', 'Carbon dioxide', ['Oxygen', 'Nitrogen', 'Helium'], 'Released by burning fossil fuels.'],
      ['What causes the seasons?', "The tilt of Earth's axis", ["Earth's distance from the Sun", 'Clouds', 'The Moon'], 'Tilt changes how directly sunlight hits.', 1],
    ],
    gens: [() => { const c = ri(-10, 40); const f = round(c * 9 / 5 + 32, 1); return n(c, `Convert ${f}°F to Celsius.`, ['Subtract 32.', 'Multiply by 5/9.'], `(${f} − 32) × 5/9 = ${c}°C.`, [{ value: round((f - 32) * 9 / 5, 1), msg: 'Use 5/9, not 9/5, when going from °F to °C.' }]); }],
  }),
  bankTopic({
    id: 'sci-space', title: 'Earth, Moon & Space',
    lesson: `Earth **rotates** on its axis once a day (day/night) and **revolves** around the Sun once a year.
The Moon's **phases** come from how much of its sunlit side we see. **Eclipses**: solar (Moon blocks Sun), lunar (Earth's shadow on Moon).
**Tides** are caused mainly by the Moon's gravity.
Our **solar system**: the Sun, 8 planets (Mercury, Venus, Earth, Mars — rocky; Jupiter, Saturn, Uranus, Neptune — gas/ice giants), moons, asteroids, comets.
**Gravity** depends on mass and distance. Your **weight** changes on other worlds; your **mass** does not.`,
    example: { problem: 'A 50 kg student: what is her mass on the Moon?', steps: [{ step: 'Still 50 kg.', why: 'Mass is the amount of matter; it does not change with location.' }, { step: 'Her weight would be about 1/6 of Earth weight.', why: 'The Moon\'s gravity is weaker.' }] },
    vocab: [{ term: 'Rotation', def: 'Spinning on an axis; causes day and night.' }, { term: 'Revolution', def: 'One orbit around another object; Earth\'s takes a year.' }, { term: 'Lunar eclipse', def: 'Earth\'s shadow falls on the Moon.' }, { term: 'Solar eclipse', def: 'The Moon blocks the Sun from view.' }, { term: 'Gravity', def: 'The attraction between masses.' }, { term: 'Light-year', def: 'The distance light travels in one year.' }],
    questions: [
      ['What causes day and night?', "Earth's rotation", ["Earth's revolution", 'The Moon', 'Seasons'], 'One spin per day.'],
      ['Which planet is largest?', 'Jupiter', ['Saturn', 'Earth', 'Neptune'], 'A gas giant more than 11× Earth\'s width.'],
      ['Tides are caused mostly by…', "The Moon's gravity", ['Wind', "Earth's core", 'Clouds'], 'The Moon pulls the oceans.'],
      ['A light-year measures…', 'Distance', ['Time', 'Brightness', 'Mass'], 'How far light goes in a year.', 1],
      ['Which is a rocky (inner) planet?', 'Mars', ['Jupiter', 'Uranus', 'Saturn'], 'The inner four are rocky.'],
    ],
    gens: [() => { const m = ri(30, 90); const w = round(m * 1.62, 1); return n(w, `A ${m} kg astronaut on the Moon (g = 1.62 m/s²). What is her weight in newtons?`, ['Weight = mass × gravity.', `${m} × 1.62`], `${m} × 1.62 ≈ ${w} N.`, [{ value: m, msg: 'That is her mass in kg. Weight is a force: multiply by g.' }]); }],
  }),
  bankTopic({
    id: 'sci-electricity', title: 'Electricity & Magnetism',
    lesson: `**Electric current** is the flow of electrons, measured in amperes (A). **Voltage** (V) pushes current; **resistance** (Ω) opposes it. **Ohm's law**: V = I × R.
A **series circuit** has one path (one bulb out → all out). A **parallel circuit** has several paths (others stay on).
**Conductors** (metals) let current flow; **insulators** (rubber, plastic) don't.
Magnets have **north and south poles**; opposites attract. Moving electricity makes magnetism (**electromagnet**), and moving magnets make electricity (**generator**).`,
    example: { problem: 'A 9 V battery powers a 3 Ω bulb. What is the current?', steps: [{ step: 'I = V ÷ R', why: "Rearrange Ohm's law." }, { step: 'I = 9 ÷ 3 = 3 A', why: 'Divide.' }] },
    vocab: [{ term: 'Current', def: 'The flow of electric charge, in amperes.' }, { term: 'Voltage', def: 'The electrical push that moves charges, in volts.' }, { term: 'Resistance', def: 'Opposition to current, in ohms.' }, { term: 'Series circuit', def: 'A circuit with only one path for current.' }, { term: 'Parallel circuit', def: 'A circuit with more than one path for current.' }, { term: 'Electromagnet', def: 'A magnet made by running current through a coil of wire.' }],
    questions: [
      ['Houses are wired in parallel because…', 'Each device works even if another is off', ['It uses less wire', 'It needs no switch', 'It has one path'], 'Several paths for current.'],
      ['Which is a good insulator?', 'Rubber', ['Copper', 'Aluminum', 'Salt water'], 'Insulators block current.'],
      ['Two north poles placed together will…', 'Repel', ['Attract', 'Stick', 'Cancel gravity'], 'Like poles repel.'],
      ['A generator turns motion into…', 'Electricity', ['Magnetism only', 'Heat only', 'Light only'], 'Moving magnets near coils make current.', 1],
    ],
    gens: [() => { const I = ri(1, 8), R = ri(2, 20); const t = pick(['V', 'I']); return t === 'V' ? n(I * R, `Current ${I} A flows through ${R} Ω. Find the voltage.`, ['V = I × R.'], `${I} × ${R} = ${I * R} V.`) : n(I, `A ${I * R} V source is connected to ${R} Ω. Find the current in amps.`, ['I = V ÷ R.'], `${I * R} ÷ ${R} = ${I} A.`, [{ value: I * R * R, msg: 'Divide voltage by resistance.' }]); }],
  }),
  bankTopic({
    id: 'sci-periodic', title: 'The Periodic Table',
    lesson: `The **periodic table** arranges elements by **atomic number** (number of protons).
**Rows** are **periods**; **columns** are **groups** (families) with similar properties because they have the same number of **valence electrons**.
**Metals** (left) are shiny, conductive, and malleable; **nonmetals** (right) are dull and brittle; **metalloids** lie along the staircase.
Key groups: **alkali metals** (group 1, very reactive), **halogens** (group 17), **noble gases** (group 18, unreactive).
Each box shows the symbol (e.g. Na for sodium), atomic number and atomic mass.`,
    example: { problem: 'Sodium (Na) has atomic number 11 and mass ~23. How many neutrons?', steps: [{ step: 'Protons = 11', why: 'Atomic number.' }, { step: 'Neutrons ≈ 23 − 11 = 12', why: 'Mass number = protons + neutrons.' }] },
    vocab: [{ term: 'Period', def: 'A horizontal row of the periodic table.' }, { term: 'Group', def: 'A vertical column of elements with similar properties.' }, { term: 'Alkali metals', def: 'Group 1 metals; very reactive with water.' }, { term: 'Noble gases', def: 'Group 18; stable and unreactive.' }, { term: 'Metalloid', def: 'An element with properties of metals and nonmetals.' }, { term: 'Chemical symbol', def: 'One or two letters that stand for an element.' }],
    questions: [
      ['The symbol Fe stands for…', 'Iron', ['Fluorine', 'Fermium', 'Francium'], 'From Latin "ferrum".'],
      ['Elements in the same column have similar…', 'Chemical properties', ['Masses', 'Colors', 'Names'], 'Same valence electrons.'],
      ['Which element is a noble gas?', 'Neon', ['Oxygen', 'Sodium', 'Iron'], 'Group 18.'],
      ['Metals are usually…', 'Good conductors', ['Brittle', 'Dull', 'Insulators'], 'Electrons move freely.'],
      ['The symbol Au stands for…', 'Gold', ['Silver', 'Aluminum', 'Argon'], 'From Latin "aurum".', 1],
    ],
  }),
  bankTopic({
    id: 'sci-resources', title: "Water Cycle & Earth's Resources",
    lesson: `The **water cycle**: **evaporation** (liquid → vapor), **condensation** (vapor → clouds), **precipitation** (rain, snow), **collection/runoff** into oceans, lakes, and **groundwater**. The Sun powers it.
**Renewable resources** replace themselves (solar, wind, water, trees if managed). **Nonrenewable** ones take millions of years (coal, oil, natural gas).
Protect resources: reduce, reuse, recycle; conserve water; prevent pollution.`,
    example: { problem: 'A puddle disappears on a sunny day. Where did the water go?', steps: [{ step: 'It evaporated into water vapor.', why: 'The Sun\'s heat turns liquid into gas.' }, { step: 'Later it may condense into clouds and fall as rain.', why: 'That is the water cycle.' }] },
    vocab: [{ term: 'Evaporation', def: 'Liquid water changing into water vapor.' }, { term: 'Condensation', def: 'Water vapor cooling into liquid droplets.' }, { term: 'Precipitation', def: 'Water falling from clouds as rain, snow, sleet, or hail.' }, { term: 'Groundwater', def: 'Water stored underground in soil and rock.' }, { term: 'Renewable resource', def: 'A resource that is naturally replaced quickly.' }, { term: 'Fossil fuel', def: 'Coal, oil or gas formed from ancient organisms; nonrenewable.' }],
    questions: [
      ['Clouds form by…', 'Condensation', ['Evaporation', 'Runoff', 'Transpiration'], 'Vapor cools into droplets.'],
      ['What powers the water cycle?', 'The Sun', ['The Moon', 'Wind only', "Earth's core"], 'It provides the heat for evaporation.'],
      ['Which is renewable?', 'Wind energy', ['Coal', 'Oil', 'Natural gas'], 'Wind keeps blowing.'],
      ['Water soaking into the ground becomes…', 'Groundwater', ['Condensation', 'Precipitation', 'Humidity'], 'Stored in aquifers.'],
      ['Plants releasing water vapor from leaves is called…', 'Transpiration', ['Condensation', 'Erosion', 'Runoff'], 'Part of the water cycle.', 1],
    ],
  }),
];
