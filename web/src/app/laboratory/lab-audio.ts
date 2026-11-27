/**
 * The sound of the laboratory (spec laboratory-3d, R-15): a small
 * synthesizer on the Web Audio API only, no file. Off by default; started
 * by the Sound button (a user gesture, as browsers require) and closed when
 * it is turned off or the laboratory closes. A quiet tone whose pitch and
 * tremolo follow the claimed over the limit; a low, short thud when the
 * engine's verdict becomes IMPOSSIBLE; a soft ping when Find Breaking Point
 * moves a slider. Sound only follows the engine; it never decides.
 */

/** The parts of an AudioContext the synthesizer uses (a fake one in the tests). */
export type AudioContextLike = Pick<AudioContext, 'currentTime' | 'destination' | 'state' | 'createOscillator' | 'createGain' | 'close'>;

/** Master level: quiet, under speech. */
export const MASTER = 0.06;

/** Pitch of the tone in Hz: 220 Hz at the limit, an octave per doubling of the ratio, two octaves either way at most. */
export function pitchFor(ratio: number | null): number {
  if (ratio === null || !Number.isFinite(ratio) || ratio <= 0) return 220;
  const octaves = Math.min(2, Math.max(-2, Math.log2(ratio)));
  return 220 * 2 ** octaves;
}

/** Tremolo rate in Hz: slow well within the limit, faster near it and past it. */
export function tremoloFor(ratio: number | null): number {
  if (ratio === null || !Number.isFinite(ratio) || ratio <= 0) return 1;
  return Math.min(9, Math.max(0.5, 0.5 + 4 * Math.min(2, ratio)));
}

export class LabAudio {
  private ctx: AudioContextLike | null = null;
  private master: GainNode | null = null;
  private tone: OscillatorNode | null = null;
  private lfo: OscillatorNode | null = null;
  private wasPast = false;

  constructor(private readonly create: () => AudioContextLike = () => new AudioContext()) {}

  get on(): boolean {
    return this.ctx !== null;
  }

  /** Starts the tone; call from a user gesture. False when the browser has no Web Audio. */
  enable(): boolean {
    if (this.ctx) return true;
    let ctx: AudioContextLike;
    try {
      ctx = this.create();
    } catch {
      return false;
    }
    const master = ctx.createGain();
    master.gain.value = MASTER;
    master.connect(ctx.destination);
    // Tremolo: a slow oscillator moves the level of the tone around a half.
    const level = ctx.createGain();
    level.gain.value = 0.5;
    const depth = ctx.createGain();
    depth.gain.value = 0.25;
    const lfo = ctx.createOscillator();
    lfo.frequency.value = tremoloFor(null);
    lfo.connect(depth);
    depth.connect(level.gain);
    const tone = ctx.createOscillator();
    tone.type = 'sine';
    tone.frequency.value = pitchFor(null);
    tone.connect(level);
    level.connect(master);
    tone.start();
    lfo.start();
    this.ctx = ctx;
    this.master = master;
    this.tone = tone;
    this.lfo = lfo;
    return true;
  }

  /** Follows the engine's last answer: pitch and tremolo from the ratio; a thud when the verdict turns IMPOSSIBLE. */
  update(ratio: number | null, impossible: boolean): void {
    if (!this.ctx || !this.tone || !this.lfo) {
      this.wasPast = impossible;
      return;
    }
    const t = this.ctx.currentTime;
    this.tone.frequency.setTargetAtTime(pitchFor(ratio), t, 0.08);
    this.lfo.frequency.setTargetAtTime(tremoloFor(ratio), t, 0.2);
    if (impossible && !this.wasPast) this.blip(70, 0.35, 0.9, 'triangle');
    this.wasPast = impossible;
  }

  /** The soft ping of Find Breaking Point. */
  pulse(): void {
    this.blip(660, 0.25, 0.35, 'sine');
  }

  /** A short note: fast attack, exponential release. */
  private blip(hz: number, seconds: number, peak: number, type: OscillatorType): void {
    if (!this.ctx || !this.master) return;
    const t = this.ctx.currentTime;
    const osc = this.ctx.createOscillator();
    osc.type = type;
    osc.frequency.setValueAtTime(hz, t);
    const env = this.ctx.createGain();
    env.gain.setValueAtTime(0.0001, t);
    env.gain.exponentialRampToValueAtTime(peak, t + 0.01);
    env.gain.exponentialRampToValueAtTime(0.0001, t + seconds);
    osc.connect(env);
    env.connect(this.master);
    osc.start(t);
    osc.stop(t + seconds + 0.02);
  }

  /** Stops every sound and closes the context. */
  disable(): void {
    const ctx = this.ctx;
    if (!ctx) return;
    try {
      this.tone?.stop();
      this.lfo?.stop();
    } catch {
      // Already stopped.
    }
    this.tone = null;
    this.lfo = null;
    this.master = null;
    this.ctx = null;
    if (ctx.state !== 'closed') void ctx.close().catch(() => undefined);
  }

  dispose(): void {
    this.disable();
  }
}
