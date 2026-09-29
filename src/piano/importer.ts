// Music importer: MusicXML (.xml/.musicxml), compressed MusicXML (.mxl) and MIDI (.mid/.midi).
// Imported songs are saved with your progress so they stay in your library.
import { parseMusicXML, parseMidiFile, type Piece } from './notation';
import type { ImportedSong } from '../lib/store';

/** Read the files inside a .zip (.mxl is a zip) using the browser's built-in decompressor. */
async function unzip(buf: ArrayBuffer): Promise<Map<string, Uint8Array>> {
  const v = new DataView(buf), u8 = new Uint8Array(buf), out = new Map<string, Uint8Array>();
  let eocd = -1;
  for (let i = buf.byteLength - 22; i >= Math.max(0, buf.byteLength - 65557); i--) if (v.getUint32(i, true) === 0x06054b50) { eocd = i; break; }
  if (eocd < 0) throw new Error('This .mxl file looks damaged (no zip directory found).');
  const count = v.getUint16(eocd + 10, true);
  let p = v.getUint32(eocd + 16, true);
  const dec = new TextDecoder();
  for (let k = 0; k < count; k++) {
    if (v.getUint32(p, true) !== 0x02014b50) break;
    const method = v.getUint16(p + 10, true), size = v.getUint32(p + 20, true);
    const nLen = v.getUint16(p + 28, true), eLen = v.getUint16(p + 30, true), cLen = v.getUint16(p + 32, true);
    const local = v.getUint32(p + 42, true);
    const name = dec.decode(u8.subarray(p + 46, p + 46 + nLen));
    const start = local + 30 + v.getUint16(local + 26, true) + v.getUint16(local + 28, true);
    const raw = u8.slice(start, start + size);
    if (method === 0) out.set(name, raw);
    else if (method === 8) {
      const stream = new Blob([raw]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
      out.set(name, new Uint8Array(await new Response(stream).arrayBuffer()));
    }
    p += 46 + nLen + eLen + cLen;
  }
  return out;
}

async function mxlToXml(buf: ArrayBuffer): Promise<string> {
  const files = await unzip(buf);
  const dec = new TextDecoder();
  const container = files.get('META-INF/container.xml');
  const root = container ? dec.decode(container).match(/full-path="([^"]+)"/)?.[1] : undefined;
  const name = root && files.has(root) ? root : [...files.keys()].find((n) => !n.startsWith('META-INF') && /\.(xml|musicxml)$/i.test(n));
  if (!name) throw new Error('No MusicXML score was found inside this .mxl file.');
  return dec.decode(files.get(name)!);
}

/** Guess a difficulty level from how busy and wide the music is. */
export function estimateLevel(p: Piece): ImportedSong['level'] {
  if (!p.events.length) return 'Beginner';
  const len = Math.max(...p.events.map((e) => e.beat + e.dur)) || 1;
  const density = p.events.length / len; // notes per beat
  const midis = p.events.map((e) => e.midi);
  const range = Math.max(...midis) - Math.min(...midis);
  const shortest = Math.min(...p.events.map((e) => e.dur));
  const score = density * 2 + range / 12 + (shortest < 0.5 ? 1.5 : shortest < 1 ? 0.5 : 0) + (p.bpm > 120 ? 1 : 0);
  return score < 4 ? 'Beginner' : score < 6.5 ? 'Intermediate' : score < 9 ? 'Pre-Advanced' : 'Advanced';
}

export async function importMusicFile(f: File): Promise<ImportedSong> {
  let piece: Piece;
  if (/\.midi?$/i.test(f.name)) piece = await parseMidiFile(await f.arrayBuffer(), f.name);
  else if (/\.mxl$/i.test(f.name)) piece = parseMusicXML(await mxlToXml(await f.arrayBuffer()), f.name);
  else if (/\.(xml|musicxml)$/i.test(f.name)) piece = parseMusicXML(await f.text(), f.name);
  else throw new Error('Pick a MusicXML (.musicxml, .xml, .mxl) or MIDI (.mid, .midi) file.');
  if (!piece.events.length) throw new Error('That file has no notes we can play.');
  const id = `imp-${f.name.replace(/\W+/g, '-').toLowerCase()}-${piece.events.length}`;
  const slim: Piece = { ...piece, id, xml: piece.xml && piece.xml.length < 400_000 ? piece.xml : undefined };
  return { id, title: piece.title, composer: piece.composer ?? 'Imported', level: estimateLevel(piece), added: new Date().toISOString().slice(0, 10), piece: slim };
}
