import { bankTopic } from './bank';
import type { Topic } from './types';

// ---------------- ENGLISH ----------------
export const ENGLISH: Topic[] = [
  bankTopic({
    id: 'eng-grammar', title: 'Grammar',
    lesson: `**Parts of speech**: noun (person/place/thing), verb (action/being), adjective (describes a noun), adverb (describes a verb/adjective), pronoun, preposition, conjunction.
A **complete sentence** needs a **subject** and a **verb** and expresses a complete thought.
A **fragment** is missing one of those. A **run-on** joins two sentences without correct punctuation.
Fix run-ons with a period, a semicolon, or a comma + **FANBOYS** conjunction (for, and, nor, but, or, yet, so).
**Subject–verb agreement**: singular subjects take singular verbs ("The dog runs"), plural subjects take plural verbs ("The dogs run").
Commonly confused: **their/there/they're**, **its/it's**, **your/you're**, **affect/effect**.`,
    example: { problem: 'Fix the run-on: "I studied all night I aced the test."', steps: [{ step: 'Find the two complete thoughts.', why: '"I studied all night" and "I aced the test" can each stand alone.' }, { step: 'Join them correctly: "I studied all night, so I aced the test."', why: 'A comma + FANBOYS conjunction connects two independent clauses.' }] },
    vocab: [
      { term: 'Noun', def: 'A person, place, thing, or idea.' }, { term: 'Verb', def: 'A word that shows action or a state of being.' }, { term: 'Adjective', def: 'A word that describes a noun.' },
      { term: 'Adverb', def: 'A word that describes a verb, adjective, or another adverb.' }, { term: 'Fragment', def: 'An incomplete sentence.' }, { term: 'Run-on sentence', def: 'Two or more sentences joined incorrectly.' },
      { term: 'Independent clause', def: 'A group of words with a subject and verb that can stand alone.' }, { term: 'Conjunction', def: 'A word that joins words or clauses (and, but, so…).' },
    ],
    questions: [
      ['Which is a complete sentence?', 'The band played loudly.', ['Because the band played.', 'Playing loudly all night.', 'The band that played.'], 'It needs a subject, a verb, and a complete thought.'],
      ['Choose the correct word: "___ going to the game later."', "They're", ['Their', 'There', 'Theyre'], '"They are" going… — the contraction.'],
      ['Choose the correct verb: "Each of the students ___ a book."', 'has', ['have', 'having', 'are having'], '"Each" is singular.', 1],
      ['In "She quickly finished," the word "quickly" is a(n)…', 'Adverb', ['Adjective', 'Noun', 'Verb'], 'It describes HOW she finished (a verb).'],
      ['Which correctly fixes: "It was late we went home."', 'It was late, so we went home.', ['It was late, we went home.', 'It was late we, went home.', 'It was, late we went home.'], 'Use a comma + a FANBOYS conjunction.'],
      ['Choose: "The cat licked ___ paw."', 'its', ["it's", 'its\'', 'it is'], '"It\'s" always means "it is."'],
      ['Which sentence is written in active voice?', 'Maya kicked the ball.', ['The ball was kicked by Maya.', 'The ball had been kicked.', 'The ball is being kicked.'], 'In active voice, the subject does the action.', 1],
      ['The rain ___ the game. (affect/effect)', 'affected', ['effected', 'effect', 'affection'], 'Affect is usually the verb; effect is usually the noun.', 2],
    ],
  }),
  bankTopic({
    id: 'eng-vocab', title: 'Vocabulary & Word Parts',
    lesson: `You can figure out many words by breaking them into parts:
**Prefixes** (front): un-/in-/dis- (not), re- (again), pre- (before), sub- (under), inter- (between), trans- (across).
**Roots** (middle): bio (life), geo (earth), graph (write), port (carry), dict (say), spect (look), chron (time).
**Suffixes** (end): -able (can be), -less (without), -ology (study of), -ful (full of).
**Context clues**: look at the words around an unknown word for definitions, examples, synonyms, or contrasts.`,
    example: { problem: 'What does "chronology" probably mean?', steps: [{ step: 'chron = time', why: 'Root meaning.' }, { step: '-ology = study of / ordered account', why: 'Suffix meaning.' }, { step: '"The order of events in time."', why: 'Put the parts together and check with the context.' }] },
    vocab: [
      { term: 'bene-', def: 'Good or well (benefit, benevolent).' }, { term: 'mal-', def: 'Bad (malfunction, malicious).' }, { term: 'Ambiguous', def: 'Having more than one possible meaning.' },
      { term: 'Meticulous', def: 'Very careful and precise.' }, { term: 'Reluctant', def: 'Unwilling or hesitant.' }, { term: 'Resilient', def: 'Able to recover quickly from difficulty.' },
      { term: 'Inevitable', def: 'Certain to happen; unavoidable.' }, { term: 'Scrutinize', def: 'To examine very closely.' }, { term: 'Synonym', def: 'A word with the same or similar meaning.' }, { term: 'Antonym', def: 'A word with the opposite meaning.' },
    ],
    questions: [
      ['"Transport" means to…', 'Carry across', ['Look at again', 'Write down', 'Say before'], 'trans = across, port = carry.'],
      ['"Biography" is writing about…', "A person's life", ['The earth', 'Time', 'Sounds'], 'bio = life, graph = write.'],
      ['"She was reluctant to jump, so she stood at the edge for ten minutes." Reluctant means…', 'Hesitant', ['Excited', 'Angry', 'Tired'], 'Context clue: she waited a long time.'],
      ['Which prefix means "before"?', 'pre-', ['post-', 'sub-', 're-'], 'Think of "preview."'],
      ['An antonym of "meticulous" is…', 'Careless', ['Careful', 'Precise', 'Thorough'], 'Antonym = opposite.', 1],
      ['"Inaudible" means…', 'Not able to be heard', ['Very loud', 'Heard again', 'Musical'], 'in = not, aud = hear, -ible = able.', 1],
    ],
  }),
  bankTopic({
    id: 'eng-reading', title: 'Reading Comprehension',
    lesson: `Active readers look for:
**Main idea** — what the whole passage is mostly about. **Supporting details** back it up.
**Inference** — a smart guess using clues from the text + what you know.
**Theme** — the life lesson or message (not just the topic).
**Author's purpose** — to persuade, inform, or entertain.
**Text evidence** — quote or point to the exact words that support your answer.

Passage: *Lena had practiced the same eight measures for a week. Every night her fingers stumbled on the jump to the high E. On Friday she slowed the metronome way down and played the jump fifty times, slowly and perfectly. On Saturday, at full speed, her hand landed right on it — as if it had always known the way.*`,
    example: { problem: 'What is the theme of the Lena passage?', steps: [{ step: 'What does the character learn?', why: 'Theme comes from how a character changes.' }, { step: 'She improved only after slow, focused practice.', why: 'That\'s the key detail.' }, { step: 'Theme: patient, careful practice leads to success.', why: 'A theme is a general life lesson, stated as a sentence.' }] },
    vocab: [
      { term: 'Main idea', def: 'The most important point of a passage.' }, { term: 'Inference', def: 'A conclusion based on evidence and reasoning.' }, { term: 'Theme', def: 'The message or lesson of a story.' },
      { term: "Author's purpose", def: 'Why the author wrote: to persuade, inform, or entertain.' }, { term: 'Text evidence', def: 'Details from the text that support an answer.' }, { term: 'Point of view', def: 'Who is telling the story (1st, 2nd, 3rd person).' },
    ],
    questions: [
      ['(Lena passage) What can you infer about why Lena slowed the metronome?', 'Playing fast kept her making the same mistake', ['She was bored', 'Her metronome broke', 'She wanted to quit'], 'Look at what happened before she slowed down.'],
      ['(Lena passage) Which detail best supports the theme?', 'She played the jump fifty times, slowly and perfectly', ['She practiced eight measures', 'It was Friday', 'The note was an E'], 'Which detail shows the lesson in action?'],
      ['An article explaining how volcanoes form is mainly written to…', 'Inform', ['Persuade', 'Entertain', 'Apologize'], 'It teaches facts.'],
      ['"I walked to the window and saw the storm." This is told in…', 'First person', ['Second person', 'Third person limited', 'Third person omniscient'], 'Look for "I."'],
      ['A theme should be stated as…', 'A complete sentence with a life lesson', ['One word like "friendship"', 'A summary of the plot', 'The title'], 'A topic is one word; a theme says something about the topic.', 1],
      ['Passage: "The old bridge groaned as the truck crept across; drivers behind it held their breath." What can you infer?', 'The bridge might be unsafe', ['The truck is new', 'The drivers are asleep', 'The bridge is made of glass'], 'What do "groaned" and "held their breath" suggest?', 1],
    ],
  }),
  bankTopic({
    id: 'eng-figurative', title: 'Figurative Language',
    lesson: `Figurative language says something in a non-literal way to create an image or feeling.
**Simile**: compares using "like" or "as" (brave as a lion).
**Metaphor**: compares directly (the classroom was a zoo).
**Personification**: gives human traits to non-human things (the wind whispered).
**Hyperbole**: extreme exaggeration (I've told you a million times).
**Onomatopoeia**: a word that sounds like what it means (buzz, crash).
**Alliteration**: repeated beginning sounds (Peter Piper picked).
**Idiom**: a phrase whose meaning isn't literal (break a leg).`,
    example: { problem: 'Identify: "The stars danced across the sky."', steps: [{ step: 'Is there "like" or "as"? No.', why: 'That rules out a simile.' }, { step: 'Stars are doing a human action (dancing).', why: 'Human traits given to an object.' }, { step: 'Personification.', why: 'That\'s the definition.' }] },
    vocab: [
      { term: 'Simile', def: 'A comparison using "like" or "as".' }, { term: 'Metaphor', def: 'A direct comparison without "like" or "as".' }, { term: 'Personification', def: 'Giving human qualities to non-human things.' },
      { term: 'Hyperbole', def: 'Extreme exaggeration for effect.' }, { term: 'Onomatopoeia', def: 'A word that imitates a sound.' }, { term: 'Alliteration', def: 'Repetition of the same beginning consonant sound.' }, { term: 'Idiom', def: 'An expression whose meaning is not literal.' },
    ],
    questions: [
      ['"Her smile was as bright as the sun."', 'Simile', ['Metaphor', 'Hyperbole', 'Idiom'], 'Look for "like" or "as".'],
      ['"This backpack weighs a ton!"', 'Hyperbole', ['Simile', 'Personification', 'Onomatopoeia'], 'Is it an extreme exaggeration?'],
      ['"The thunder grumbled angrily."', 'Personification', ['Metaphor', 'Alliteration', 'Simile'], 'Can thunder really be angry?'],
      ['"Sizzle went the bacon."', 'Onomatopoeia', ['Idiom', 'Hyperbole', 'Metaphor'], 'The word makes the sound.'],
      ['"Time is a thief."', 'Metaphor', ['Simile', 'Personification', 'Onomatopoeia'], 'A direct comparison with no "like" or "as".'],
      ['"Silly snakes slither silently."', 'Alliteration', ['Onomatopoeia', 'Hyperbole', 'Idiom'], 'Listen to the first sounds.'],
      ['"It\'s raining cats and dogs."', 'Idiom', ['Simile', 'Personification', 'Onomatopoeia'], 'The meaning is not literal and is a common expression.', 1],
    ],
  }),
  bankTopic({
    id: 'eng-essay', title: 'Essay Structure & Writing',
    lesson: `A strong essay has:
**Introduction** — a **hook** to grab attention, background, and a **thesis statement** (your main claim, usually the last sentence of the intro).
**Body paragraphs** — each starts with a **topic sentence**, then evidence, then **explanation** (why the evidence proves your point). Try the **CER** pattern: Claim, Evidence, Reasoning.
**Transitions** connect ideas: first, furthermore, however, for example, as a result, in conclusion.
**Conclusion** — restate the thesis in new words, sum up, and end with a final thought.
Use the **Essay Coach** below to get feedback on YOUR writing. It never writes the essay for you.`,
    example: { problem: 'Turn the topic "school uniforms" into a thesis.', steps: [{ step: 'Take a position.', why: 'A thesis is arguable, not just a fact.' }, { step: 'Add reasons: "Schools should not require uniforms because they limit self-expression and add costs for families."', why: 'A strong thesis previews your main points.' }] },
    vocab: [
      { term: 'Thesis statement', def: 'The main claim of an essay.' }, { term: 'Hook', def: 'An opening line that grabs the reader\'s attention.' }, { term: 'Topic sentence', def: 'The first sentence of a body paragraph stating its main point.' },
      { term: 'Transition', def: 'A word or phrase that connects ideas.' }, { term: 'Counterclaim', def: 'An opposing argument that the writer addresses.' }, { term: 'Evidence', def: 'Facts, quotes, or examples that support a claim.' },
    ],
    questions: [
      ['Where does the thesis usually go?', 'End of the introduction', ['First sentence of the conclusion', 'Middle of body paragraph 2', 'In the title'], 'It sets up everything that follows.'],
      ['Which is the strongest thesis?', 'Homework should be limited because it causes stress and cuts into sleep.', ['Homework exists.', 'This essay is about homework.', 'Some people like homework and some don\'t.'], 'It takes a position and gives reasons.'],
      ['Which transition shows contrast?', 'However', ['Furthermore', 'For example', 'First'], 'Contrast means "the opposite idea."'],
      ['After you give evidence, you should…', 'Explain how it supports your claim', ['Start a new topic', 'Repeat the evidence', 'End the essay'], 'Evidence doesn\'t speak for itself — reasoning connects it.'],
      ['A counterclaim is used to…', 'Address the other side and show why your side is stronger', ['Change your thesis', 'Confuse the reader', 'Fill space'], 'It makes an argument more convincing.', 1],
    ],
  }),
];

// ---------------- HISTORY ----------------
export const HISTORY: Topic[] = [
  bankTopic({
    id: 'his-colonies', title: 'The Thirteen Colonies',
    lesson: `England founded 13 colonies along the Atlantic coast between 1607 and 1732.
**Jamestown** (1607, Virginia) was the first permanent English settlement; tobacco made it profitable.
The **Pilgrims** arrived on the Mayflower (1620) and wrote the **Mayflower Compact**, an early agreement for self-government.
**New England** colonies: fishing, shipbuilding, trade; Puritans. **Middle** colonies ("breadbasket"): grain, religious tolerance. **Southern** colonies: plantations (tobacco, rice, indigo) that relied on enslaved labor.
The **House of Burgesses** (1619) was the first representative assembly in the colonies.`,
    example: { problem: 'Why were the Middle Colonies called the "breadbasket"?', steps: [{ step: 'Look at their economy.', why: 'Nicknames often describe what a region produces.' }, { step: 'Fertile soil and a good climate let them grow lots of wheat and grain.', why: 'Grain → bread.' }] },
    vocab: [
      { term: 'Jamestown', def: 'First permanent English settlement in North America (1607).' }, { term: 'Mayflower Compact', def: 'Agreement by the Pilgrims to govern themselves by majority rule (1620).' },
      { term: 'House of Burgesses', def: 'First elected legislature in the English colonies (Virginia, 1619).' }, { term: 'Mercantilism', def: 'Economic idea that colonies exist to make the home country rich.' },
      { term: 'Indentured servant', def: 'Person who worked for several years in exchange for passage to America.' }, { term: 'Triangular trade', def: 'Trade route linking Europe, Africa, and the Americas, including the slave trade.' },
    ],
    questions: [
      ['The first permanent English settlement was…', 'Jamestown', ['Plymouth', 'Boston', 'Roanoke'], 'Founded in 1607 in Virginia.'],
      ['Which region relied most on plantation agriculture?', 'Southern Colonies', ['New England', 'Middle Colonies', 'Canada'], 'Long growing season, cash crops.'],
      ['The Mayflower Compact is important because it…', 'Was an early example of self-government', ['Ended the Revolution', 'Created the Constitution', 'Banned slavery'], 'The colonists agreed to make and follow their own laws.'],
      ['Why did many Puritans come to New England?', 'Religious freedom for their own beliefs', ['To find gold', 'To grow tobacco', 'To escape the French'], 'They wanted to practice their religion their way.'],
      ['Under mercantilism, colonies existed to…', 'Benefit the mother country', ['Become independent', 'Trade freely with anyone', 'Avoid taxes'], 'Raw materials flowed to England; goods flowed back.', 1],
    ],
  }),
  bankTopic({
    id: 'his-revolution', title: 'The American Revolution',
    lesson: `After the French and Indian War (1754–1763), Britain taxed the colonies to pay its debts: **Stamp Act**, **Townshend Acts**, **Tea Act**.
Colonists protested "**no taxation without representation**." Events: **Boston Massacre** (1770), **Boston Tea Party** (1773), **Intolerable Acts** (1774).
Fighting began at **Lexington and Concord** (1775). The **Declaration of Independence** (July 4, 1776), written mainly by **Thomas Jefferson**, listed grievances and said people have unalienable rights to "life, liberty, and the pursuit of happiness."
The victory at **Saratoga** (1777) convinced France to help. The war ended with the British surrender at **Yorktown** (1781) and the **Treaty of Paris** (1783).`,
    example: { problem: 'Why was Saratoga a turning point?', steps: [{ step: 'What happened after Saratoga?', why: 'Turning points change the direction of events.' }, { step: 'France became an ally and sent money, troops, and ships.', why: 'The win proved the Americans could beat the British.' }] },
    vocab: [
      { term: 'Stamp Act', def: '1765 tax on printed materials; sparked protest.' }, { term: 'Boston Tea Party', def: '1773 protest where colonists dumped British tea into Boston Harbor.' },
      { term: 'Declaration of Independence', def: '1776 document declaring the colonies free from Britain.' }, { term: 'Loyalist', def: 'A colonist who stayed loyal to Britain.' },
      { term: 'Patriot', def: 'A colonist who supported independence.' }, { term: 'Treaty of Paris (1783)', def: 'Ended the Revolutionary War and recognized U.S. independence.' },
    ],
    questions: [
      ['Where were the first shots of the Revolution fired?', 'Lexington and Concord', ['Yorktown', 'Saratoga', 'Bunker Hill'], '1775, in Massachusetts.'],
      ['Who was the main author of the Declaration of Independence?', 'Thomas Jefferson', ['George Washington', 'Benjamin Franklin', 'John Adams'], 'He was from Virginia and later became the 3rd president.'],
      ['"No taxation without representation" meant colonists…', "Had no vote in Parliament, which taxed them", ['Didn\'t want any government', 'Wanted more taxes', 'Wanted a king'], 'They objected to being taxed by a body they couldn\'t elect.'],
      ['Which battle ended major fighting in the war?', 'Yorktown', ['Saratoga', 'Trenton', 'Lexington'], '1781, Cornwallis surrendered.'],
      ['Put in order: Stamp Act, Declaration of Independence, Boston Tea Party', 'Stamp Act → Tea Party → Declaration', ['Tea Party → Stamp Act → Declaration', 'Declaration → Tea Party → Stamp Act', 'Stamp Act → Declaration → Tea Party'], '1765, 1773, 1776.', 1],
    ],
  }),
  bankTopic({
    id: 'his-constitution', title: 'The Constitution',
    lesson: `The first plan, the **Articles of Confederation**, made a weak national government (no power to tax, no national army, no president).
At the **Constitutional Convention** (1787), leaders wrote a new **Constitution**. The **Great Compromise** created a two-house Congress: the Senate (equal votes) and the House (based on population).
Key principles: **popular sovereignty**, **federalism** (power shared between national and state governments), **separation of powers**, **checks and balances**, **limited government**.
The **Bill of Rights** (first 10 amendments, 1791) protects freedoms like speech, religion, press, and a fair trial.`,
    example: { problem: 'How is the president "checked" by Congress?', steps: [{ step: 'Recall checks and balances.', why: 'Each branch can limit the others.' }, { step: 'Congress can override a veto with a 2/3 vote, the Senate approves appointments, and Congress can impeach.', why: 'These are powers over the executive branch.' }] },
    vocab: [
      { term: 'Articles of Confederation', def: 'First U.S. constitution; created a weak central government.' }, { term: 'Great Compromise', def: 'Created a two-house legislature: Senate and House of Representatives.' },
      { term: 'Federalism', def: 'Division of power between national and state governments.' }, { term: 'Separation of powers', def: 'Splitting government into legislative, executive, judicial branches.' },
      { term: 'Checks and balances', def: 'Each branch can limit the powers of the others.' }, { term: 'Bill of Rights', def: 'The first ten amendments to the Constitution.' }, { term: 'Amendment', def: 'A change or addition to the Constitution.' },
    ],
    questions: [
      ['Which branch makes laws?', 'Legislative', ['Executive', 'Judicial', 'Military'], 'Congress.'],
      ['Freedom of speech is protected by which amendment?', 'First', ['Second', 'Fifth', 'Tenth'], 'It also covers religion, press, assembly, petition.'],
      ['A major weakness of the Articles of Confederation was…', 'Congress could not collect taxes', ['The president was too strong', 'There were too many courts', 'States had no power'], 'The national government couldn\'t pay its bills.'],
      ['The president vetoing a bill is an example of…', 'Checks and balances', ['Federalism', 'Popular sovereignty', 'Amendment'], 'One branch limits another.'],
      ['The Great Compromise settled a dispute over…', 'Representation in Congress', ['Slavery in the territories', 'Taxes on tea', 'The capital city'], 'Large states vs. small states.', 1],
    ],
  }),
  bankTopic({
    id: 'his-expansion', title: 'Early Republic & Westward Expansion',
    lesson: `**George Washington** was the first president (1789). His Farewell Address warned against political parties and foreign alliances.
The **Louisiana Purchase** (1803, Jefferson) doubled the size of the U.S.; **Lewis and Clark** explored it.
The **War of 1812** against Britain boosted American nationalism.
The **Monroe Doctrine** (1823) warned Europe not to colonize the Americas.
**Manifest Destiny** was the belief that the U.S. should expand coast to coast. It led to the Oregon Trail, the Mexican-American War, and the **Trail of Tears** — the forced removal of Cherokee and other Native nations under the Indian Removal Act (1830).`,
    example: { problem: 'Why was the Louisiana Purchase important?', steps: [{ step: 'Size: it doubled the U.S.', why: 'Huge new land for farming and settlement.' }, { step: 'Control of the Mississippi River and New Orleans.', why: 'Farmers needed the river to ship goods.' }] },
    vocab: [
      { term: 'Louisiana Purchase', def: '1803 purchase of land from France that doubled U.S. size.' }, { term: 'Manifest Destiny', def: 'Belief that the U.S. was meant to expand across the continent.' },
      { term: 'Monroe Doctrine', def: '1823 policy warning Europe against new colonies in the Americas.' }, { term: 'Trail of Tears', def: 'Forced removal of the Cherokee and other nations to the west, causing thousands of deaths.' },
      { term: 'Nationalism', def: 'Strong pride in and loyalty to one\'s country.' },
    ],
    questions: [
      ['Who was president during the Louisiana Purchase?', 'Thomas Jefferson', ['George Washington', 'James Monroe', 'Andrew Jackson'], '1803.'],
      ['Manifest Destiny was the idea that…', 'The U.S. should expand to the Pacific', ['The U.S. should rejoin Britain', 'Slavery should end', 'States should have no power'], '"From sea to shining sea."'],
      ['The Trail of Tears resulted from…', 'The Indian Removal Act', ['The Monroe Doctrine', 'The War of 1812', 'The Bill of Rights'], 'Passed under Andrew Jackson in 1830.'],
      ['Lewis and Clark\'s expedition explored…', 'The Louisiana Territory and the Northwest', ['Florida', 'New England', 'Mexico City'], 'They reached the Pacific in 1805.'],
      ['Washington\'s Farewell Address warned against…', 'Political parties and permanent foreign alliances', ['Westward expansion', 'Paying taxes', 'Building a navy'], 'He feared division and entanglement.', 1],
    ],
  }),
  bankTopic({
    id: 'his-civilwar', title: 'Civil War & Reconstruction',
    lesson: `Tensions over **slavery** and **states' rights** grew as the nation expanded (Missouri Compromise, Compromise of 1850, Kansas-Nebraska Act, **Dred Scott** decision).
After **Abraham Lincoln** was elected in 1860, Southern states **seceded** and formed the Confederacy. The war began at **Fort Sumter** (1861).
The **Emancipation Proclamation** (1863) freed enslaved people in Confederate states. **Gettysburg** (1863) was a turning point. The war ended at **Appomattox** (1865).
**Reconstruction** (1865–1877) rebuilt the South. The **13th** (ended slavery), **14th** (citizenship, equal protection), and **15th** (voting rights for Black men) Amendments were passed. After Reconstruction, **Jim Crow** laws took away many of these rights.`,
    example: { problem: 'What did the 14th Amendment do?', steps: [{ step: 'Recall the Reconstruction Amendments: 13, 14, 15.', why: 'They go in order: freedom, citizenship, voting.' }, { step: 'The 14th granted citizenship to everyone born in the U.S. and equal protection of the laws.', why: 'It is the "citizenship" amendment.' }] },
    vocab: [
      { term: 'Secession', def: 'Formally withdrawing from the Union.' }, { term: 'Emancipation Proclamation', def: '1863 order freeing enslaved people in Confederate states.' },
      { term: 'Reconstruction', def: 'Period of rebuilding the South after the Civil War (1865–1877).' }, { term: '13th Amendment', def: 'Abolished slavery (1865).' },
      { term: '14th Amendment', def: 'Granted citizenship and equal protection under the law (1868).' }, { term: '15th Amendment', def: 'Gave Black men the right to vote (1870).' }, { term: 'Jim Crow laws', def: 'Laws enforcing racial segregation in the South.' },
    ],
    questions: [
      ['Where did the Civil War begin?', 'Fort Sumter', ['Gettysburg', 'Appomattox', 'Antietam'], 'South Carolina, April 1861.'],
      ['Which amendment abolished slavery?', '13th', ['14th', '15th', '1st'], 'Order: freedom, citizenship, voting.'],
      ['The battle often called the turning point of the Civil War was…', 'Gettysburg', ['Bull Run', 'Fort Sumter', 'Yorktown'], 'July 1863; Lincoln later gave his famous address there.'],
      ['Who was president during the Civil War?', 'Abraham Lincoln', ['Andrew Johnson', 'Ulysses S. Grant', 'Jefferson Davis'], 'Davis was the Confederate president.'],
      ['Jim Crow laws were designed to…', 'Enforce segregation and limit Black rights', ['End slavery', 'Rebuild railroads', 'Give women the vote'], 'They appeared after Reconstruction ended.', 1],
    ],
  }),
  bankTopic({
    id: 'his-civics', title: 'Civics',
    lesson: `The U.S. is a **constitutional republic** — citizens elect representatives, and the Constitution is the supreme law.
**Legislative branch** (Congress: Senate + House) makes laws. **Executive branch** (President) enforces laws. **Judicial branch** (Supreme Court) interprets laws and can use **judicial review**.
How a bill becomes a law: introduced → committee → vote in both houses → president signs (or vetoes; Congress can override with 2/3).
Citizens' **rights** include speech, religion, and due process. **Responsibilities** include voting, jury duty, obeying laws, paying taxes, and staying informed.`,
    example: { problem: 'The president vetoes a bill. Can it still become law?', steps: [{ step: 'Recall checks and balances.', why: 'Congress can check the president.' }, { step: 'Yes — if 2/3 of both the House and Senate vote to override.', why: 'That is the veto override process.' }] },
    vocab: [
      { term: 'Republic', def: 'Government where citizens elect representatives.' }, { term: 'Judicial review', def: 'Power of courts to declare laws unconstitutional (Marbury v. Madison).' },
      { term: 'Veto', def: 'The president\'s power to reject a bill.' }, { term: 'Citizen', def: 'A legal member of a country.' }, { term: 'Due process', def: 'Fair legal procedures before the government takes life, liberty, or property.' },
    ],
    questions: [
      ['How many senators does each state have?', '2', ['1', 'Depends on population', '10'], 'Equal representation in the Senate.'],
      ['Which branch can declare a law unconstitutional?', 'Judicial', ['Legislative', 'Executive', 'State governors'], 'Judicial review.'],
      ['How many members are in the House of Representatives?', '435', ['100', '50', '538'], 'Based on state population.', 1],
      ['Which is a responsibility (not only a right) of citizens?', 'Serving on a jury', ['Freedom of religion', 'Freedom of speech', 'Right to bear arms'], 'Responsibilities are duties.'],
      ['What is needed to override a veto?', '2/3 vote of both houses', ['Simple majority of the Senate', 'Supreme Court approval', 'A vote by the states'], 'A high bar on purpose.', 1],
    ],
  }),
  bankTopic({
    id: 'his-geography', title: 'Geography',
    lesson: `The **five themes of geography**: **Location** (absolute: latitude/longitude; relative: near what?), **Place** (physical and human features), **Human–Environment Interaction**, **Movement** (people, goods, ideas), **Region** (areas with common features).
**Latitude** lines run east–west and measure north/south of the **Equator** (0°). **Longitude** lines run north–south and measure east/west of the **Prime Meridian** (0°).
Major U.S. features: **Appalachian** and **Rocky Mountains**, **Mississippi River**, **Great Plains**, **Great Lakes**.`,
    example: { problem: 'Is "40°N, 75°W" absolute or relative location?', steps: [{ step: 'It uses exact coordinates.', why: 'Latitude and longitude pinpoint a spot.' }, { step: 'Absolute location.', why: 'Relative location describes a place compared to others.' }] },
    vocab: [
      { term: 'Latitude', def: 'Imaginary lines measuring distance north or south of the Equator.' }, { term: 'Longitude', def: 'Imaginary lines measuring distance east or west of the Prime Meridian.' },
      { term: 'Absolute location', def: 'Exact position using coordinates.' }, { term: 'Relative location', def: 'Position described in relation to other places.' }, { term: 'Region', def: 'An area with common features.' },
    ],
    questions: [
      ['The longest river system in the U.S. is the…', 'Mississippi–Missouri', ['Colorado', 'Hudson', 'Rio Grande'], 'It drains the middle of the country.'],
      ['"Two blocks north of the library" is…', 'Relative location', ['Absolute location', 'Latitude', 'Region'], 'Described compared to something else.'],
      ['The Equator is 0° of…', 'Latitude', ['Longitude', 'Elevation', 'Time'], 'It divides the Northern and Southern Hemispheres.'],
      ['Which mountains are in the eastern U.S.?', 'Appalachians', ['Rockies', 'Sierra Nevada', 'Cascades'], 'Older, lower mountains.'],
      ['Building a dam is an example of which theme?', 'Human–Environment Interaction', ['Location', 'Movement', 'Place'], 'People changing their environment.', 1],
    ],
  }),
];

// ---------------- FOREIGN LANGUAGE ----------------
type Pair = [string, string];
function langTopics(lang: 'spanish' | 'french'): Topic[] {
  const S = lang === 'spanish';
  const L = S ? 'Spanish' : 'French';
  const sets: { id: string; title: string; lesson: string; words: Pair[]; grammarQ?: [string, string, string[], string][] }[] = [
    {
      id: 'greet', title: 'Greetings & Phrases',
      lesson: S ? `**Hola** (hello), **Buenos días** (good morning), **Buenas tardes** (good afternoon), **Buenas noches** (good night).
**¿Cómo estás?** (How are you? informal) — **Bien, gracias** (Fine, thanks).
**¿Cómo te llamas?** (What's your name?) — **Me llamo…** (My name is…).
**Por favor** (please), **Gracias** (thank you), **De nada** (you're welcome), **Adiós** / **Hasta luego** (goodbye / see you later).
Spanish questions and exclamations start with upside-down marks: **¿…?** **¡…!**`
        : `**Bonjour** (hello/good morning), **Bonsoir** (good evening), **Salut** (hi, informal).
**Comment ça va ?** (How's it going?) — **Ça va bien, merci** (Fine, thanks).
**Comment tu t'appelles ?** (What's your name?) — **Je m'appelle…** (My name is…).
**S'il te plaît** (please), **Merci** (thank you), **De rien** (you're welcome), **Au revoir** / **À bientôt** (goodbye / see you soon).
Use **tu** with friends and family, **vous** with adults you don't know well or groups.`,
      words: S ? [['hola', 'hello'], ['adiós', 'goodbye'], ['gracias', 'thank you'], ['por favor', 'please'], ['de nada', "you're welcome"], ['buenos días', 'good morning'], ['buenas noches', 'good night'], ['me llamo', 'my name is'], ['hasta luego', 'see you later'], ['lo siento', "I'm sorry"]]
        : [['bonjour', 'hello'], ['au revoir', 'goodbye'], ['merci', 'thank you'], ["s'il te plaît", 'please'], ['de rien', "you're welcome"], ['bonsoir', 'good evening'], ['bonne nuit', 'good night'], ["je m'appelle", 'my name is'], ['à bientôt', 'see you soon'], ['pardon', 'excuse me']],
    },
    {
      id: 'numbers', title: 'Numbers & Time',
      lesson: S ? `**uno, dos, tres, cuatro, cinco, seis, siete, ocho, nueve, diez**. 11–15: once, doce, trece, catorce, quince. 20: veinte. 30: treinta. 100: cien.
Days: **lunes, martes, miércoles, jueves, viernes, sábado, domingo** (not capitalized in Spanish).
Time: **¿Qué hora es?** — **Es la una** (1:00), **Son las tres** (3:00).`
        : `**un, deux, trois, quatre, cinq, six, sept, huit, neuf, dix**. 11–16: onze, douze, treize, quatorze, quinze, seize. 20: vingt. 30: trente. 100: cent.
Days: **lundi, mardi, mercredi, jeudi, vendredi, samedi, dimanche** (not capitalized in French).
Time: **Quelle heure est-il ?** — **Il est trois heures** (It's 3:00).`,
      words: S ? [['uno', 'one'], ['cinco', 'five'], ['siete', 'seven'], ['diez', 'ten'], ['doce', 'twelve'], ['quince', 'fifteen'], ['veinte', 'twenty'], ['lunes', 'Monday'], ['viernes', 'Friday'], ['domingo', 'Sunday']]
        : [['un', 'one'], ['cinq', 'five'], ['sept', 'seven'], ['dix', 'ten'], ['douze', 'twelve'], ['quinze', 'fifteen'], ['vingt', 'twenty'], ['lundi', 'Monday'], ['vendredi', 'Friday'], ['dimanche', 'Sunday']],
    },
    {
      id: 'family', title: 'Family & School',
      lesson: S ? `Family: **la madre** (mother), **el padre** (father), **el hermano / la hermana** (brother/sister), **los abuelos** (grandparents).
School: **la escuela** (school), **el libro** (book), **el cuaderno** (notebook), **el lápiz** (pencil), **la clase** (class), **la tarea** (homework).
Possessives: **mi** (my), **tu** (your), **su** (his/her).`
        : `Family: **la mère** (mother), **le père** (father), **le frère / la sœur** (brother/sister), **les grands-parents** (grandparents).
School: **l'école** (school), **le livre** (book), **le cahier** (notebook), **le crayon** (pencil), **la classe** (class), **les devoirs** (homework).
Possessives: **mon/ma/mes** (my), **ton/ta/tes** (your).`,
      words: S ? [['la madre', 'mother'], ['el padre', 'father'], ['el hermano', 'brother'], ['la hermana', 'sister'], ['los abuelos', 'grandparents'], ['la escuela', 'school'], ['el libro', 'book'], ['el lápiz', 'pencil'], ['la tarea', 'homework'], ['el cuaderno', 'notebook']]
        : [['la mère', 'mother'], ['le père', 'father'], ['le frère', 'brother'], ['la sœur', 'sister'], ['les grands-parents', 'grandparents'], ["l'école", 'school'], ['le livre', 'book'], ['le crayon', 'pencil'], ['les devoirs', 'homework'], ['le cahier', 'notebook']],
    },
    {
      id: 'food', title: 'Food & Everyday Words',
      lesson: S ? `**la comida** (food), **el agua** (water), **la leche** (milk), **el pan** (bread), **la manzana** (apple), **el pollo** (chicken), **el arroz** (rice), **el queso** (cheese).
**Tengo hambre** (I'm hungry), **Tengo sed** (I'm thirsty), **Me gusta…** (I like…), **No me gusta…** (I don't like…).`
        : `**la nourriture** (food), **l'eau** (water), **le lait** (milk), **le pain** (bread), **la pomme** (apple), **le poulet** (chicken), **le riz** (rice), **le fromage** (cheese).
**J'ai faim** (I'm hungry), **J'ai soif** (I'm thirsty), **J'aime…** (I like…), **Je n'aime pas…** (I don't like…).`,
      words: S ? [['el agua', 'water'], ['la leche', 'milk'], ['el pan', 'bread'], ['la manzana', 'apple'], ['el pollo', 'chicken'], ['el arroz', 'rice'], ['el queso', 'cheese'], ['tengo hambre', "I'm hungry"], ['tengo sed', "I'm thirsty"], ['me gusta', 'I like']]
        : [["l'eau", 'water'], ['le lait', 'milk'], ['le pain', 'bread'], ['la pomme', 'apple'], ['le poulet', 'chicken'], ['le riz', 'rice'], ['le fromage', 'cheese'], ["j'ai faim", "I'm hungry"], ["j'ai soif", "I'm thirsty"], ["j'aime", 'I like']],
    },
    {
      id: 'grammar', title: 'Grammar: Articles & Verbs',
      lesson: S ? `Nouns have **gender**: masculine usually ends in -o (**el** libro), feminine in -a (**la** mesa). Plural: **los / las**.
Adjectives match the noun: **el gato negro**, **las casas blancas**.
Present tense of **-ar** verbs (hablar): hablo, hablas, habla, hablamos, habláis, hablan.
**Ser** (permanent traits: soy, eres, es…) vs. **estar** (location & feelings: estoy, estás, está…).`
        : `Nouns have **gender**: **le** (masculine), **la** (feminine), **l'** before a vowel, **les** (plural).
Adjectives match the noun: **un chat noir**, **une maison blanche**.
Present tense of **-er** verbs (parler): je parle, tu parles, il/elle parle, nous parlons, vous parlez, ils parlent.
**Être** (to be): je suis, tu es, il est, nous sommes, vous êtes, ils sont. **Avoir** (to have): j'ai, tu as, il a…`,
      words: S ? [['hablar', 'to speak'], ['comer', 'to eat'], ['vivir', 'to live'], ['ser', 'to be (permanent)'], ['estar', 'to be (location/feeling)'], ['tener', 'to have'], ['ir', 'to go'], ['estudiar', 'to study']]
        : [['parler', 'to speak'], ['manger', 'to eat'], ['habiter', 'to live'], ['être', 'to be'], ['avoir', 'to have'], ['aller', 'to go'], ['étudier', 'to study'], ['finir', 'to finish']],
      grammarQ: S ? [
        ['Choose the article: ___ mesa', 'la', ['el', 'los', 'un'], 'Mesa ends in -a — feminine singular.'],
        ['Yo ___ español. (hablar)', 'hablo', ['habla', 'hablas', 'hablamos'], 'The "yo" form of -ar verbs ends in -o.'],
        ['Nosotros ___ en la clase. (estar)', 'estamos', ['somos', 'están', 'estoy'], 'Location uses estar; "nosotros" form.'],
        ['Ella ___ inteligente. (ser)', 'es', ['está', 'soy', 'son'], 'A trait uses ser; "ella" form.'],
        ['Choose: las casas ___ (white)', 'blancas', ['blanco', 'blanca', 'blancos'], 'Feminine plural noun → feminine plural adjective.'],
      ] : [
        ['Choose the article: ___ maison (feminine)', 'la', ['le', 'les', "l'"], 'Feminine singular.'],
        ['Je ___ français. (parler)', 'parle', ['parles', 'parlons', 'parlez'], 'The "je" form of -er verbs ends in -e.'],
        ['Nous ___ étudiants. (être)', 'sommes', ['êtes', 'sont', 'suis'], '"Nous" form of être.'],
        ["Tu ___ un chien ? (avoir)", 'as', ['ai', 'a', 'avons'], '"Tu" form of avoir.'],
        ['Choose: une maison ___ (white)', 'blanche', ['blanc', 'blancs', 'blanches'], 'Feminine singular adjective.'],
      ],
    },
  ];
  return sets.map((set) => {
    const [ex0, ex1] = set.words;
    return bankTopic({
      id: `lang-${lang}-${set.id}`, title: set.title, lesson: set.lesson,
      example: {
        problem: `Translate "${ex1[1]}" into ${L}.`,
        steps: [
          { step: `Find the word: "${ex1[1]}" = "${ex1[0]}".`, why: 'Look for it in the lesson vocabulary.' },
          { step: `Compare with a word you know: "${ex0[0]}" = "${ex0[1]}".`, why: 'Connecting new words to known words helps you remember them.' },
        ],
      },
      vocab: set.words.map(([term, def]) => ({ term, def })),
      questions: [
        ...(set.grammarQ ?? []).map((g) => [...g, 0] as [string, string, string[], string, number]),
        ...set.words.map(([w, m]) => [`What does "${w}" mean?`, m, set.words.filter((x) => x[1] !== m).map((x) => x[1]), `Think about when you'd use "${w}".`, 0] as [string, string, string[], string, number]),
        ...set.words.map(([w, m]) => [`How do you say "${m}" in ${L}?`, w, set.words.filter((x) => x[0] !== w).map((x) => x[0]), 'Say the options out loud — which one sounds familiar from the lesson?', 1] as [string, string, string[], string, number]),
      ],
    });
  });
}
export const SPANISH = langTopics('spanish');
export const FRENCH = langTopics('french');
