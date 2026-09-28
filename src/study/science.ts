import type { Difficulty } from '../lib/store';
import type { Question } from './types';
import { bankTopic } from './bank';
import { ri, round, fmt } from './rand';

const n = (value: number, prompt: string, hints: string[], explanation: string, mistakes: { value: number; msg: string }[] = []): Question => ({ prompt, answer: { kind: 'number', value, mistakes }, hints, explanation });

const density = (d: Difficulty): Question => {
  const V = ri(2, 20), D = d >= 2 ? round(ri(5, 130) / 10, 1) : ri(1, 9), m = round(D * V, 1);
  return d >= 2 && Math.random() < 0.5
    ? n(round(m / D, 1), `An object has a mass of ${m} g and a density of ${D} g/cm³. What is its volume in cm³?`, ['Density = mass ÷ volume. Rearrange to solve for volume.', 'Volume = mass ÷ density.'], `V = ${m} ÷ ${D} = ${round(m / D, 1)} cm³.`)
    : n(D, `An object has a mass of ${m} g and a volume of ${V} cm³. What is its density in g/cm³?`, ['Density tells how much mass is packed into each cm³.', 'D = m ÷ V.'], `D = ${m} ÷ ${V} = ${D} g/cm³.`, [{ value: round(V / m, 2), msg: 'You divided volume by mass. Density = mass ÷ volume.' }]);
};
const speed = (d: Difficulty): Question => {
  const t = ri(2, 12), v = ri(3, 30), dist = v * t;
  return d >= 1 && Math.random() < 0.5
    ? n(dist, `A cyclist rides at ${v} m/s for ${t} s. How far do they go (in m)?`, ['Speed = distance ÷ time, so distance = speed × time.'], `${v} × ${t} = ${dist} m.`)
    : n(v, `A runner goes ${dist} m in ${t} s. What is the average speed in m/s?`, ['Speed = distance ÷ time.'], `${dist} ÷ ${t} = ${v} m/s.`, [{ value: round(t / dist, 3), msg: 'Flip it: speed = distance ÷ time.' }]);
};
const force = (d: Difficulty): Question => {
  const m = ri(2, 50), a = ri(1, 12), F = m * a;
  return d >= 1 && Math.random() < 0.5
    ? n(a, `A ${F} N net force acts on a ${m} kg object. What is its acceleration (m/s²)?`, ["Newton's 2nd law: F = m × a.", 'Rearrange: a = F ÷ m.'], `a = ${F} ÷ ${m} = ${a} m/s².`)
    : n(F, `What net force is needed to accelerate a ${m} kg cart at ${a} m/s²? (in N)`, ["Newton's 2nd law: F = m × a."], `F = ${m} × ${a} = ${F} N.`);
};
const energy = (d: Difficulty): Question => {
  const m = ri(1, 20), v = ri(1, 10), h = ri(1, 20);
  return d >= 1 && Math.random() < 0.5
    ? n(m * 9.8 * h, `Find the gravitational potential energy of a ${m} kg object ${h} m high. Use g = 9.8 m/s². (J)`, ['PE = m × g × h.'], `PE = ${m} × 9.8 × ${h} = ${fmt(m * 9.8 * h)} J.`)
    : n(0.5 * m * v * v, `Find the kinetic energy of a ${m} kg ball moving at ${v} m/s. (J)`, ['KE = ½ m v².', 'Square the speed first, then multiply by the mass and by ½.'], `KE = ½ × ${m} × ${v * v} = ${fmt(0.5 * m * v * v)} J.`, [{ value: 0.5 * m * v, msg: 'Remember to SQUARE the velocity.' }]);
};
const wave = (): Question => {
  const f = ri(2, 20), l = ri(1, 10);
  return n(f * l, `A wave has a frequency of ${f} Hz and a wavelength of ${l} m. What is its speed (m/s)?`, ['Wave speed = frequency × wavelength (v = fλ).'], `v = ${f} × ${l} = ${f * l} m/s.`);
};
const punnett = (): Question => {
  const cross = [['Bb × Bb', 75, 'BB, Bb, Bb, bb → 3 of 4 show the dominant trait.'], ['Bb × bb', 50, 'Bb, Bb, bb, bb → 2 of 4.'], ['BB × bb', 100, 'All offspring are Bb.'], ['bb × bb', 0, 'All offspring are bb.']] as const;
  const [c, p, why] = cross[ri(0, 3)];
  return n(p, `In a cross ${c}, what percent of offspring are expected to show the dominant trait (B)?`, ['Draw a Punnett square: put one parent\'s alleles on top and the other\'s on the side.', 'Any box with at least one capital B shows the dominant trait.'], `${why} = ${p}%.`);
};

export const SCIENCE = [
  bankTopic({
    id: 'sci-matter', title: 'Matter & Atoms',
    lesson: `**Matter** is anything that has mass and takes up space. It is made of **atoms**.
An atom has a **nucleus** with **protons** (+) and **neutrons** (no charge), surrounded by **electrons** (−).
The **atomic number** = number of protons; it identifies the element. **Mass number** = protons + neutrons.
**Elements** have one kind of atom. **Compounds** are two or more elements chemically bonded (like H₂O). **Mixtures** are physically combined.
States of matter: **solid** (fixed shape & volume), **liquid** (fixed volume), **gas** (neither). **Density** = mass ÷ volume.`,
    example: { problem: 'Carbon has atomic number 6 and mass number 12. How many neutrons does it have?', steps: [{ step: 'Protons = atomic number = 6', why: 'The atomic number counts protons.' }, { step: 'Neutrons = mass number − protons = 12 − 6 = 6', why: 'Mass number counts protons + neutrons together.' }] },
    vocab: [
      { term: 'Atom', def: 'The smallest unit of an element.' }, { term: 'Proton', def: 'Positive particle in the nucleus.' }, { term: 'Neutron', def: 'Neutral particle in the nucleus.' },
      { term: 'Electron', def: 'Negative particle outside the nucleus.' }, { term: 'Element', def: 'A pure substance made of one type of atom.' }, { term: 'Compound', def: 'Two or more elements chemically combined.' },
      { term: 'Mixture', def: 'Substances physically combined, not chemically bonded.' }, { term: 'Density', def: 'Mass per unit volume.' },
    ],
    questions: [
      ['Which particle determines what element an atom is?', 'Proton', ['Neutron', 'Electron', 'Photon'], 'The atomic number (count of this particle) identifies the element.'],
      ['Salt water is an example of a…', 'Mixture', ['Element', 'Compound', 'Atom'], 'The salt and water are not chemically bonded; you can separate them by evaporation.'],
      ['In which state do particles move the fastest?', 'Gas', ['Solid', 'Liquid', 'They all move the same'], 'More energy means faster particles and more space between them.'],
      ['Oxygen has atomic number 8. How many electrons does a neutral oxygen atom have?', '8', ['16', '4', '0'], 'In a neutral atom, electrons equal protons.'],
      ['H₂O is a…', 'Compound', ['Element', 'Mixture', 'Isotope'], 'Hydrogen and oxygen are chemically bonded.'],
      ['Where is almost all of an atom\'s mass?', 'In the nucleus', ['In the electron cloud', 'Spread evenly', 'In the empty space'], 'Protons and neutrons are much heavier than electrons.', 1],
      ['Atoms of the same element with different numbers of neutrons are called…', 'Isotopes', ['Ions', 'Molecules', 'Compounds'], 'Same protons, different mass number.', 1],
      ['An object floats in water (density 1 g/cm³). Its density must be…', 'Less than 1 g/cm³', ['More than 1 g/cm³', 'Exactly 1 g/cm³', 'Zero'], 'Less dense things float on more dense things.', 1],
    ],
    gens: [density],
  }),
  bankTopic({
    id: 'sci-reactions', title: 'Chemical Reactions',
    lesson: `In a **chemical reaction**, **reactants** turn into new substances called **products**. Atoms are rearranged, not created or destroyed.
The **Law of Conservation of Mass**: total mass of reactants = total mass of products. That's why equations must be **balanced**.
Signs of a chemical change: gas bubbles, color change, temperature change, light, or a **precipitate** (solid forms).
A **physical change** (melting, cutting, dissolving) does not make a new substance.
**Exothermic** reactions release heat; **endothermic** reactions absorb heat.`,
    example: { problem: '10 g of baking soda reacts with 20 g of vinegar in a closed bag. What is the total mass of the products?', steps: [{ step: 'Reactant mass = 10 + 20 = 30 g', why: 'Add the masses of everything that reacts.' }, { step: 'Products = 30 g', why: 'Conservation of mass — in a closed system, no mass is lost or gained.' }] },
    vocab: [
      { term: 'Reactant', def: 'A starting substance in a chemical reaction.' }, { term: 'Product', def: 'A new substance formed by a reaction.' }, { term: 'Conservation of mass', def: 'Mass is neither created nor destroyed in a reaction.' },
      { term: 'Precipitate', def: 'A solid that forms from two liquids reacting.' }, { term: 'Exothermic', def: 'A reaction that releases energy (gets warmer).' }, { term: 'Endothermic', def: 'A reaction that absorbs energy (gets colder).' },
      { term: 'Coefficient', def: 'The number in front of a formula showing how many molecules.' },
    ],
    questions: [
      ['Which is a chemical change?', 'Iron rusting', ['Ice melting', 'Paper being cut', 'Sugar dissolving'], 'A chemical change makes a new substance.'],
      ['In 2H₂ + O₂ → 2H₂O, the reactants are…', 'H₂ and O₂', ['H₂O', 'Only O₂', 'H₂O and O₂'], 'Reactants are on the left side of the arrow.'],
      ['A cold pack gets cold when squeezed. This reaction is…', 'Endothermic', ['Exothermic', 'A physical change only', 'Nuclear'], 'It absorbs heat from its surroundings (your hand).'],
      ['How many oxygen atoms are in 2H₂O?', '2', ['1', '4', '3'], 'The coefficient 2 multiplies every atom in the formula.', 1],
      ['Why must chemical equations be balanced?', 'Atoms are not created or destroyed', ['To look neat', 'Because energy is lost', 'To make more product'], 'Conservation of mass.'],
      ['Which is NOT a sign of a chemical reaction?', 'Changing shape', ['Gas bubbles', 'Color change', 'A precipitate forms'], 'Shape changes are physical.'],
      ['Balance: _Na + Cl₂ → 2NaCl. What goes in the blank?', '2', ['1', '3', '4'], 'Count sodium atoms on the right side.', 1],
    ],
  }),
  bankTopic({
    id: 'sci-forces', title: 'Forces & Motion',
    lesson: `A **force** is a push or pull, measured in **newtons (N)**.
**Speed** = distance ÷ time. **Velocity** is speed with a direction. **Acceleration** is how fast velocity changes.
**Newton's 1st law** (inertia): an object stays at rest or keeps moving unless a net force acts on it.
**Newton's 2nd law**: F = m × a.
**Newton's 3rd law**: every action has an equal and opposite reaction.
**Balanced forces** → no change in motion. **Unbalanced forces** → acceleration.`,
    example: { problem: 'A 4 kg skateboard is pushed with a net force of 12 N. Find its acceleration.', steps: [{ step: 'F = m·a', why: "Newton's 2nd law links force, mass, and acceleration." }, { step: '12 = 4·a', why: 'Substitute.' }, { step: 'a = 3 m/s²', why: 'Divide both sides by 4.' }] },
    vocab: [
      { term: 'Force', def: 'A push or pull on an object.' }, { term: 'Inertia', def: 'The tendency of an object to resist changes in motion.' }, { term: 'Velocity', def: 'Speed in a given direction.' },
      { term: 'Acceleration', def: 'The rate at which velocity changes.' }, { term: 'Net force', def: 'The total of all forces acting on an object.' }, { term: 'Friction', def: 'A force that opposes motion between surfaces.' },
      { term: 'Gravity', def: 'The attractive force between masses.' },
    ],
    questions: [
      ['A passenger lurches forward when a car stops suddenly. Which law explains this?', "Newton's 1st law", ["Newton's 2nd law", "Newton's 3rd law", 'Conservation of mass'], 'The body keeps moving because of inertia.'],
      ['A rocket pushes gas down and moves up. Which law?', "Newton's 3rd law", ["Newton's 1st law", "Newton's 2nd law", 'Law of gravity'], 'Action and reaction pairs.'],
      ['Two teams pull a rope with equal force. The rope…', "Doesn't move", ['Moves left', 'Moves right', 'Speeds up'], 'Balanced forces → no change in motion.'],
      ['Which has more inertia?', 'A bowling ball', ['A tennis ball', 'A feather', 'A balloon'], 'Inertia depends on mass.'],
      ['Which force slows a sliding book?', 'Friction', ['Magnetism', 'Inertia', 'Buoyancy'], 'It acts between surfaces in contact.'],
    ],
    gens: [speed, force],
  }),
  bankTopic({
    id: 'sci-energy', title: 'Energy',
    lesson: `**Energy** is the ability to do work, measured in **joules (J)**.
**Kinetic energy** is energy of motion: KE = ½mv². **Potential energy** is stored energy; gravitational PE = mgh.
The **Law of Conservation of Energy**: energy can change form, but is never created or destroyed.
Forms: thermal, chemical, electrical, light, sound, nuclear, mechanical.
Heat moves by **conduction** (touching), **convection** (moving fluids), and **radiation** (waves).`,
    example: { problem: 'A 2 kg ball moves at 3 m/s. Find its kinetic energy.', steps: [{ step: 'KE = ½mv²', why: 'Kinetic energy formula.' }, { step: 'KE = ½(2)(3²) = ½(2)(9)', why: 'Square the velocity first.' }, { step: 'KE = 9 J', why: 'Multiply.' }] },
    vocab: [
      { term: 'Kinetic energy', def: 'Energy of motion.' }, { term: 'Potential energy', def: 'Stored energy due to position or condition.' }, { term: 'Conduction', def: 'Heat transfer by direct contact.' },
      { term: 'Convection', def: 'Heat transfer by the movement of fluids.' }, { term: 'Radiation', def: 'Energy transfer by electromagnetic waves.' }, { term: 'Joule', def: 'The SI unit of energy.' },
    ],
    questions: [
      ['A roller coaster at the top of a hill has the most…', 'Potential energy', ['Kinetic energy', 'Sound energy', 'Nuclear energy'], 'Height gives stored energy.'],
      ['A metal spoon gets hot in soup. This is…', 'Conduction', ['Convection', 'Radiation', 'Insulation'], 'Heat moves through direct contact.'],
      ['The Sun warms Earth through…', 'Radiation', ['Conduction', 'Convection', 'Friction'], 'Space has no matter to conduct or convect heat.'],
      ['If a ball\'s speed doubles, its kinetic energy…', 'Quadruples', ['Doubles', 'Halves', 'Stays the same'], 'KE depends on v², and 2² = 4.', 2],
      ['A battery stores energy as…', 'Chemical energy', ['Kinetic energy', 'Light energy', 'Sound energy'], 'Chemical reactions release its energy.'],
    ],
    gens: [energy],
  }),
  bankTopic({
    id: 'sci-waves', title: 'Waves',
    lesson: `A **wave** carries energy from place to place without carrying matter.
**Mechanical waves** (like sound) need a **medium**. **Electromagnetic waves** (like light) can travel through empty space.
**Transverse** waves move up and down, perpendicular to the direction of travel. **Longitudinal** waves compress and stretch, parallel to travel.
**Wavelength** (λ): distance between crests. **Frequency** (f): waves per second (Hz). **Amplitude**: height of the wave (energy).
Wave speed: **v = f × λ**. Higher frequency sound = higher **pitch**; bigger amplitude = louder.`,
    example: { problem: 'A sound wave has frequency 340 Hz and wavelength 1 m. Find its speed.', steps: [{ step: 'v = f × λ', why: 'Wave speed equation.' }, { step: 'v = 340 × 1 = 340 m/s', why: 'Multiply.' }] },
    vocab: [
      { term: 'Wavelength', def: 'The distance from one crest to the next.' }, { term: 'Frequency', def: 'Number of waves passing a point per second (Hz).' }, { term: 'Amplitude', def: 'The height of a wave; relates to energy.' },
      { term: 'Medium', def: 'The material a wave travels through.' }, { term: 'Transverse wave', def: 'Particles move perpendicular to the wave direction.' }, { term: 'Longitudinal wave', def: 'Particles move parallel to the wave direction.' },
      { term: 'Pitch', def: 'How high or low a sound is; depends on frequency.' },
    ],
    questions: [
      ['Sound cannot travel through…', 'A vacuum (empty space)', ['Water', 'Steel', 'Air'], 'Sound is a mechanical wave that needs a medium.'],
      ['A louder sound has a greater…', 'Amplitude', ['Frequency', 'Wavelength', 'Speed'], 'Amplitude relates to energy/volume.'],
      ['A higher-pitched note has a higher…', 'Frequency', ['Amplitude', 'Wavelength', 'Mass'], 'Pitch depends on how many vibrations per second.'],
      ['Sound is what type of wave?', 'Longitudinal', ['Transverse', 'Electromagnetic', 'Surface only'], 'It is made of compressions and rarefactions.'],
      ['Light is what type of wave?', 'Electromagnetic', ['Mechanical', 'Longitudinal', 'Sound'], 'It can travel through space.'],
    ],
    gens: [wave],
  }),
  bankTopic({
    id: 'sci-earth', title: "Earth's History",
    lesson: `Scientists study Earth's past using **rock layers** and **fossils**.
**Law of Superposition**: in undisturbed rock layers, the oldest layers are at the bottom.
**Relative dating** puts events in order; **absolute dating** (radiometric dating) finds actual ages using **half-lives** of radioactive elements.
**Index fossils** lived for a short time over a wide area — great for matching layers.
The **geologic time scale** divides Earth's 4.6-billion-year history into eons, eras (Paleozoic, Mesozoic, Cenozoic), and periods.
**Plate tectonics**: Earth's crust is broken into plates that move, causing earthquakes, volcanoes, and mountains.`,
    example: { problem: 'Layer A is on top of Layer B, which is on top of Layer C. Which is oldest?', steps: [{ step: 'Use the Law of Superposition.', why: 'Layers are deposited on top of older ones.' }, { step: 'Layer C (bottom) is oldest.', why: 'It was deposited first.' }] },
    vocab: [
      { term: 'Law of Superposition', def: 'In undisturbed layers, older rocks are below younger ones.' }, { term: 'Fossil', def: 'Preserved remains or traces of an ancient organism.' },
      { term: 'Index fossil', def: 'A fossil used to date rock layers because it lived briefly but widely.' }, { term: 'Half-life', def: 'Time for half of a radioactive sample to decay.' },
      { term: 'Relative dating', def: 'Ordering events without exact ages.' }, { term: 'Absolute dating', def: 'Finding the actual age of rocks, often by radioactive decay.' },
      { term: 'Plate tectonics', def: 'The theory that Earth\'s crust is made of moving plates.' },
    ],
    questions: [
      ['Dinosaurs lived mainly in which era?', 'Mesozoic', ['Cenozoic', 'Paleozoic', 'Precambrian'], 'This era is called the "Age of Reptiles."'],
      ['A sample has a half-life of 1,000 years. After 2,000 years, how much remains?', '¼', ['½', '⅛', 'None'], 'Each half-life cuts the amount in half: 1 → ½ → ¼.', 1],
      ['Which is the best index fossil?', 'Lived a short time over a large area', ['Lived a long time in one place', 'Is very large', 'Is still alive today'], 'Index fossils narrow down a time period across many locations.'],
      ['Earth is about how old?', '4.6 billion years', ['6,000 years', '65 million years', '100 billion years'], 'Based on radiometric dating of meteorites and rocks.'],
      ['Earthquakes happen most often…', 'At plate boundaries', ['In the middle of plates', 'Only in oceans', 'Only at the poles'], 'Plates grind, pull apart, or collide there.'],
    ],
  }),
  bankTopic({
    id: 'sci-genetics', title: 'Genetics',
    lesson: `**Genes** are sections of **DNA** that code for traits. Each organism gets one **allele** (version of a gene) from each parent.
**Dominant** alleles (capital letter, B) show up if at least one is present. **Recessive** alleles (lowercase, b) only show when there are two (bb).
**Genotype** is the allele pair (BB, Bb, bb). **Phenotype** is the trait you see.
**Homozygous**: two of the same allele. **Heterozygous**: two different alleles.
A **Punnett square** predicts the chances of offspring genotypes.`,
    example: { problem: 'Cross Bb × Bb. What fraction of offspring are bb?', steps: [{ step: 'Draw a 2×2 square with B, b on top and B, b on the side.', why: 'Each parent gives one allele.' }, { step: 'Boxes: BB, Bb, Bb, bb', why: 'Combine the row and column letters.' }, { step: '1 of 4 is bb → ¼ = 25%', why: 'Count the boxes.' }] },
    vocab: [
      { term: 'Gene', def: 'A segment of DNA that codes for a trait.' }, { term: 'Allele', def: 'A version of a gene.' }, { term: 'Dominant', def: 'An allele that shows when at least one copy is present.' },
      { term: 'Recessive', def: 'An allele that shows only with two copies.' }, { term: 'Genotype', def: 'The combination of alleles (e.g., Bb).' }, { term: 'Phenotype', def: 'The physical trait that shows.' },
      { term: 'Homozygous', def: 'Two identical alleles (BB or bb).' }, { term: 'Heterozygous', def: 'Two different alleles (Bb).' },
    ],
    questions: [
      ['Bb is an example of a…', 'Heterozygous genotype', ['Homozygous genotype', 'Phenotype', 'Recessive trait'], 'Two different letters.'],
      ['"Brown eyes" is a…', 'Phenotype', ['Genotype', 'Allele', 'Gene pair'], 'It is the trait you can observe.'],
      ['How many alleles for each gene do you get from each parent?', '1', ['2', '0', '4'], 'One from mom, one from dad.'],
      ['Where is DNA found in a plant or animal cell?', 'In the nucleus', ['In the cell wall', 'Only in the blood', 'In the membrane'], 'The nucleus is the control center.'],
      ['A change in a DNA sequence is called a…', 'Mutation', ['Phenotype', 'Punnett square', 'Adaptation'], 'Mutations can be harmful, helpful, or neutral.', 1],
    ],
    gens: [punnett],
  }),
  bankTopic({
    id: 'sci-ecosystems', title: 'Ecosystems',
    lesson: `An **ecosystem** includes all the living (**biotic**) and nonliving (**abiotic**) things in an area.
**Producers** (plants) make food by photosynthesis. **Consumers** eat other organisms: herbivores, carnivores, omnivores. **Decomposers** break down dead matter.
A **food chain** shows one path of energy; a **food web** shows many connected chains.
Only about **10%** of energy passes to the next level of an **energy pyramid** — the rest is used or lost as heat.
Relationships: **predation**, **competition**, and **symbiosis** (mutualism, commensalism, parasitism).`,
    example: { problem: 'Grass has 10,000 J of energy. How much reaches a hawk in grass → mouse → snake → hawk?', steps: [{ step: 'Each step passes about 10%.', why: 'The 10% rule.' }, { step: 'Mouse: 1,000 J → Snake: 100 J → Hawk: 10 J', why: 'Multiply by 0.1 three times.' }] },
    vocab: [
      { term: 'Producer', def: 'An organism that makes its own food (e.g., plants).' }, { term: 'Consumer', def: 'An organism that eats other organisms.' }, { term: 'Decomposer', def: 'Breaks down dead organisms and returns nutrients.' },
      { term: 'Biotic factor', def: 'A living part of an ecosystem.' }, { term: 'Abiotic factor', def: 'A nonliving part of an ecosystem (sunlight, water, soil).' }, { term: 'Mutualism', def: 'A relationship where both species benefit.' },
      { term: 'Parasitism', def: 'One organism benefits while the other is harmed.' }, { term: 'Commensalism', def: 'One benefits; the other is not affected.' },
    ],
    questions: [
      ['Which is an abiotic factor?', 'Sunlight', ['Grass', 'Bacteria', 'Deer'], 'Abiotic means not living.'],
      ['Bees get nectar and flowers get pollinated. This is…', 'Mutualism', ['Parasitism', 'Commensalism', 'Competition'], 'Both species benefit.'],
      ['A tick feeding on a dog is…', 'Parasitism', ['Mutualism', 'Commensalism', 'Predation'], 'One benefits, one is harmed.'],
      ['Mushrooms are…', 'Decomposers', ['Producers', 'Herbivores', 'Carnivores'], 'They break down dead material.'],
      ['Where does almost all energy in a food chain start?', 'The Sun', ['Decomposers', 'Top predators', 'Soil'], 'Producers capture sunlight.'],
      ['If producers have 5,000 J, about how much do primary consumers get?', '500 J', ['5,000 J', '50 J', '2,500 J'], 'About 10% passes up each level.', 1],
    ],
  }),
];
