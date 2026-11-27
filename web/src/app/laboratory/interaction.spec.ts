import { Object3D, PerspectiveCamera, Vector3 } from 'three';
import { HANDLE_KEY, Handle, aperturePosition, probeUpdates, radialClaim, radialDistance, sourceHeight, sourcePosition, wallPosition } from './drag-map';
import { canGlow } from './glow';
import { AudioContextLike, LabAudio, MASTER, pitchFor, tremoloFor } from './lab-audio';
import { boxHalfWidth } from './apparatus';
import { handleOf, dragPlane } from './pointer';
import { scale } from './view-state';

describe('laboratory: dragging handles maps back to sliders', () => {
  it('inverts the scene scales, linear and log', () => {
    const lin = scale(400, 1000, -8, 8, false);
    expect(lin.from(lin.to(700))).toBeCloseTo(700, 9);
    const log = scale(1, 1e4, 0.3, 8);
    expect(log.from(log.to(250))).toBeCloseTo(250, 6);
  });
  it('turns the probe height into the claim and its width place into the first axis', () => {
    const ys = scale(0, 1, 0.3, 8, false);
    const xs = scale(400, 1000, -8, 8, false);
    const u = probeUpdates({ x: 0, y: ys.to(0.6), z: 2 }, ys, xs, 'Th');
    expect(u[0].id).toBe('claim');
    expect(u[0].value).toBeCloseTo(0.6, 9);
    expect(u[1]).toEqual({ id: 'Th', value: 700 });
    // Below the floor of a positive scale there is no claim to set.
    expect(probeUpdates({ x: 0, y: -5, z: 0 }, ys, null, null)).toEqual([]);
  });
  it('reads a radius as the claim over the limit', () => {
    expect(radialClaim(5, 2.5, 3)).toEqual([{ id: 'claim', value: 6 }]);
    expect(radialClaim(5, 2.5, null)).toEqual([]);
    expect(radialClaim(0, 2.5, 3)[0].value).toBeCloseTo(0.03, 12);
    expect(radialDistance({ x: 3, y: 9, z: 4 }, { x: 0, y: 0, z: 0 })).toBe(5);
  });
  it('inverts the bench drawings: walls, aperture, hot source', () => {
    for (const place of [0, 0.25, 1]) expect(wallPosition(boxHalfWidth(place))).toBeCloseTo(place, 12);
    expect(wallPosition(-boxHalfWidth(0.5))).toBeCloseTo(0.5, 12);
    expect(aperturePosition(0.6 + 1.8 * 0.4)).toBeCloseTo(0.4, 12);
    expect(sourcePosition(sourceHeight(0.7))).toBeCloseTo(0.7, 12);
    expect(sourcePosition(99)).toBe(1);
    expect(wallPosition(0)).toBe(0);
  });
  it('finds the handle on an object or its ancestors', () => {
    const parent = new Object3D();
    const child = new Object3D();
    parent.add(child);
    const h: Handle = { name: 'wall', plane: 'horizontal', map: () => [] };
    parent.userData[HANDLE_KEY] = h;
    expect(handleOf(child)).toBe(h);
    expect(handleOf(new Object3D())).toBeNull();
    expect(handleOf(null)).toBeNull();
  });
  it('drags in a horizontal plane at the handle height, or a vertical one facing the camera', () => {
    const camera = new PerspectiveCamera();
    camera.position.set(10, 5, 0);
    camera.lookAt(0, 0, 0);
    camera.updateMatrixWorld();
    const flat = dragPlane({ plane: 'horizontal' }, new Vector3(1, 2.5, 1), camera);
    expect(flat.normal.y).toBe(1);
    expect(flat.distanceToPoint(new Vector3(7, 2.5, -3))).toBeCloseTo(0, 12);
    const upright = dragPlane({ plane: 'vertical' }, new Vector3(0, 3, 0), camera);
    expect(upright.normal.y).toBeCloseTo(0, 12);
    expect(upright.distanceToPoint(new Vector3(0, 9, 4))).toBeCloseTo(0, 12);
  });
});

describe('laboratory: the glow falls back without float targets', () => {
  it('needs WebGL2 and a float colour buffer', () => {
    const gl = (ext: string | null) => ({ getExtension: (n: string) => (n === ext ? {} : null) });
    const r = (webgl2: boolean, ext: string | null) => ({ capabilities: { isWebGL2: webgl2 }, getContext: () => gl(ext) }) as never;
    expect(canGlow(r(true, 'EXT_color_buffer_float'))).toBe(true);
    expect(canGlow(r(true, 'EXT_color_buffer_half_float'))).toBe(true);
    expect(canGlow(r(true, null))).toBe(false);
    expect(canGlow(r(false, 'EXT_color_buffer_float'))).toBe(false);
  });
});

/** A fake AudioContext that records what the synthesizer does. */
function fakeContext() {
  const log = { oscillators: 0, started: 0, stopped: 0, closed: 0, targets: [] as number[] };
  const param = () => ({
    value: 0,
    setTargetAtTime: (v: number) => log.targets.push(v),
    setValueAtTime: () => undefined,
    exponentialRampToValueAtTime: () => undefined,
  });
  const node = () => ({ connect: () => undefined });
  const ctx = {
    currentTime: 0,
    destination: {},
    state: 'running',
    createGain: () => ({ ...node(), gain: param() }),
    createOscillator: () => {
      log.oscillators++;
      return { ...node(), type: 'sine', frequency: param(), start: () => log.started++, stop: () => log.stopped++ };
    },
    close: () => {
      log.closed++;
      ctx.state = 'closed';
      return Promise.resolve();
    },
  };
  return { ctx: ctx as unknown as AudioContextLike, log };
}

describe('laboratory: the sound', () => {
  it('maps the ratio to pitch and tremolo, within two octaves', () => {
    expect(pitchFor(1)).toBe(220);
    expect(pitchFor(2)).toBe(440);
    expect(pitchFor(100)).toBe(880);
    expect(pitchFor(null)).toBe(220);
    expect(tremoloFor(2)).toBeGreaterThan(tremoloFor(0.5));
    expect(MASTER).toBeLessThan(0.1);
  });
  it('is off until enabled, then closes its context when turned off', () => {
    const { ctx, log } = fakeContext();
    const audio = new LabAudio(() => ctx);
    expect(audio.on).toBe(false);
    audio.update(2, false);
    expect(log.oscillators).toBe(0);
    expect(audio.enable()).toBe(true);
    expect(audio.on).toBe(true);
    expect(log.started).toBe(2);
    audio.update(2, false);
    expect(log.targets).toContain(440);
    audio.disable();
    expect(audio.on).toBe(false);
    expect(log.closed).toBe(1);
    audio.dispose();
    expect(log.closed).toBe(1);
  });
  it('thuds once when the verdict turns IMPOSSIBLE, and pings on a breaking point', () => {
    const { ctx, log } = fakeContext();
    const audio = new LabAudio(() => ctx);
    audio.enable();
    const base = log.oscillators;
    audio.update(0.5, false);
    audio.update(3, true);
    expect(log.oscillators).toBe(base + 1);
    audio.update(3, true);
    expect(log.oscillators).toBe(base + 1);
    audio.pulse();
    expect(log.oscillators).toBe(base + 2);
    audio.dispose();
    expect(log.closed).toBe(1);
  });
  it('stays off when the browser has no Web Audio', () => {
    const audio = new LabAudio(() => {
      throw new Error('no audio');
    });
    expect(audio.enable()).toBe(false);
    expect(audio.on).toBe(false);
  });
});
