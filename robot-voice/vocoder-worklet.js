// Robot Voice's live voice changer, run on the browser's audio thread (an AudioWorklet).
// A channel vocoder: the mic's sound, split into bands, shapes a steady buzz at the robot's pitch, so the voice comes out
// flat and robotic. Before that the mic is levelled (a quiet headset mic and a loud one end up the same), and one smooth
// noise gate for the whole voice keeps room noise out without flickering.
// Output 0 is the robot, output 1 the levelled, gated mic (the page blends them with the Robot effect slider).
// AudioParams: pitch (Hz of the buzz).

const BANDS = (() => { const n = 22, lo = 120, hi = 6400, f = []; for (let i = 0; i < n; i++) f.push(lo * Math.pow(hi / lo, i / (n - 1))); return f; })();
const Q = 5;
const TARGET_DB = -28; // levelled speech loudness (RMS): the robot comes out about as loud as the typed lines
const MIN_OPEN_DB = -60; // anything quieter than this is never speech

const dbOf = (x) => 10 * Math.log10(x + 1e-12); // x is a mean square
const coef = (secs, rate) => 1 - Math.exp(-1 / (secs * rate));

/** Band-pass biquad (RBJ, 0 dB at the centre), transposed direct form II. */
function bandpass(f, q, rate) {
  const w = (2 * Math.PI * f) / rate, alpha = Math.sin(w) / (2 * q), a0 = 1 + alpha;
  return { b0: alpha / a0, b2: -alpha / a0, a1: (-2 * Math.cos(w)) / a0, a2: (1 - alpha) / a0, z1: 0, z2: 0 };
}
function highpass(f, rate) {
  const w = (2 * Math.PI * f) / rate, alpha = Math.sin(w) / (2 * Math.SQRT1_2), c = Math.cos(w), a0 = 1 + alpha;
  return { b0: (1 + c) / 2 / a0, b1: -(1 + c) / a0, b2: (1 + c) / 2 / a0, a1: (-2 * c) / a0, a2: (1 - alpha) / a0, z1: 0, z2: 0 };
}
function runBp(s, x) { const y = s.b0 * x + s.z1; s.z1 = -s.a1 * y + s.z2; s.z2 = s.b2 * x - s.a2 * y; return y; }
function runHp(s, x) { const y = s.b0 * x + s.z1; s.z1 = s.b1 * x - s.a1 * y + s.z2; s.z2 = s.b2 * x - s.a2 * y; return y; }

class RobotVocoder extends AudioWorkletProcessor {
  static get parameterDescriptors() { return [{ name: 'pitch', defaultValue: 86, minValue: 30, maxValue: 400, automationRate: 'k-rate' }]; }
  constructor() {
    super();
    const r = sampleRate;
    this.hp = highpass(90, r);
    this.ana = BANDS.map((f) => bandpass(f, Q, r));
    this.syn = BANDS.map((f) => bandpass(f, Q, r));
    this.env = new Float64Array(BANDS.length); // each band's loudness (mean square) in the mic
    this.car = new Float64Array(BANDS.length).fill(1e-4); // each band's loudness in the buzz, to even the buzz out
    this.hiss = BANDS.map((f) => (f > 3000 ? 0.45 : 0)); // top bands get some hiss for s, f and t sounds
    this.hfBand = BANDS.map((f) => f > 3500);
    this.lfBand = BANDS.map((f) => f < 1200);
    this.aAtt = coef(0.004, r); this.aRel = coef(0.022, r); this.aCar = coef(0.05, r);
    this.phase = 0; this.seed = 22222;
    this.floorDb = -70; this.speechDb = -30; this.gainDb = 6; this.heard = 0;
    this.open = false; this.quietBlocks = 0; this.gate = 0; this.uv = 0;
    this.gAtt = coef(0.003, r); this.gRel = coef(0.06, r);
  }
  process(inputs, outputs, params) {
    const input = inputs[0] && inputs[0][0], robot = outputs[0][0], dry = outputs[1] && outputs[1][0];
    if (!robot) return true;
    const n = robot.length, r = sampleRate;
    if (!input) { robot.fill(0); if (dry) dry.fill(0); return true; }

    // ---- block level: noise floor, gate, levelling (2.7 ms blocks) ----
    let ms = 0; for (let i = 0; i < n; i++) ms += input[i] * input[i];
    const db = dbOf(ms / n), blockSecs = n / r;
    // the noise floor drops quickly to quiet moments and creeps up slowly (3 dB a second) when it's loud
    if (db < this.floorDb) this.floorDb += (db - this.floorDb) * 0.2; else this.floorDb = Math.min(db, this.floorDb + 3 * blockSecs);
    const openAt = Math.max(this.floorDb + 12, MIN_OPEN_DB), closeAt = Math.max(this.floorDb + 6, MIN_OPEN_DB - 4);
    if (db > openAt) { this.open = true; this.quietBlocks = 0; }
    else if (this.open && db < closeAt && ++this.quietBlocks * blockSecs > 0.15) this.open = false; // 150 ms of quiet before closing
    if (this.open && db > openAt) { // learn how loud this person speaks, then level it to TARGET_DB
      this.heard += blockSecs;
      this.speechDb += (db - this.speechDb) * Math.min(1, blockSecs / (this.heard < 1 ? 0.15 : 0.6));
    }
    const wantDb = Math.max(-6, Math.min(30, TARGET_DB - this.speechDb));
    this.gainDb += (wantDb - this.gainDb) * Math.min(1, blockSecs / (wantDb < this.gainDb ? 0.05 : 0.3));
    const gain = Math.pow(10, this.gainDb / 20);

    // ---- voiceless sounds (s, f, sh, t): more hiss in the buzz when the top bands outweigh the low ones ----
    let hf = 0, lf = 0; for (let b = 0; b < BANDS.length; b++) { if (this.hfBand[b]) hf += this.env[b]; else if (this.lfBand[b]) lf += this.env[b]; }
    const uvTarget = Math.max(0, Math.min(1, (dbOf(hf) - dbOf(lf) + 6) / 12));
    this.uv += (uvTarget - this.uv) * Math.min(1, blockSecs / 0.01);

    const f0 = params.pitch[0], dt = f0 / r;
    const env = this.env, car = this.car, ana = this.ana, syn = this.syn, hiss = this.hiss;
    for (let i = 0; i < n; i++) {
      const x = runHp(this.hp, input[i]) * gain;
      // the buzz: a band-limited sawtooth (PolyBLEP) and white noise
      this.phase += dt; if (this.phase >= 1) this.phase -= 1;
      let saw = 2 * this.phase - 1;
      if (this.phase < dt) { const t = this.phase / dt; saw -= t + t - t * t - 1; } else if (this.phase > 1 - dt) { const t = (this.phase - 1) / dt; saw -= t * t + t + t + 1; }
      let s = this.seed; s ^= s << 13; s ^= s >>> 17; s ^= s << 5; this.seed = s; // xorshift32
      const noise = (s >>> 0) / 2147483648 - 1;
      let y = 0;
      for (let b = 0; b < env.length; b++) {
        const m = runBp(ana[b], x), m2 = m * m;
        env[b] += (m2 - env[b]) * (m2 > env[b] ? this.aAtt : this.aRel);
        const share = Math.min(0.9, hiss[b] + this.uv * (b > 9 ? 0.7 : 0.25));
        const c = runBp(syn[b], saw * (1 - share) + noise * share);
        car[b] += (c * c - car[b]) * this.aCar;
        y += c * Math.sqrt(env[b] / (car[b] + 1e-9));
      }
      this.gate += ((this.open ? 1 : 0) - this.gate) * (this.open ? this.gAtt : this.gRel);
      const g = this.gate;
      robot[i] = Math.tanh(y * g * 1.1);
      if (dry) dry[i] = Math.tanh(x * g);
    }
    return true;
  }
}
registerProcessor('robot-vocoder', RobotVocoder);
