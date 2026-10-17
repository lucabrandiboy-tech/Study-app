// Robot Voice engine: eSpeak (via meSpeak.js, GPL) turning text into a WAV file.
// Runs as a Web Worker (the page posts { id, text, pitch, speed, wordgap } and gets { id, wav } back),
// or on the page itself as window.RobotVoiceEngine if a worker can't start.
var warn = console.warn;
if (typeof window === 'undefined') console.warn = function () {}; // in a worker meSpeak warns that it can only make raw audio, which is all we want
var meSpeak = require('mespeak/src/index.js');
console.warn = warn;
meSpeak.loadConfig(require('mespeak/src/mespeak_config.json'));
var us = require('mespeak/voices/en/en-us.json');
meSpeak.loadVoice(us);
// The robot voice is US English with a flat pitch (no rise and fall) and no natural wobble.
meSpeak.loadVoice({ voice_id: 'robot', voice: atob(us.voice).replace('name english-us', 'name robot') + '\npitch 95 95\nflutter 0\n', voice_encoding: 'text' });

function speak(o) {
  var wav = meSpeak.speak(String(o.text), { voice: 'robot', pitch: o.pitch, speed: o.speed, wordgap: o.wordgap || 0, rawdata: 'arraybuffer' });
  if (!wav) throw new Error('The voice engine made no sound.');
  return wav;
}

if (typeof window === 'undefined') {
  self.onmessage = function (e) {
    var d = e.data;
    try { var wav = speak(d); self.postMessage({ id: d.id, wav: wav }, [wav]); }
    catch (err) { self.postMessage({ id: d.id, error: String((err && err.message) || err) }); }
  };
  self.postMessage({ ready: true });
} else {
  window.RobotVoiceEngine = { speak: speak };
}
