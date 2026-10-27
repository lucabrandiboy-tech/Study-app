// The reference panel: a box on the right of the browser version for looking things up while
// drawing. Pictures, videos and articles come from free, public sources that need no account
// or key (Wikipedia, Wikimedia Commons, Openverse, the Art Institute of Chicago, NASA, the
// Internet Archive); YouTube and other video links play inside it. Shown inside the app, never
// linked out.
//
// Private: searches are kept nowhere (no history, nothing in the address bar or local
// storage), the browser is asked not to cache the answers, the search box is outside any form
// (so the browser keeps no autofill list), and YouTube plays from its no-cookie address.

// ---------------------------------------------------------------- fetching
async function getJson(url, signal) {
  // Nothing kept in the browser's cache, and the sources are not told which page asked.
  const r = await fetch(url, { signal, cache: "no-store", referrerPolicy: "no-referrer", credentials: "omit" });
  if (!r.ok) throw new Error(`${new URL(url).host} answered ${r.status}`);
  return r.json();
}
const wmApi = (host, params) => `https://${host}/w/api.php?${new URLSearchParams({ ...params, format: "json", formatversion: "2", origin: "*" })}`;
const ENT = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: " " };
export const plain = html => String(html ?? "").replace(/<[^>]*>/g, "").replace(/&(#x?[0-9a-f]+|\w+);/gi, (m, e) => {
  if (e[0] === "#") { const c = e[1] === "x" || e[1] === "X" ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10); return Number.isFinite(c) ? String.fromCodePoint(c) : m; }
  return ENT[e.toLowerCase()] ?? m;
}).replace(/\s+/g, " ").trim();
const meta = (m, k) => plain(m?.[k]?.value);
/** Mixes lists so each source gets a turn: a1 b1 c1 a2 b2 c2… */
export const interleave = lists => { const out = []; for (let i = 0; lists.some(l => i < l.length); i++) for (const l of lists) if (i < l.length) out.push(l[i]); return out; };
const sorted = pages => (pages ?? []).slice().sort((a, b) => (a.index ?? 0) - (b.index ?? 0));

// ---------------------------------------------------------------- pictures
// Each picture: { thumb, title, credit, source, large?, details?(signal) -> {large, caption, credit} }
export function parseCommonsPictures(j) {
  return sorted(j.query?.pages).flatMap(p => {
    const info = p.imageinfo?.[0];
    if (!info?.thumburl) return [];
    const caption = meta(info.extmetadata, "ImageDescription") || meta(info.extmetadata, "ObjectName");
    const file = p.title.replace(/^File:/i, "");
    return [{ thumb: info.thumburl, title: caption || file.replace(/\.[a-z0-9]+$/i, ""), credit: [meta(info.extmetadata, "Artist"), meta(info.extmetadata, "LicenseShortName")].filter(Boolean).join(" · "), source: "Wikimedia Commons", file, host: "commons.wikimedia.org" }];
  });
}
export function parseLeadPictures(j) {
  return sorted(j.query?.pages).flatMap(p => {
    if (!p.thumbnail?.source || !p.pageimage) return [];
    return [{ thumb: p.thumbnail.source, title: p.title, credit: "", source: "Wikipedia", file: p.pageimage, host: "en.wikipedia.org" }];
  });
}
export function parseOpenverse(j) {
  return (j.results ?? []).flatMap(r => {
    const title = plain(r.title);
    if (!r.thumbnail) return [];
    const licence = r.license ? `CC ${r.license.toUpperCase()} ${r.license_version ?? ""}`.trim() : "";
    return [{ thumb: r.thumbnail, large: r.url, title: title || "Photo", credit: [plain(r.creator), licence].filter(Boolean).join(" · "), source: `Openverse${r.source ? ` (${r.source})` : ""}` }];
  });
}
export function parseArt(j) {
  const iiif = j.config?.iiif_url ?? "https://www.artic.edu/iiif/2";
  return (j.data ?? []).flatMap(a => {
    if (!a.image_id) return [];
    return [{ thumb: `${iiif}/${a.image_id}/full/400,/0/default.jpg`, large: `${iiif}/${a.image_id}/full/1686,/0/default.jpg`, title: [plain(a.title), plain(a.date_display)].filter(Boolean).join(", "), credit: plain(a.artist_display), source: "Art Institute of Chicago" }];
  });
}
export function parseNasa(j) {
  return (j.collection?.items ?? []).flatMap(it => {
    const d = it.data?.[0], thumb = it.links?.find(l => l.rel === "preview")?.href ?? it.links?.[0]?.href;
    if (!d?.nasa_id || !thumb) return [];
    return [{ thumb, large: thumb.replace("~thumb", "~medium"), title: plain(d.title), credit: "NASA · public domain", source: "NASA" }];
  });
}
async function searchPictures(q, signal) {
  const e = encodeURIComponent(q);
  const r = await Promise.allSettled([
    getJson(wmApi("en.wikipedia.org", { action: "query", generator: "search", gsrsearch: q, gsrlimit: "10", prop: "pageimages", piprop: "thumbnail|name", pithumbsize: "400" }), signal).then(parseLeadPictures),
    getJson(wmApi("commons.wikimedia.org", { action: "query", generator: "search", gsrsearch: `${q} filetype:bitmap`, gsrnamespace: "6", gsrlimit: "30", prop: "imageinfo", iiprop: "url|extmetadata", iiurlwidth: "400", iiextmetadatafilter: "ImageDescription|ObjectName|Artist|LicenseShortName", iiextmetadatalanguage: "en" }), signal).then(parseCommonsPictures),
    getJson(`https://api.openverse.org/v1/images/?q=${e}&page_size=24&mature=false`, signal).then(parseOpenverse),
    getJson(`https://api.artic.edu/api/v1/artworks/search?q=${e}&limit=15&fields=id,title,artist_display,date_display,image_id,medium_display`, signal).then(parseArt),
    getJson(`https://images-api.nasa.gov/search?q=${e}&media_type=image&page_size=10`, signal).then(parseNasa),
  ]);
  if (r.every(x => x.status === "rejected")) throw r[0].reason;
  return interleave(r.map(x => (x.status === "fulfilled" ? x.value : [])));
}
/** A bigger copy of a Wikimedia picture (the search only asks for small ones). */
async function wikimediaLarge(p, signal) {
  const j = await getJson(wmApi(p.host, { action: "query", titles: `File:${p.file}`, prop: "imageinfo", iiprop: "url", iiurlwidth: "1600" }), signal);
  const info = j.query?.pages?.[0]?.imageinfo?.[0];
  return info?.thumburl ?? info?.url ?? p.thumb;
}

// ---------------------------------------------------------------- videos
// Each video: { thumb, title, credit, source, embed (an address to show in a frame) }
export function parseCommonsVideos(j) {
  return sorted(j.query?.pages).flatMap(p => {
    const info = p.imageinfo?.[0];
    if (!info?.thumburl) return [];
    const caption = meta(info.extmetadata, "ImageDescription") || meta(info.extmetadata, "ObjectName");
    const file = p.title.replace(/^File:/i, "");
    return [{ thumb: info.thumburl, title: caption || file.replace(/\.[a-z0-9]+$/i, ""), credit: meta(info.extmetadata, "Artist"), source: "Wikimedia Commons",
      embed: `https://commons.wikimedia.org/wiki/File:${encodeURIComponent(file.replace(/ /g, "_"))}?embedplayer=yes` }];
  });
}
export function parseArchive(j) {
  return (j.response?.docs ?? []).flatMap(d => {
    const list = v => (Array.isArray(v) ? v.join(" ") : String(v ?? ""));
    const title = plain(list(d.title));
    if (!d.identifier || !title) return [];
    const id = encodeURIComponent(d.identifier);
    return [{ thumb: `https://archive.org/services/img/${id}`, title, credit: plain(list(d.creator)), source: "Internet Archive", embed: `https://archive.org/embed/${id}` }];
  });
}
async function searchVideos(q, signal) {
  const r = await Promise.allSettled([
    getJson(wmApi("commons.wikimedia.org", { action: "query", generator: "search", gsrsearch: `${q} filetype:video`, gsrnamespace: "6", gsrlimit: "20", prop: "imageinfo", iiprop: "url|extmetadata", iiurlwidth: "400", iiextmetadatafilter: "ImageDescription|ObjectName|Artist", iiextmetadatalanguage: "en" }), signal).then(parseCommonsVideos),
    getJson(`https://archive.org/advancedsearch.php?${new URLSearchParams({ q: `(${q}) AND mediatype:(movies)`, "fl[]": "identifier", rows: "24", output: "json" })}&fl[]=title&fl[]=creator`, signal).then(parseArchive),
  ]);
  if (r.every(x => x.status === "rejected")) throw r[0].reason;
  return interleave(r.map(x => (x.status === "fulfilled" ? x.value : [])));
}

// ---------------------------------------------------------------- articles
async function searchArticles(q, signal) {
  const j = await getJson(wmApi("en.wikipedia.org", { action: "query", list: "search", srsearch: q, srlimit: "12", srprop: "snippet" }), signal);
  return (j.query?.search ?? []).map(h => ({ title: h.title, snippet: plain(h.snippet) }));
}
const SKIP = /^(references|notes|sources|citations|footnotes|bibliography|external links|further reading|see also|gallery)$/i;
/** "== Heading ==" lines between paragraphs, as a list of { heading, paras }. */
export function sections(text) {
  const out = [{ heading: "", paras: [] }];
  let skipping = false;
  for (const raw of String(text).split("\n")) {
    const line = raw.trim();
    if (!line) continue;
    const h = line.match(/^={2,6}\s*(.*?)\s*=+$/);
    if (h) { skipping = SKIP.test(h[1]); if (!skipping) out.push({ heading: h[1], paras: [] }); continue; }
    if (!skipping && !line.includes("{\\displaystyle")) out[out.length - 1].paras.push(line);
  }
  return out.filter(s => s.paras.length);
}
async function article(title, signal) {
  const j = await getJson(wmApi("en.wikipedia.org", { action: "query", prop: "extracts|pageimages", titles: title, explaintext: "1", exsectionformat: "wiki", redirects: "1", piprop: "thumbnail", pithumbsize: "800" }), signal);
  const p = j.query?.pages?.[0];
  if (!p || p.missing || !p.extract) throw new Error("That article was not found.");
  return { title: p.title, image: p.thumbnail?.source, sections: sections(p.extract) };
}

// ---------------------------------------------------------------- links to watch
/** Where a pasted link plays: { frame } for a player page, { video } for a plain video file. */
export function playable(link) {
  const s = String(link).trim();
  const yt = s.match(/^[\w-]{11}$/) ? [0, s] : s.match(/(?:youtube(?:-nocookie)?\.com\/(?:watch\?(?:.*&)?v=|shorts\/|embed\/|live\/|v\/)|youtu\.be\/)([\w-]{11})/i);
  if (yt) {
    const t = s.match(/[?&#](?:t|start)=(?:(\d+)h)?(?:(\d+)m)?(\d+)s?\b/);
    const start = t ? (+(t[1] ?? 0)) * 3600 + (+(t[2] ?? 0)) * 60 + (+(t[3] ?? 0)) : 0;
    return { frame: `https://www.youtube-nocookie.com/embed/${yt[1]}?rel=0&playsinline=1&modestbranding=1&autoplay=1${start ? `&start=${start}` : ""}`, youtube: true };
  }
  const list = s.match(/youtube\.com\/playlist\?(?:.*&)?list=([\w-]+)/i);
  if (list) return { frame: `https://www.youtube-nocookie.com/embed/videoseries?list=${list[1]}&rel=0&modestbranding=1`, youtube: true };
  const vimeo = s.match(/vimeo\.com\/(?:video\/)?(\d+)/i);
  if (vimeo) return { frame: `https://player.vimeo.com/video/${vimeo[1]}?dnt=1&autoplay=1` };
  const ia = s.match(/archive\.org\/(?:details|embed)\/([^/?#\s]+)/i);
  if (ia) return { frame: `https://archive.org/embed/${ia[1]}` };
  const commons = s.match(/commons\.wikimedia\.org\/wiki\/(File:[^?#\s]+\.(?:webm|ogv|ogg|mpg|mpeg|mp4))/i);
  if (commons) return { frame: `https://commons.wikimedia.org/wiki/${commons[1]}?embedplayer=yes` };
  if (/^https:\/\/\S+\.(mp4|webm|ogv|mov)(\?\S*)?$/i.test(s)) return { video: s };
  return null;
}

// ---------------------------------------------------------------- the panel
const CSS = `
#reference { position: fixed; top: 0; right: 0; bottom: 0; width: var(--ref-w); display: none; flex-direction: column; background: #1c1c20; color: #ddd; border-left: 1px solid #3a3a42; font: 14px system-ui, sans-serif; z-index: 5; }
#reference.open { display: flex; }
#reference * { box-sizing: border-box; }
#reference .grip { position: absolute; left: -4px; top: 0; bottom: 0; width: 8px; cursor: ew-resize; touch-action: none; }
#reference .bar { display: flex; align-items: center; gap: 6px; padding: 8px 8px 6px 12px; }
#reference .bar b { font-weight: 600; font-size: 15px; margin-right: auto; }
#reference button { font: inherit; color: #ddd; background: #2c2c33; border: 1px solid #3c3c45; border-radius: 6px; padding: 5px 10px; cursor: pointer; }
#reference button:hover { background: #36363f; }
#reference button.on { background: #1d5d8c; border-color: #2a73a8; color: #fff; }
#reference .x { border: none; background: none; font-size: 20px; line-height: 1; padding: 2px 8px; color: #aaa; }
#reference .x:hover { color: #fff; background: #3a2a2a; }
#reference .tabs, #reference .kinds { display: flex; gap: 6px; padding: 0 12px 8px; }
#reference .row { display: flex; gap: 6px; padding: 0 12px 8px; }
#reference input { flex: 1; min-width: 0; font: inherit; color: #eee; background: #121215; border: 1px solid #3c3c45; border-radius: 6px; padding: 7px 10px; outline: none; }
#reference input:focus { border-color: #2a73a8; }
#reference .body { flex: 1; overflow: auto; padding: 0 12px 12px; }
#reference .note { color: #999; padding: 6px 0; line-height: 1.4; }
#reference .note.err { color: #e9a092; }
#reference .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(120px, 1fr)); gap: 8px; }
#reference .card { background: #26262c; border-radius: 8px; overflow: hidden; cursor: pointer; border: 1px solid transparent; }
#reference .card:hover { border-color: #2a73a8; }
#reference .card img { display: block; width: 100%; aspect-ratio: 1; object-fit: cover; background: #111; }
#reference .vid img { aspect-ratio: 16 / 9; }
#reference .card div { padding: 5px 7px 7px; font-size: 12px; color: #bbb; overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
#reference .hit { padding: 8px 2px; border-bottom: 1px solid #2c2c33; cursor: pointer; }
#reference .hit:hover b { color: #7cc0f0; }
#reference .hit p { margin: 4px 0 0; color: #aaa; font-size: 13px; line-height: 1.35; }
#reference .back { margin-bottom: 8px; }
#reference .view img { display: block; max-width: 100%; margin: 0 auto; border-radius: 6px; cursor: zoom-in; background: #111; }
#reference .view img.zoom { max-width: none; cursor: zoom-out; }
#reference .view .wide { overflow: auto; max-height: calc(100vh - 230px); border-radius: 6px; }
#reference .cap { margin: 8px 0 2px; line-height: 1.4; }
#reference .src { color: #888; font-size: 12px; }
#reference .player { position: relative; width: 100%; aspect-ratio: 16 / 9; background: #000; border-radius: 8px; overflow: hidden; }
#reference .player iframe, #reference .player video { position: absolute; inset: 0; width: 100%; height: 100%; border: 0; }
#reference article h2 { font-size: 18px; margin: 4px 0 8px; }
#reference article h3 { font-size: 15px; margin: 14px 0 4px; color: #9fd0f5; }
#reference article p { line-height: 1.5; margin: 0 0 8px; color: #ccc; }
#reference article img { max-width: 100%; border-radius: 6px; margin-bottom: 8px; }
`;

const el = (tag, props = {}, ...kids) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(props)) {
    if (k === "on") for (const [ev, f] of Object.entries(v)) e.addEventListener(ev, f);
    else if (k in e && k !== "list") e[k] = v;
    else e.setAttribute(k, v);
  }
  e.append(...kids.flat().filter(k => k != null));
  return e;
};

function create() {
  document.head.append(el("style", { textContent: CSS }));
  const canvas = document.getElementById("recedo");
  let width = Math.min(420, Math.round(innerWidth * 0.4));
  let kind = "pictures";
  let busy = null; // the search or page being loaded (to cancel it)
  let results = null; // the last list shown, to come back to

  const panel = el("aside", { id: "reference", "aria-label": "Reference" });
  const body = el("div", { className: "body" });
  const field = el("input", { type: "text", placeholder: "Search pictures, videos, articles", autocomplete: "off", spellcheck: false, "aria-label": "Search" });
  const link = el("input", { type: "text", placeholder: "Paste a YouTube (or other video) link", autocomplete: "off", spellcheck: false, "aria-label": "Video link" });
  const note = (text, err) => el("div", { className: err ? "note err" : "note", textContent: text });
  const show = (...nodes) => { body.replaceChildren(...nodes); body.scrollTop = 0; };
  const stopLoading = () => { busy?.abort(); busy = null; };

  // Two pages: Search and Watch.
  const searchPage = el("div", {}, el("div", { className: "row" }, field, el("button", { className: "on", textContent: "Search", on: { click: () => run() } })));
  const kinds = el("div", { className: "kinds" });
  for (const [k, label] of [["pictures", "Pictures"], ["videos", "Videos"], ["articles", "Articles"]]) {
    kinds.append(el("button", { textContent: label, className: k === kind ? "on" : "", on: { click: e => { kind = k; for (const b of kinds.children) b.className = b === e.target ? "on" : ""; if (field.value.trim()) run(); } } }));
  }
  searchPage.append(kinds);
  const watchPage = el("div", { hidden: true }, el("div", { className: "row" }, link, el("button", { className: "on", textContent: "Play", on: { click: () => play() } })));
  const tabs = el("div", { className: "tabs" });
  const tab = (name, page) => el("button", { textContent: name, on: { click: e => {
    for (const b of tabs.children) b.className = b === e.target ? "on" : "";
    searchPage.hidden = page !== searchPage; watchPage.hidden = page !== watchPage;
    stopLoading();
    if (page === watchPage) { show(note("Paste a link from YouTube, Vimeo, the Internet Archive or Wikimedia Commons, then Play. Nothing is saved.")); link.focus(); }
    else { results ? show(...results) : show(intro()); field.focus(); }
  } } });
  tabs.append(tab("Search", searchPage), tab("Watch a link", watchPage));
  tabs.firstChild.className = "on";
  const intro = () => note("Look up references while you draw: pictures and art from Wikipedia, Wikimedia Commons, Openverse, the Art Institute of Chicago and NASA; videos from Wikimedia Commons and the Internet Archive; Wikipedia articles. Searches are not saved.");

  panel.append(
    el("div", { className: "grip", title: "Drag to change the width" }),
    el("div", { className: "bar" }, el("b", { textContent: "Reference" }), el("button", { className: "x", textContent: "×", title: "Close", "aria-label": "Close", on: { click: () => setOpen(false) } })),
    tabs, searchPage, watchPage, body);
  document.body.append(panel);
  show(intro());

  field.addEventListener("keydown", e => { if (e.key === "Enter") run(); });
  link.addEventListener("keydown", e => { if (e.key === "Enter") play(); });
  link.addEventListener("paste", () => setTimeout(() => { if (playable(link.value)) play(); }, 0));
  // Keys typed here belong to the panel, not to the painting's shortcuts.
  for (const t of ["keydown", "keyup"]) panel.addEventListener(t, e => e.stopPropagation());
  // Back to the painting: a press on the canvas takes the keyboard back from the panel (or a
  // video player in it), so the painting's shortcuts work again.
  canvas.addEventListener("pointerdown", () => { if (document.activeElement !== canvas) canvas.focus({ preventScroll: true }); }, true);

  // Width: drag the left edge.
  const grip = panel.firstChild;
  grip.addEventListener("pointerdown", e => {
    grip.setPointerCapture(e.pointerId);
    const move = ev => { width = Math.max(280, Math.min(innerWidth * 0.7, innerWidth - ev.clientX)); layout(); };
    const up = () => { grip.removeEventListener("pointermove", move); grip.removeEventListener("pointerup", up); };
    grip.addEventListener("pointermove", move);
    grip.addEventListener("pointerup", up);
  });
  addEventListener("resize", () => layout());

  function layout() {
    const open = panel.classList.contains("open");
    const w = Math.round(Math.min(width, innerWidth * 0.7));
    panel.style.setProperty("--ref-w", `${w}px`);
    canvas.style.right = open ? `${w}px` : "0";
    canvas.style.width = open ? `calc(100% - ${w}px)` : "100%";
  }
  function setOpen(open) {
    panel.classList.toggle("open", open);
    if (!open) { stopLoading(); for (const m of body.querySelectorAll("iframe, video")) m.remove(); show(...(results ?? [intro()])); canvas.focus({ preventScroll: true }); }
    else if (!watchPage.hidden) link.focus(); else field.focus();
    layout();
  }

  async function run() {
    const q = field.value.trim();
    if (!q) return;
    stopLoading();
    const ctl = (busy = new AbortController());
    show(note("Searching…"));
    try {
      const nodes = kind === "pictures" ? pictureGrid(await searchPictures(q, ctl.signal))
        : kind === "videos" ? videoGrid(await searchVideos(q, ctl.signal))
        : articleList(await searchArticles(q, ctl.signal));
      if (ctl.signal.aborted) return;
      results = nodes;
      show(...nodes);
    } catch (e) {
      if (ctl.signal.aborted) return;
      results = [note(`Could not search: ${e.message}. Check the internet connection.`, true)];
      show(...results);
    }
  }

  const backButton = () => el("button", { className: "back", textContent: "← Back to results", on: { click: () => { stopLoading(); show(...(results ?? [intro()])); } } });
  function pictureGrid(list) {
    if (!list.length) return [note("No pictures found. Try other words.")];
    return [el("div", { className: "grid" }, list.map(p => el("div", { className: "card", title: p.title, on: { click: () => openPicture(p) } }, el("img", { src: p.thumb, alt: p.title, loading: "lazy", referrerPolicy: "no-referrer" }), el("div", { textContent: p.title }))))];
  }
  function openPicture(p) {
    const img = el("img", { src: p.large ?? p.thumb, alt: p.title, referrerPolicy: "no-referrer", title: "Click to see it at full size", on: { click: () => img.classList.toggle("zoom") } });
    show(backButton(), el("div", { className: "view" }, el("div", { className: "wide" }, img), el("div", { className: "cap", textContent: p.title }), el("div", { className: "src", textContent: [p.credit, p.source].filter(Boolean).join(" · ") })));
    if (p.file && !p.large) {
      const ctl = (busy = new AbortController());
      wikimediaLarge(p, ctl.signal).then(src => { if (!ctl.signal.aborted) img.src = src; }).catch(() => {});
    }
  }
  function videoGrid(list) {
    if (!list.length) return [note("No videos found. Try other words, or paste a YouTube link under \"Watch a link\".")];
    return [el("div", { className: "grid" }, list.map(v => el("div", { className: "card vid", title: v.title, on: { click: () => openVideo(v) } }, el("img", { src: v.thumb, alt: v.title, loading: "lazy", referrerPolicy: "no-referrer" }), el("div", { textContent: v.title }))))];
  }
  const frame = src => el("div", { className: "player" }, el("iframe", { src, title: "Video", allow: "autoplay; encrypted-media; picture-in-picture; fullscreen", allowFullscreen: true, referrerPolicy: "strict-origin-when-cross-origin", sandbox: "allow-scripts allow-same-origin allow-presentation" }));
  function openVideo(v) {
    show(backButton(), frame(v.embed), el("div", { className: "cap", textContent: v.title }), el("div", { className: "src", textContent: [v.credit, v.source].filter(Boolean).join(" · ") }));
  }
  function articleList(hits) {
    if (!hits.length) return [note("No articles found. Try other words.")];
    return hits.map(h => el("div", { className: "hit", on: { click: () => openArticle(h.title) } }, el("b", { textContent: h.title }), el("p", { textContent: h.snippet })));
  }
  async function openArticle(title) {
    stopLoading();
    const ctl = (busy = new AbortController());
    show(backButton(), note("Loading…"));
    try {
      const a = await article(title, ctl.signal);
      if (ctl.signal.aborted) return;
      show(backButton(), el("article", {}, el("h2", { textContent: a.title }), a.image ? el("img", { src: a.image, alt: "", referrerPolicy: "no-referrer" }) : null,
        a.sections.flatMap(s => [s.heading ? el("h3", { textContent: s.heading }) : null, ...s.paras.map(p => el("p", { textContent: p }))]),
        el("div", { className: "src", textContent: "From Wikipedia (CC BY-SA)" })));
    } catch (e) {
      if (!ctl.signal.aborted) show(backButton(), note(`Could not open it: ${e.message}`, true));
    }
  }

  function play() {
    const where = playable(link.value);
    if (!where) { show(note("That link can't be played here. Paste a link to a YouTube, Vimeo, Internet Archive or Wikimedia Commons video.", true)); return; }
    const player = where.frame ? frame(where.frame) : el("div", { className: "player" }, el("video", { src: where.video, controls: true, autoplay: true, playsInline: true }));
    show(player, note(where.youtube ? "Playing from YouTube's no-cookie player. The link isn't saved." : "The link isn't saved."));
    link.value = "";
  }

  layout();
  return { toggle: () => setOpen(!panel.classList.contains("open")), isOpen: () => panel.classList.contains("open"), open: () => setOpen(true), close: () => setOpen(false) };
}

if (typeof document !== "undefined" && document.getElementById("recedo")) window.recedoReference = create();
