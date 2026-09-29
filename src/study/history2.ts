import { bankTopic } from './bank';

export const HISTORY2 = [
  bankTopic({
    id: 'his-native', title: 'Native Americans & Early America',
    lesson: `Millions of people lived in the Americas long before Europeans arrived, in hundreds of nations with different cultures shaped by their environment.
• **Pueblo** peoples (Southwest): adobe homes, farming corn. • **Plains** nations (Lakota, Comanche): followed the **bison**.
• **Iroquois Confederacy** (Northeast): five (later six) nations united under the **Great Law of Peace** — an early model of shared government.
• **Pacific Northwest** (Kwakiutl): fishing, cedar plank houses, potlatches.
Great civilizations in Mesoamerica and South America: **Maya, Aztec, Inca**.`,
    example: { problem: 'Why did Plains nations live in tipis?', steps: [{ step: 'They moved often to follow bison herds.', why: 'Their food and materials came from the bison.' }, { step: 'Tipis were portable and made from bison hides.', why: 'Environment shaped their way of life.' }] },
    vocab: [{ term: 'Iroquois Confederacy', def: 'An alliance of Northeast nations governed by the Great Law of Peace.' }, { term: 'Adobe', def: 'Sun-dried brick made of clay and straw.' }, { term: 'Culture region', def: 'An area where groups share similar ways of life.' }, { term: 'Bison', def: 'The animal central to Plains life, providing food, clothing, and tools.' }, { term: 'Aztec', def: 'Empire in central Mexico with capital Tenochtitlan.' }],
    questions: [
      ['Which group united under the Great Law of Peace?', 'The Iroquois Confederacy', ['The Aztec', 'The Pueblo', 'The Inca'], 'Northeast woodlands.'],
      ['Plains nations depended most on…', 'Bison', ['Salmon', 'Corn only', 'Cattle'], 'Food, clothing, and shelter.'],
      ['Adobe homes were built by peoples in the…', 'Southwest', ['Northeast', 'Pacific Northwest', 'Arctic'], 'Dry climate, clay available.'],
      ['The Inca Empire was located in…', 'South America (Andes)', ['Central Mexico', 'The Great Plains', 'Canada'], 'Capital: Cusco.', 1],
    ],
  }),
  bankTopic({
    id: 'his-exploration', title: 'Age of Exploration',
    lesson: `In the 1400s–1500s Europeans explored to find **gold, God, and glory** — trade routes to Asia, wealth, and spreading Christianity.
**Columbus** (1492, for Spain) reached the Caribbean. **Magellan's** expedition first sailed around the world. Spain's **conquistadors** (Cortés, Pizarro) conquered the Aztec and Inca.
The **Columbian Exchange** swapped plants, animals, and diseases between hemispheres: horses and wheat came to the Americas; corn and potatoes went to Europe; diseases like **smallpox** killed millions of Native people.`,
    example: { problem: 'How did the Columbian Exchange change diets in Europe?', steps: [{ step: 'New crops arrived from the Americas: potatoes, corn, tomatoes.', why: 'Plants moved west to east.' }, { step: 'Potatoes grew well and fed more people, helping populations grow.', why: 'A cause-and-effect chain.' }] },
    vocab: [{ term: 'Columbian Exchange', def: 'The transfer of plants, animals, people, and diseases between the Americas and Europe/Africa.' }, { term: 'Conquistador', def: 'A Spanish conqueror in the Americas.' }, { term: 'Northwest Passage', def: 'A hoped-for water route through North America to Asia.' }, { term: 'Circumnavigate', def: 'To sail all the way around the world.' }],
    questions: [
      ['Which came FROM the Americas to Europe?', 'Potatoes', ['Horses', 'Wheat', 'Smallpox'], 'An American crop.'],
      ['Who conquered the Aztec Empire?', 'Hernán Cortés', ['Francisco Pizarro', 'Christopher Columbus', 'Ferdinand Magellan'], 'Pizarro conquered the Inca.'],
      ['The biggest cause of Native population loss was…', 'Disease', ['Famine only', 'Volcanoes', 'Migration'], 'Europeans brought diseases Native people had no immunity to.'],
      ['"Gold, God, and glory" were…', 'Motives for exploration', ['Names of ships', 'Colonies', 'Treaties'], 'Why Europeans explored.'],
    ],
  }),
  bankTopic({
    id: 'his-french-indian', title: 'French & Indian War',
    lesson: `The **French and Indian War** (1754–1763) was fought between Britain and France (and their Native allies) over the **Ohio River Valley**.
Britain won; the **Treaty of Paris (1763)** gave Britain Canada and land east of the Mississippi.
Results: Britain had huge war **debt**, so it began taxing the colonies. The **Proclamation of 1763** banned settling west of the Appalachians, angering colonists. These set the stage for the Revolution.
Young **George Washington** gained military experience in this war.`,
    example: { problem: 'How did winning the war lead to colonial anger?', steps: [{ step: 'Britain was in debt from the war.', why: 'Wars are expensive.' }, { step: 'Parliament taxed the colonies (Stamp Act) and blocked westward settlement.', why: 'Colonists felt they had no say — "no taxation without representation".' }] },
    vocab: [{ term: 'Proclamation of 1763', def: 'British order forbidding colonists to settle west of the Appalachian Mountains.' }, { term: 'Ohio River Valley', def: 'The region Britain and France fought over.' }, { term: 'Treaty of Paris (1763)', def: 'Ended the French and Indian War; France lost most of North America.' }, { term: 'Albany Plan of Union', def: 'Benjamin Franklin\'s 1754 plan to unite the colonies (rejected).' }],
    questions: [
      ['The French and Indian War was mainly over…', 'The Ohio River Valley', ['Florida', 'Tea taxes', 'Slavery'], 'Land west of the colonies.'],
      ['The Proclamation of 1763 angered colonists because it…', 'Banned settling west of the Appalachians', ['Raised taxes on tea', 'Ended trade with France', 'Freed enslaved people'], 'They wanted new land.'],
      ['Who won the French and Indian War?', 'Great Britain', ['France', 'Spain', 'The Iroquois'], 'France lost Canada.'],
      ['Why did Britain start taxing the colonies after 1763?', 'To pay war debts', ['To punish France', 'To fund exploration', 'To build railroads'], 'The war was costly.'],
    ],
  }),
  bankTopic({
    id: 'his-jackson', title: 'Age of Jackson & Reform',
    lesson: `**Andrew Jackson** (president 1829–1837) expanded democracy for common white men (more could vote), but signed the **Indian Removal Act** (1830), leading to the **Trail of Tears**.
He fought the **Bank of the United States** and faced the **Nullification Crisis** over states' rights.
The 1800s **reform movements**: **abolition** (ending slavery — Frederick Douglass, Harriet Tubman, William Lloyd Garrison), **women's rights** (Seneca Falls Convention 1848, Elizabeth Cady Stanton), **temperance**, and public **education** (Horace Mann).`,
    example: { problem: 'Why is Jackson called both "champion of the common man" and criticized?', steps: [{ step: 'More ordinary white men gained the vote and a voice.', why: 'Expansion of democracy.' }, { step: 'But he forced Native nations off their lands and supported slavery.', why: 'Democracy expanded for some, not all.' }] },
    vocab: [{ term: 'Abolition', def: 'The movement to end slavery.' }, { term: 'Seneca Falls Convention', def: '1848 meeting that launched the women\'s rights movement.' }, { term: 'Underground Railroad', def: 'Network of people and hiding places that helped enslaved people escape to freedom.' }, { term: 'Nullification', def: 'The idea that a state can reject a federal law.' }, { term: 'Temperance', def: 'The movement to limit or ban alcohol.' }],
    questions: [
      ['Harriet Tubman is famous for…', 'Leading people to freedom on the Underground Railroad', ['Writing the Constitution', 'Founding Jamestown', 'Fighting at Gettysburg'], 'She made about 13 trips.'],
      ['The Seneca Falls Convention (1848) focused on…', "Women's rights", ['Ending the Civil War', 'Buying Louisiana', 'Temperance only'], 'Declaration of Sentiments.'],
      ['Frederick Douglass was a…', 'Formerly enslaved abolitionist and writer', ['Confederate general', 'President', 'Explorer'], 'His autobiography was a bestseller.'],
      ['The Indian Removal Act was signed by…', 'Andrew Jackson', ['Thomas Jefferson', 'Abraham Lincoln', 'John Adams'], '1830.'],
    ],
  }),
  bankTopic({
    id: 'his-industry', title: 'Industrial Growth & Immigration',
    lesson: `The **Industrial Revolution** came to the U.S. in the early 1800s: **Samuel Slater's** textile mill, **Eli Whitney's cotton gin** (1793, which sadly increased slavery in the South), **interchangeable parts**, the **steamboat**, and **railroads**.
The North industrialized with factories; the South relied on cotton and enslaved labor — a growing divide.
Immigrants (Irish after the potato famine, Germans) filled factory and railroad jobs. The **transcontinental railroad** (1869) linked the coasts, built largely by Chinese and Irish workers.`,
    example: { problem: 'How did the cotton gin affect slavery?', steps: [{ step: 'It cleaned cotton much faster, making cotton very profitable.', why: 'More cotton could be processed.' }, { step: 'Planters wanted more land and more enslaved workers to grow it.', why: 'An invention had an unexpected social effect.' }] },
    vocab: [{ term: 'Cotton gin', def: 'Eli Whitney\'s machine that separates cotton seeds from fiber.' }, { term: 'Interchangeable parts', def: 'Identical parts that can replace each other, enabling mass production.' }, { term: 'Transcontinental railroad', def: 'Railroad completed in 1869 connecting the East and West coasts.' }, { term: 'Immigrant', def: 'A person who moves to a new country to live.' }, { term: 'Urbanization', def: 'The growth of cities.' }],
    questions: [
      ['Who invented the cotton gin?', 'Eli Whitney', ['Samuel Slater', 'Robert Fulton', 'Thomas Edison'], '1793.'],
      ['The transcontinental railroad was completed in…', '1869', ['1776', '1812', '1929'], 'Promontory Summit, Utah.'],
      ['Many Irish immigrants came in the 1840s because of…', 'The potato famine', ['The Gold Rush only', 'The Civil War', 'Religious freedom in Ireland'], 'Crops failed.'],
      ['Interchangeable parts made it possible to…', 'Mass-produce goods', ['End slavery', 'Build canals', 'Grow cotton'], 'Parts fit any copy of the product.', 1],
    ],
  }),
  bankTopic({
    id: 'his-road-war', title: 'The Road to Civil War',
    lesson: `As the U.S. grew, the question was: would new states be **free** or **slave**?
• **Missouri Compromise (1820)**: Missouri slave, Maine free; no slavery north of 36°30′.
• **Compromise of 1850**: California free; strict **Fugitive Slave Act**.
• **Uncle Tom's Cabin** (1852) turned many Northerners against slavery.
• **Kansas-Nebraska Act (1854)**: **popular sovereignty** (voters decide) → violence in "Bleeding Kansas".
• **Dred Scott decision (1857)**: Supreme Court said enslaved people were not citizens.
• **John Brown's raid** (1859), then **Lincoln's election** (1860) → Southern secession.`,
    example: { problem: 'Why did the Kansas-Nebraska Act cause violence?', steps: [{ step: 'It let settlers vote on slavery (popular sovereignty).', why: 'That overturned the Missouri Compromise line.' }, { step: 'Pro- and anti-slavery settlers rushed in and fought.', why: '"Bleeding Kansas".' }] },
    vocab: [{ term: 'Missouri Compromise', def: '1820 deal admitting Missouri as a slave state and Maine as free.' }, { term: 'Popular sovereignty', def: 'Letting the people of a territory vote on slavery.' }, { term: 'Fugitive Slave Act', def: '1850 law requiring escaped enslaved people to be returned.' }, { term: 'Dred Scott v. Sandford', def: '1857 ruling that enslaved people were not citizens.' }, { term: 'Sectionalism', def: 'Loyalty to one\'s region over the nation.' }],
    questions: [
      ['Which book stirred Northern anti-slavery feeling?', "Uncle Tom's Cabin", ['Common Sense', 'The Federalist', 'Moby-Dick'], 'Harriet Beecher Stowe, 1852.'],
      ['"Bleeding Kansas" followed the…', 'Kansas-Nebraska Act', ['Missouri Compromise', 'Louisiana Purchase', 'Emancipation Proclamation'], 'Popular sovereignty led to fighting.'],
      ['The Dred Scott decision ruled that…', 'Enslaved people were not citizens', ['Slavery was illegal', 'Kansas was free', 'Lincoln was president'], 'A Supreme Court case.'],
      ['Put in order: Missouri Compromise, Dred Scott, Lincoln elected', 'Missouri Compromise → Dred Scott → Lincoln elected', ['Dred Scott → Missouri Compromise → Lincoln elected', 'Lincoln elected → Dred Scott → Missouri Compromise', 'Missouri Compromise → Lincoln elected → Dred Scott'], '1820, 1857, 1860.', 1],
    ],
  }),
  bankTopic({
    id: 'his-courts', title: 'Landmark Supreme Court Cases',
    lesson: `The Supreme Court interprets the Constitution. Landmark cases:
• **Marbury v. Madison (1803)**: established **judicial review** — courts can strike down unconstitutional laws.
• **McCulloch v. Maryland (1819)**: federal law beats state law; Congress has implied powers.
• **Gibbons v. Ogden (1824)**: Congress controls interstate commerce.
• **Dred Scott v. Sandford (1857)**: denied citizenship to Black Americans (overturned by the 14th Amendment).
• **Plessy v. Ferguson (1896)**: allowed "separate but equal" segregation — later overturned by **Brown v. Board (1954)**.`,
    example: { problem: 'Why is Marbury v. Madison so important?', steps: [{ step: 'It gave the Supreme Court the power of judicial review.', why: 'Courts can declare laws unconstitutional.' }, { step: 'This makes the judicial branch a real check on Congress and the president.', why: 'Checks and balances.' }] },
    vocab: [{ term: 'Judicial review', def: 'The power of courts to declare laws unconstitutional.' }, { term: 'Precedent', def: 'An earlier ruling used to decide later cases.' }, { term: 'Implied powers', def: 'Powers not listed but needed to carry out listed powers.' }, { term: 'Supremacy Clause', def: 'The Constitution and federal laws are the highest law of the land.' }],
    questions: [
      ['Which case established judicial review?', 'Marbury v. Madison', ['McCulloch v. Maryland', 'Gibbons v. Ogden', 'Plessy v. Ferguson'], '1803, Chief Justice John Marshall.'],
      ['"Separate but equal" came from…', 'Plessy v. Ferguson', ['Brown v. Board', 'Marbury v. Madison', 'Dred Scott'], '1896.'],
      ['McCulloch v. Maryland ruled that…', 'Federal law is supreme over state law', ['States can ignore Congress', 'Slavery was legal', 'Schools must integrate'], 'Maryland could not tax the national bank.'],
      ['Gibbons v. Ogden was about…', 'Interstate commerce', ['Voting rights', 'Free speech', 'Slavery'], 'Steamboats between states.', 1],
    ],
  }),
];
