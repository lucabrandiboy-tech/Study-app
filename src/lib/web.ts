// Open web search for the teacher (only when the picture filter is off with the teacher PIN). All keyless:
// Marginalia (independent web search engine, public API), DuckDuckGo instant answers, and r.jina.ai, which turns
// any web page into plain text so it can be read inside the app instead of opening the website.
import { stripHtml } from './wiki';

export interface WebHit { url: string; title: string; snippet: string; host: string }
export interface Instant { heading: string; text: string; source: string; image?: string; related: string[] }
export interface WebPage { title: string; url: string; paras: { kind: 'h' | 'p' | 'li'; text: string }[] }

const hostOf = (u: string) => { try { return new URL(u).hostname.replace(/^www\./, ''); } catch { return u; } };

type MG = { results?: { url: string; title?: string; description?: string }[] };
export function parseMarginalia(j: MG): WebHit[] {
  return (j.results ?? []).filter((r) => /^https?:\/\//.test(r.url)).map((r) => ({ url: r.url, title: stripHtml(r.title || hostOf(r.url)), snippet: stripHtml(r.description ?? ''), host: hostOf(r.url) }));
}
export async function webSearch(q: string, signal?: AbortSignal): Promise<WebHit[]> {
  const res = await fetch(`https://api.marginalia.nu/public/search/${encodeURIComponent(q)}?count=20`, { signal });
  if (!res.ok) throw new Error(`Web search answered ${res.status}`);
  return parseMarginalia(await res.json());
}

type DDG = { Heading?: string; AbstractText?: string; AbstractSource?: string; Image?: string; Answer?: string; Definition?: string; RelatedTopics?: { Text?: string }[] };
export function parseInstant(j: DDG): Instant | null {
  const text = j.AbstractText || j.Answer || j.Definition || '';
  if (!text) return null;
  return { heading: j.Heading || '', text: stripHtml(String(text)), source: j.AbstractSource || 'DuckDuckGo', image: j.Image ? (j.Image.startsWith('http') ? j.Image : `https://duckduckgo.com${j.Image}`) : undefined, related: (j.RelatedTopics ?? []).map((r) => r.Text ?? '').filter(Boolean).slice(0, 5) };
}
/** DuckDuckGo's instant answer. It has no CORS headers, so it's loaded as JSONP (a script tag with a callback). */
export function instantAnswer(q: string): Promise<Instant | null> {
  return new Promise((resolve) => {
    const cb = `__ddg${Date.now()}${Math.floor(Math.random() * 1e6)}`;
    const s = document.createElement('script');
    const done = (v: Instant | null) => { clearTimeout(t); delete (window as unknown as Record<string, unknown>)[cb]; s.remove(); resolve(v); };
    const t = setTimeout(() => done(null), 8000);
    (window as unknown as Record<string, unknown>)[cb] = (j: DDG) => done(parseInstant(j));
    s.onerror = () => done(null);
    s.src = `https://api.duckduckgo.com/?q=${encodeURIComponent(q)}&format=json&no_html=1&skip_disambig=1&callback=${cb}`;
    document.head.appendChild(s);
  });
}

/** Turns the reader's markdown into plain headings, paragraphs and list items; links become their text, pictures are dropped. */
export function parseReader(md: string, url: string): WebPage {
  const title = md.match(/^Title:\s*(.+)$/m)?.[1]?.trim() || hostOf(url);
  const body = md.includes('Markdown Content:') ? md.slice(md.indexOf('Markdown Content:') + 17) : md;
  const plain = (s: string) => s.replace(/!\[[^\]]*\]\([^)]*\)/g, '').replace(/\[([^\]]*)\]\([^)]*\)/g, '$1').replace(/<[^>]+>/g, '').replace(/[*_`]{1,3}([^*_`]+)[*_`]{1,3}/g, '$1').replace(/https?:\/\/\S+/g, '').replace(/\s+/g, ' ').trim();
  const paras: WebPage['paras'] = [];
  for (const raw of body.split('\n')) {
    const line = raw.trim();
    if (!line || /^[-=_*]{3,}$/.test(line)) continue;
    const h = line.match(/^#{1,6}\s+(.*)/);
    const li = line.match(/^(?:[-*+]|\d+\.)\s+(.*)/);
    const text = plain(h ? h[1] : li ? li[1] : line);
    if (text.length < 2) continue;
    paras.push({ kind: h ? 'h' : li ? 'li' : 'p', text });
  }
  return { title, url, paras: paras.slice(0, 400) };
}
/** Any web page as plain text, to read inside the app. */
export async function readPage(url: string, signal?: AbortSignal): Promise<WebPage> {
  const res = await fetch(`https://r.jina.ai/${url}`, { signal });
  if (!res.ok) throw new Error(`Couldn't load that page (${res.status}).`);
  return parseReader(await res.text(), url);
}
