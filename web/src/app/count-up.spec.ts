import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { CountUp, reducedMotion } from './count-up';

@Component({ imports: [CountUp], template: '<span [appCountUp]="n()"></span>' })
class Host {
  readonly n = signal(0);
}

describe('count-up and reduced motion', () => {
  const realMatch = window.matchMedia;
  afterEach(() => {
    window.matchMedia = realMatch;
  });

  function media(reduce: boolean) {
    window.matchMedia = ((q: string) => ({ matches: reduce && q.includes('reduce'), media: q })) as unknown as typeof window.matchMedia;
  }

  it('reads prefers-reduced-motion', () => {
    media(true);
    expect(reducedMotion()).toBe(true);
    media(false);
    expect(reducedMotion()).toBe(false);
  });

  it('writes the final number at once under reduced motion', async () => {
    media(true);
    await TestBed.configureTestingModule({ imports: [Host] }).compileComponents();
    const f = TestBed.createComponent(Host);
    f.componentInstance.n.set(42);
    f.detectChanges();
    await f.whenStable();
    expect((f.nativeElement as HTMLElement).textContent).toBe('42');
  });

  it('counts up through intermediate values when motion is allowed', async () => {
    media(false);
    const frames: FrameRequestCallback[] = [];
    vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => (frames.push(cb), frames.length));
    vi.stubGlobal('cancelAnimationFrame', () => undefined);
    try {
      await TestBed.configureTestingModule({ imports: [Host] }).compileComponents();
      const f = TestBed.createComponent(Host);
      f.detectChanges();
      f.componentInstance.n.set(100);
      TestBed.tick();
      f.detectChanges();
      await f.whenStable();
      const t0 = performance.now();
      for (const cb of frames.splice(0)) cb(t0 + 150);
      const mid = Number((f.nativeElement as HTMLElement).textContent);
      expect(mid).toBeGreaterThan(0);
      expect(mid).toBeLessThan(100);
      for (const cb of frames.splice(0)) cb(t0 + 1000);
      expect((f.nativeElement as HTMLElement).textContent).toBe('100');
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
