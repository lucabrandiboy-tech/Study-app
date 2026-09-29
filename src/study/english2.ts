import { bankTopic } from './bank';

export const ENGLISH2 = [
  bankTopic({
    id: 'eng-punctuation', title: 'Punctuation & Capitalization',
    lesson: `**Commas**: after introductory words ("However, …"), between items in a list, before FANBOYS joining two sentences, and around extra information ("My dog, a beagle, snores.").
**Apostrophes** show possession (the dog's bowl, the dogs' bowls) or contractions (can't). Never use one to make a plural!
**Quotation marks** go around exact words; periods and commas go inside: "Let's go," she said.
**Capitalize** proper nouns, the first word of a sentence, and titles (*To Kill a Mockingbird*).`,
    example: { problem: 'Fix: "my friends dog, max, cant find it\'s ball"', steps: [{ step: 'Capitalize "My" and the name "Max".', why: 'First word and proper noun.' }, { step: 'friend\'s dog (possession), can\'t (contraction), its ball (possessive its has no apostrophe).', why: 'Apostrophes only for possession and contractions; "its" is already possessive.' }, { step: '"My friend\'s dog, Max, can\'t find its ball."', why: 'Commas set off the extra name.' }] },
    vocab: [{ term: 'Apostrophe', def: 'Mark (\') showing possession or a contraction.' }, { term: 'Proper noun', def: 'The name of a specific person, place, or thing; always capitalized.' }, { term: 'Contraction', def: 'Two words shortened into one with an apostrophe.' }, { term: 'Appositive', def: 'A phrase that renames a noun, set off by commas.' }, { term: 'Quotation marks', def: 'Marks around someone\'s exact words.' }],
    questions: [
      ['Which is correct?', "The students' lockers were painted.", ["The student's' lockers were painted.", 'The students lockers\' were painted.', "The students's lockers were painted."], 'Plural noun ending in s: add the apostrophe after the s.'],
      ['Which is correctly punctuated?', '"I\'m ready," said Leo.', ['"I\'m ready", said Leo.', '"I\'m ready" said Leo.', 'I\'m ready, said Leo.'], 'The comma goes inside the quotation marks.'],
      ['Which needs a capital letter?', 'lake michigan', ['a lake', 'the river', 'my town'], 'Specific names are proper nouns.'],
      ['Choose the correct sentence.', 'After lunch, we went outside.', ['After lunch we, went outside.', 'After, lunch we went outside.', 'After lunch we went, outside.'], 'Comma after an introductory phrase.'],
      ['Which uses an apostrophe WRONGLY?', 'I bought three apple\'s.', ["It's raining.", "Sam's hat is red.", "They're late."], 'Plurals never need an apostrophe.', 1],
    ],
  }),
  bankTopic({
    id: 'eng-poetry', title: 'Poetry',
    lesson: `Poems use sound and form to create meaning.
**Stanza**: a group of lines (a poem's "paragraph"). **Rhyme scheme**: pattern of end rhymes, labeled with letters (ABAB, AABB).
**Meter**: the beat pattern of stressed/unstressed syllables (iambic = da-DUM). **Free verse** has no set rhyme or meter.
Forms: **haiku** (5-7-5 syllables), **sonnet** (14 lines), **limerick** (funny, AABBA), **ballad** (tells a story).
Sound devices: **rhyme, repetition, alliteration, onomatopoeia**. **Imagery** appeals to the five senses.`,
    example: { problem: 'Find the rhyme scheme: "The cat sat by the door / It watched the rain all day / It napped upon the floor / Then quietly slipped away."', steps: [{ step: 'door (A), day (B), floor (A), away (B)', why: 'Give each new end sound a new letter; matching sounds get the same letter.' }, { step: 'ABAB', why: 'Lines 1 & 3 rhyme, lines 2 & 4 rhyme.' }] },
    vocab: [{ term: 'Stanza', def: 'A group of lines in a poem.' }, { term: 'Rhyme scheme', def: 'The pattern of rhymes at the ends of lines.' }, { term: 'Meter', def: 'The rhythm pattern of stressed and unstressed syllables.' }, { term: 'Free verse', def: 'Poetry with no regular rhyme or meter.' }, { term: 'Haiku', def: 'A three-line poem of 5, 7, and 5 syllables.' }, { term: 'Imagery', def: 'Words that appeal to the senses.' }],
    questions: [
      ['A 14-line poem with a set rhyme scheme is a…', 'Sonnet', ['Haiku', 'Limerick', 'Free verse'], 'Shakespeare wrote 154 of them.'],
      ['How many syllables are in a haiku\'s second line?', '7', ['5', '3', '14'], 'The pattern is 5-7-5.'],
      ['"The crunch of leaves, the smell of smoke" mostly uses…', 'Imagery', ['Rhyme scheme', 'Meter', 'Dialogue'], 'It appeals to hearing and smell.'],
      ['A funny five-line poem with AABBA rhyme is a…', 'Limerick', ['Ballad', 'Sonnet', 'Ode'], '"There once was a man from…"'],
      ['Poetry without regular rhyme or rhythm is…', 'Free verse', ['A sonnet', 'Iambic', 'A couplet'], 'It follows natural speech.', 1],
    ],
  }),
  bankTopic({
    id: 'eng-narrative', title: 'Narrative Writing & Plot',
    lesson: `Stories follow a **plot structure**: **exposition** (characters, setting) → **rising action** (conflict builds) → **climax** (turning point) → **falling action** → **resolution**.
**Conflict** types: character vs. character, vs. self, vs. nature, vs. society.
Good narratives **show, don't tell** ("Her hands trembled" instead of "She was nervous"), use **dialogue**, sensory details, and a clear **point of view**.
Use transitions for time order: *later, meanwhile, suddenly, finally*.`,
    example: { problem: 'Turn "He was scared" into showing.', steps: [{ step: 'Think: what would a scared person do or feel?', why: 'Showing uses actions and senses.' }, { step: '"His heart pounded as he pressed his back against the cold wall, holding his breath."', why: 'The reader infers fear from details.' }] },
    vocab: [{ term: 'Exposition', def: 'The beginning of a story that introduces characters and setting.' }, { term: 'Rising action', def: 'Events that build tension toward the climax.' }, { term: 'Climax', def: 'The turning point or moment of highest tension.' }, { term: 'Resolution', def: 'How the conflict ends.' }, { term: 'Conflict', def: 'The central struggle in a story.' }, { term: 'Show, don\'t tell', def: 'Revealing feelings through actions and details instead of stating them.' }],
    questions: [
      ['The turning point of a story is the…', 'Climax', ['Exposition', 'Resolution', 'Rising action'], 'Highest tension.'],
      ['A hiker lost in a blizzard is which conflict?', 'Character vs. nature', ['Character vs. self', 'Character vs. society', 'Character vs. character'], 'The opponent is the weather.'],
      ['Which sentence SHOWS instead of TELLS?', 'She slammed the door and threw her bag across the room.', ['She was angry.', 'She felt mad.', 'She had anger.'], 'Actions reveal the feeling.'],
      ['Where are characters and setting introduced?', 'Exposition', ['Climax', 'Falling action', 'Resolution'], 'The beginning.'],
      ['A character struggling with a hard decision is…', 'Character vs. self', ['Character vs. nature', 'Character vs. society', 'Character vs. technology'], 'The conflict is internal.', 1],
    ],
  }),
  bankTopic({
    id: 'eng-argument', title: 'Argumentative Writing',
    lesson: `An **argument** tries to convince with reasons and evidence.
Structure: **claim** (your position) → **reasons** → **evidence** (facts, statistics, expert quotes, examples) → **counterclaim** (the other side) → **rebuttal** (why your side is still stronger) → conclusion.
Strong evidence is **relevant**, **credible** (trustworthy source), and **sufficient** (enough of it).
Avoid opinions presented as facts. Use formal language.`,
    example: { problem: 'Claim: "Schools should start later." Write a counterclaim and rebuttal.', steps: [{ step: 'Counterclaim: "Some say later start times make after-school activities end too late."', why: 'Fairly state the other side.' }, { step: 'Rebuttal: "However, well-rested students perform better, and activities can be adjusted."', why: 'Explain why your claim still wins.' }] },
    vocab: [{ term: 'Claim', def: 'The main position in an argument.' }, { term: 'Evidence', def: 'Facts, data, or examples that support a claim.' }, { term: 'Counterclaim', def: 'An opposing position.' }, { term: 'Rebuttal', def: 'A response showing why the counterclaim is weaker.' }, { term: 'Credible source', def: 'A trustworthy, expert, or reliable source.' }],
    questions: [
      ['Which is the most credible source for health facts?', 'A national health agency website', ['A random social media post', 'An ad for vitamins', 'A friend\'s opinion'], 'Experts with no product to sell.'],
      ['"However, this concern is outweighed by…" begins a…', 'Rebuttal', ['Claim', 'Hook', 'Counterclaim'], 'It answers the other side.'],
      ['Which is evidence rather than opinion?', '72% of students surveyed wanted longer lunch.', ['Lunch is the best part of the day.', 'Everyone hates short lunches.', 'I think lunch should be longer.'], 'Data can be checked.'],
      ['Why include a counterclaim?', 'It shows you considered other views, making your argument stronger', ['To confuse readers', 'To change your claim', 'To make it longer'], 'Addressing objections builds trust.'],
    ],
  }),
  bankTopic({
    id: 'eng-research', title: 'Research & Citing Sources',
    lesson: `Research steps: form a **research question** → find sources → take notes → **paraphrase** or quote → cite → write.
**Primary sources** are firsthand (diaries, photos, speeches); **secondary sources** interpret them (textbooks, articles).
**Paraphrase** = put ideas in your own words (still cite!). **Plagiarism** = using someone's words or ideas without credit.
Check websites: author? date? purpose? .gov/.edu are often reliable. A **works cited** page lists your sources (MLA style).`,
    example: { problem: 'Is a letter written by a Civil War soldier primary or secondary?', steps: [{ step: 'Was it created by someone who was there, at the time?', why: 'That defines primary.' }, { step: 'Yes → primary source.', why: 'A modern book about the war would be secondary.' }] },
    vocab: [{ term: 'Primary source', def: 'A firsthand record created at the time of an event.' }, { term: 'Secondary source', def: 'A source that interprets or analyzes primary sources.' }, { term: 'Paraphrase', def: 'Restating ideas in your own words.' }, { term: 'Plagiarism', def: 'Presenting someone else\'s work as your own.' }, { term: 'Works cited', def: 'The list of sources at the end of a paper.' }],
    questions: [
      ['Which is a primary source?', 'A diary from 1863', ['A 2020 history textbook', 'An encyclopedia article', 'A documentary review'], 'Created by an eyewitness.'],
      ['Copying a paragraph from a website without credit is…', 'Plagiarism', ['Paraphrasing', 'Citing', 'Summarizing'], 'Always give credit.'],
      ['If you paraphrase an idea, you…', 'Still need to cite the source', ['Don\'t need to cite it', 'Must use quotation marks', 'Change the facts'], 'The idea still came from someone else.'],
      ['Which question helps judge a website?', 'Who wrote it and when?', ['Is it colorful?', 'Does it have ads?', 'Is it long?'], 'Author and date affect credibility.'],
    ],
  }),
  bankTopic({
    id: 'eng-speech', title: 'Parts of Speech in Depth',
    lesson: `Beyond noun and verb:
**Pronouns** replace nouns (he, they, whom). **Prepositions** show relationships (in, under, between).
**Conjunctions**: coordinating (FANBOYS), subordinating (because, although, when).
**Interjections** show emotion (Wow!). **Verbals**: **gerund** (-ing as a noun: *Swimming is fun*), **participle** (verb as adjective: *the broken vase*), **infinitive** (to + verb: *I want to win*).
Verb **tense** should stay consistent within a paragraph.`,
    example: { problem: 'Identify the gerund: "Reading before bed helps me relax."', steps: [{ step: 'Find the -ing word: "Reading".', why: 'Gerunds end in -ing.' }, { step: 'It is the subject of the sentence → it acts as a noun → gerund.', why: 'A gerund is a verb form used as a noun.' }] },
    vocab: [{ term: 'Preposition', def: 'A word showing the relationship between a noun and another word (in, on, under).' }, { term: 'Gerund', def: 'An -ing verb form used as a noun.' }, { term: 'Infinitive', def: '"To" plus a verb.' }, { term: 'Participle', def: 'A verb form used as an adjective.' }, { term: 'Interjection', def: 'A word expressing strong emotion.' }, { term: 'Subordinating conjunction', def: 'A word like "because" that begins a dependent clause.' }],
    questions: [
      ['In "The cat hid under the bed," "under" is a…', 'Preposition', ['Adverb', 'Conjunction', 'Pronoun'], 'It shows where.'],
      ['In "I love to paint," "to paint" is a(n)…', 'Infinitive', ['Gerund', 'Participle', 'Preposition'], 'To + verb.'],
      ['In "the frozen lake," "frozen" is a…', 'Participle', ['Gerund', 'Infinitive', 'Noun'], 'A verb form describing a noun.'],
      ['Which is a subordinating conjunction?', 'Although', ['And', 'But', 'Or'], 'It starts a dependent clause.'],
      ['Which keeps tense consistent?', 'She ran to the bus and caught it.', ['She runs to the bus and caught it.', 'She ran to the bus and catches it.', 'She will run to the bus and caught it.'], 'Both verbs in past tense.', 1],
    ],
  }),
];
