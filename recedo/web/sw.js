// Recedo's offline worker: keeps the app's files so the installed app opens without a
// connection. VERSION changes with every build, so an update replaces the old files.
const VERSION = "__VERSION__";
const FILES = ["./", "index.html", "recedo_web.js", "recedo_web_bg.wasm", "reference.js", "manifest.webmanifest", "icon-192.png", "icon-512.png"];
self.addEventListener("install", e => {
  e.waitUntil(caches.open(VERSION).then(c => c.addAll(FILES)).then(() => self.skipWaiting()));
});
self.addEventListener("activate", e => {
  e.waitUntil(caches.keys().then(keys => Promise.all(keys.filter(k => k !== VERSION).map(k => caches.delete(k)))).then(() => self.clients.claim()));
});
// Network first (so a new version arrives when online; asking the server even if the
// browser has a recent copy), the kept copy when offline.
self.addEventListener("fetch", e => {
  // Only the app's own files: searches and videos from other sites are never kept.
  if (e.request.method !== "GET" || new URL(e.request.url).origin !== location.origin) return;
  e.respondWith(fetch(e.request, { cache: "no-cache" }).then(r => {
    const copy = r.clone();
    caches.open(VERSION).then(c => c.put(e.request, copy));
    return r;
  }).catch(() => caches.match(e.request, { ignoreSearch: true })));
});
