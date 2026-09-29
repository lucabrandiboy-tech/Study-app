import { bankTopic } from './bank';
import type { Topic } from './types';

type Pair = [string, string];
type Set = { id: string; title: string; es: { lesson: string; words: Pair[] }; fr: { lesson: string; words: Pair[] } };

const SETS: Set[] = [
  { id: 'colors', title: 'Colors & Clothes',
    es: { lesson: `Colors: **rojo** (red), **azul** (blue), **verde** (green), **amarillo** (yellow), **negro** (black), **blanco** (white).
Clothes: **la camisa** (shirt), **los pantalones** (pants), **los zapatos** (shoes), **el vestido** (dress), **la chaqueta** (jacket).
Colors go AFTER the noun and match it: **la camisa roja**, **los zapatos negros**. "I'm wearing" = **Llevo…**`,
      words: [['rojo', 'red'], ['azul', 'blue'], ['verde', 'green'], ['amarillo', 'yellow'], ['negro', 'black'], ['blanco', 'white'], ['la camisa', 'shirt'], ['los zapatos', 'shoes'], ['la chaqueta', 'jacket'], ['los pantalones', 'pants']] },
    fr: { lesson: `Colors: **rouge** (red), **bleu** (blue), **vert** (green), **jaune** (yellow), **noir** (black), **blanc** (white).
Clothes: **la chemise** (shirt), **le pantalon** (pants), **les chaussures** (shoes), **la robe** (dress), **la veste** (jacket).
Most colors go AFTER the noun and agree: **une chemise verte**, **des chaussures noires**. "I'm wearing" = **Je porte…**`,
      words: [['rouge', 'red'], ['bleu', 'blue'], ['vert', 'green'], ['jaune', 'yellow'], ['noir', 'black'], ['blanc', 'white'], ['la chemise', 'shirt'], ['les chaussures', 'shoes'], ['la veste', 'jacket'], ['le pantalon', 'pants']] } },
  { id: 'weather', title: 'Weather & Seasons',
    es: { lesson: `¿Qué tiempo hace? (What's the weather like?) — **Hace sol** (sunny), **Hace calor** (hot), **Hace frío** (cold), **Hace viento** (windy), **Llueve** (it's raining), **Nieva** (it's snowing).
Seasons: **la primavera** (spring), **el verano** (summer), **el otoño** (fall), **el invierno** (winter).`,
      words: [['hace sol', "it's sunny"], ['hace calor', "it's hot"], ['hace frío', "it's cold"], ['llueve', "it's raining"], ['nieva', "it's snowing"], ['la primavera', 'spring'], ['el verano', 'summer'], ['el otoño', 'fall'], ['el invierno', 'winter']] },
    fr: { lesson: `Quel temps fait-il ? (What's the weather like?) — **Il fait beau** (nice), **Il fait chaud** (hot), **Il fait froid** (cold), **Il pleut** (it's raining), **Il neige** (it's snowing), **Il y a du vent** (windy).
Seasons: **le printemps** (spring), **l'été** (summer), **l'automne** (fall), **l'hiver** (winter). "In summer" = **en été**, but "in spring" = **au printemps**.`,
      words: [['il fait beau', "it's nice out"], ['il fait chaud', "it's hot"], ['il fait froid', "it's cold"], ['il pleut', "it's raining"], ['il neige', "it's snowing"], ['le printemps', 'spring'], ["l'été", 'summer'], ["l'automne", 'fall'], ["l'hiver", 'winter']] } },
  { id: 'body', title: 'Body & Health',
    es: { lesson: `Body: **la cabeza** (head), **la mano** (hand), **el pie** (foot), **los ojos** (eyes), **la boca** (mouth), **el brazo** (arm).
To say something hurts: **Me duele la cabeza** (my head hurts), **Me duelen los pies** (my feet hurt — plural!).
**Estoy enfermo/a** = I'm sick. **el médico** = doctor.`,
      words: [['la cabeza', 'head'], ['la mano', 'hand'], ['el pie', 'foot'], ['los ojos', 'eyes'], ['la boca', 'mouth'], ['el brazo', 'arm'], ['me duele', 'it hurts me'], ['enfermo', 'sick'], ['el médico', 'doctor']] },
    fr: { lesson: `Body: **la tête** (head), **la main** (hand), **le pied** (foot), **les yeux** (eyes), **la bouche** (mouth), **le bras** (arm).
To say something hurts: **J'ai mal à la tête** (I have a headache), **J'ai mal aux pieds** (my feet hurt).
**Je suis malade** = I'm sick. **le médecin** = doctor.`,
      words: [['la tête', 'head'], ['la main', 'hand'], ['le pied', 'foot'], ['les yeux', 'eyes'], ['la bouche', 'mouth'], ['le bras', 'arm'], ["j'ai mal", 'it hurts / I have pain'], ['malade', 'sick'], ['le médecin', 'doctor']] } },
  { id: 'home', title: 'House & Home',
    es: { lesson: `Rooms: **la cocina** (kitchen), **el baño** (bathroom), **el dormitorio** (bedroom), **la sala** (living room).
Things: **la cama** (bed), **la mesa** (table), **la silla** (chair), **la ventana** (window), **la puerta** (door).
**Hay** = there is/are: **Hay una cama en el dormitorio.**`,
      words: [['la cocina', 'kitchen'], ['el baño', 'bathroom'], ['el dormitorio', 'bedroom'], ['la sala', 'living room'], ['la cama', 'bed'], ['la mesa', 'table'], ['la silla', 'chair'], ['la ventana', 'window'], ['la puerta', 'door'], ['hay', 'there is / there are']] },
    fr: { lesson: `Rooms: **la cuisine** (kitchen), **la salle de bains** (bathroom), **la chambre** (bedroom), **le salon** (living room).
Things: **le lit** (bed), **la table** (table), **la chaise** (chair), **la fenêtre** (window), **la porte** (door).
**Il y a** = there is/are: **Il y a un lit dans la chambre.**`,
      words: [['la cuisine', 'kitchen'], ['la salle de bains', 'bathroom'], ['la chambre', 'bedroom'], ['le salon', 'living room'], ['le lit', 'bed'], ['la table', 'table'], ['la chaise', 'chair'], ['la fenêtre', 'window'], ['la porte', 'door'], ['il y a', 'there is / there are']] } },
  { id: 'town', title: 'Places & Directions',
    es: { lesson: `Places: **la biblioteca** (library), **el parque** (park), **el supermercado** (supermarket), **la tienda** (store), **el cine** (movie theater), **el hospital**.
Directions: **a la derecha** (to the right), **a la izquierda** (to the left), **todo recto** (straight ahead), **cerca de** (near), **lejos de** (far from).
**¿Dónde está…?** = Where is…? **Voy a la tienda** = I'm going to the store.`,
      words: [['la biblioteca', 'library'], ['el parque', 'park'], ['la tienda', 'store'], ['el cine', 'movie theater'], ['a la derecha', 'to the right'], ['a la izquierda', 'to the left'], ['todo recto', 'straight ahead'], ['cerca de', 'near'], ['lejos de', 'far from'], ['¿dónde está?', 'where is?']] },
    fr: { lesson: `Places: **la bibliothèque** (library), **le parc** (park), **le supermarché** (supermarket), **le magasin** (store), **le cinéma** (movie theater), **l'hôpital**.
Directions: **à droite** (to the right), **à gauche** (to the left), **tout droit** (straight ahead), **près de** (near), **loin de** (far from).
**Où est… ?** = Where is…? **Je vais au magasin** = I'm going to the store.`,
      words: [['la bibliothèque', 'library'], ['le parc', 'park'], ['le magasin', 'store'], ['le cinéma', 'movie theater'], ['à droite', 'to the right'], ['à gauche', 'to the left'], ['tout droit', 'straight ahead'], ['près de', 'near'], ['loin de', 'far from'], ['où est ?', 'where is?']] } },
  { id: 'verbs', title: 'Everyday Verbs (Present Tense)',
    es: { lesson: `Regular endings — **-ar** (hablar): o, as, a, amos, áis, an. **-er** (comer): o, es, e, emos, éis, en. **-ir** (vivir): o, es, e, imos, ís, en.
Irregular "yo" forms: **tengo** (I have), **hago** (I do/make), **voy** (I go), **soy/estoy** (I am), **salgo** (I leave).
**Me gusta + infinitive** = I like to…: *Me gusta nadar.*`,
      words: [['tengo', 'I have'], ['hago', 'I do / make'], ['voy', 'I go'], ['salgo', 'I leave'], ['como', 'I eat'], ['vivo', 'I live'], ['hablamos', 'we speak'], ['escriben', 'they write'], ['nadar', 'to swim'], ['leer', 'to read']] },
    fr: { lesson: `Regular endings — **-er** (parler): e, es, e, ons, ez, ent. **-ir** (finir): is, is, it, issons, issez, issent. **-re** (vendre): s, s, —, ons, ez, ent.
Key irregulars: **avoir** (j'ai), **être** (je suis), **aller** (je vais), **faire** (je fais), **prendre** (je prends).
**J'aime + infinitive** = I like to…: *J'aime nager.*`,
      words: [["j'ai", 'I have'], ['je fais', 'I do / make'], ['je vais', 'I go'], ['je prends', 'I take'], ['je mange', 'I eat'], ["j'habite", 'I live'], ['nous parlons', 'we speak'], ['ils écrivent', 'they write'], ['nager', 'to swim'], ['lire', 'to read']] } },
];

export function language2(lang: 'spanish' | 'french'): Topic[] {
  const L = lang === 'spanish' ? 'Spanish' : 'French';
  return SETS.map((s) => {
    const d = lang === 'spanish' ? s.es : s.fr;
    const [a, b] = d.words;
    return bankTopic({
      id: `lang-${lang}-${s.id}`, title: s.title, lesson: d.lesson,
      example: { problem: `How do you say "${b[1]}" in ${L}?`, steps: [{ step: `"${b[1]}" = "${b[0]}"`, why: 'Find it in the lesson list.' }, { step: `Practice it in a phrase with "${a[0]}" (${a[1]}).`, why: 'Using words together helps you remember them.' }] },
      vocab: d.words.map(([term, def]) => ({ term, def })),
      questions: [
        ...d.words.map(([w, m]) => [`What does "${w}" mean?`, m, d.words.filter((x) => x[1] !== m).map((x) => x[1]), `Picture using "${w}" in a sentence.`, 0] as [string, string, string[], string, number]),
        ...d.words.map(([w, m]) => [`How do you say "${m}" in ${L}?`, w, d.words.filter((x) => x[0] !== w).map((x) => x[0]), 'Say each option out loud and match it to the lesson.', 1] as [string, string, string[], string, number]),
      ],
    });
  });
}
