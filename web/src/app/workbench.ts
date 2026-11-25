import { Component, computed, inject, signal } from '@angular/core';
import { Observable, firstValueFrom } from 'rxjs';
import { AnalyzeResponse, AnalyzedClaim, PipelineInfo } from './analysis.models';
import { AnalysisService } from './analysis.service';
import { ApiFailure, LimitInfo, Verdict } from './api.models';
import { ApiService } from './api.service';
import { CarnotApparatus } from './carnot-apparatus';
import { formatValue } from './chart-geometry';
import { CopApparatus, TwinVerdicts } from './cop-apparatus';
import { Explorer } from './explorer';
import { STRINGS, WB } from './strings';
import {
  badge,
  explain,
  paragraphs,
  rememberDirection,
  shiftClaim,
  solveSentence,
  verdictLabel,
} from './workbench-logic';

/** The pre-filled text: synthetic claims about imaginary products. */
export const ILLUSTRATIVE_TEXT = [
  'The Volta cell stores 5 kWh of energy in a mass of 1 kg.',
  'The Aurora heat pump delivers heat with a COP of 14.5, hot reservoir at 300 K, cold reservoir at 280 K.',
  'With an unshrouded rotor, the Boreal wind turbine reaches a power coefficient of 0.70.',
  'The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.',
].join('\n\n');

/** Answers tried per paragraph when the model answer is unusable. */
export const ATTEMPTS = 3;

/** One tab: a claim, or a paragraph the pipeline could not analyse. */
export type Entry =
  | { kind: 'claim'; claim: AnalyzedClaim }
  | { kind: 'failure'; paragraph: number; text: string; failure: ApiFailure; attempts: number };

/** A line of the run log, shown above the trace while and after analysing. */
export interface LogLine {
  text: string;
  status: 'running' | 'done' | 'failed';
}

const VERDICT_CLASS: Record<Verdict, string> = {
  IMPOSSIBLE: 'verdict--impossible',
  CONDITIONAL: 'verdict--conditional',
  COMPATIBLE: 'verdict--compatible',
  INSUFFICIENT_DATA: 'verdict--insufficient-data',
};

/**
 * The analysis screen: the text, the pipeline trace of the selected claim
 * on the left, the claims on the right with the engine verdict, the plain
 * explanation, the law badge and the explorer.
 */
@Component({
  selector: 'app-workbench',
  imports: [Explorer, CarnotApparatus, CopApparatus],
  templateUrl: './workbench.html',
})
export class Workbench {
  private readonly analysis = inject(AnalysisService);
  private readonly api = inject(ApiService);
  protected readonly s = WB;

  protected readonly text = signal(ILLUSTRATIVE_TEXT);
  protected readonly running = signal(false);
  protected readonly log = signal<LogLine[]>([]);
  protected readonly entries = signal<Entry[]>([]);
  protected readonly selected = signal(0);
  protected readonly analysed = signal(false);
  protected readonly limits = signal<LimitInfo[]>([]);
  protected readonly info = signal<PipelineInfo | null>(null);
  protected readonly solved = signal<Record<number, string>>({});
  protected readonly twins = signal<Record<number, TwinVerdicts>>({});

  protected readonly entry = computed(() => this.entries()[this.selected()] ?? null);
  protected readonly claim = computed(() => {
    const e = this.entry();
    return e?.kind === 'claim' ? e.claim : null;
  });
  protected readonly limit = computed(() => {
    const id = this.claim()?.limit_id;
    return this.limits().find((l) => l.id === id) ?? null;
  });
  protected readonly explanation = computed(() => {
    const c = this.claim();
    return c ? explain(c) : [];
  });

  constructor() {
    this.loadLimits();
    this.analysis.pipeline().subscribe({ next: (i) => this.info.set(i), error: () => this.info.set(null) });
  }

  private loadLimits(): void {
    this.api.getLimits().subscribe({
      next: (r) => {
        for (const l of r.limits) rememberDirection(l.id, l.direction);
        this.limits.set(r.limits);
      },
      error: () => undefined,
    });
  }

  protected label(e: Entry): string {
    return e.kind === 'claim' ? verdictLabel(e.claim) : WB.paragraphFailed;
  }

  protected tabTitle(e: Entry): string {
    return e.kind === 'claim'
      ? `${e.claim.quantity}: ${formatValue(e.claim.raw_value)} ${e.claim.raw_unit}`
      : `${WB.progressParagraph} ${e.paragraph}`;
  }

  protected verdictText(c: AnalyzedClaim): string {
    return verdictLabel(c);
  }

  protected verdictClass(c: AnalyzedClaim): string {
    return c.result ? VERDICT_CLASS[c.result.verdict] : 'verdict--insufficient-data';
  }

  protected badgeOf(c: AnalyzedClaim): string {
    return badge(c);
  }

  /** The classification of the law of the claim, from GET /v1/limits; empty when unknown. */
  protected classOf(c: AnalyzedClaim): string {
    const id = this.limits().find((l) => l.id === c.limit_id)?.classification;
    return id ? (STRINGS.classification[id] ?? id) : '';
  }

  protected number(v: number | null | undefined): string {
    return v === null || v === undefined ? '-' : formatValue(v);
  }

  protected bound(c: AnalyzedClaim): string {
    const b = c.result?.bounds?.permissive;
    if (!b) return '-';
    return b.b_min === b.b_max ? formatValue(b.b_min) : `${formatValue(b.b_min)} to ${formatValue(b.b_max)}`;
  }

  protected paramText(c: AnalyzedClaim, id: string): string {
    const v = c.parameters[id]?.value;
    return v === null || v === undefined ? '' : String(v);
  }

  protected twinOf(i: number): TwinVerdicts {
    const t: TwinVerdicts | undefined = this.twins()[i];
    return t ?? { heating: null, cooling: null };
  }

  protected select(i: number): void {
    this.selected.set(i);
    void this.enrich(i);
  }

  protected onKey(event: KeyboardEvent, i: number): void {
    const n = this.entries().length;
    if (event.key === 'ArrowRight') this.select((i + 1) % n);
    else if (event.key === 'ArrowLeft') this.select((i - 1 + n) % n);
    else return;
    event.preventDefault();
  }

  private push(line: LogLine): number {
    this.log.update((l) => [...l, line]);
    return this.log().length - 1;
  }

  private settle(index: number, status: LogLine['status'], text?: string): void {
    this.log.update((l) => l.map((x, i) => (i === index ? { text: text ?? x.text, status } : x)));
  }

  /** Reads an image dropped or pasted on the input, through the vision model. */
  protected async takeImage(file: File | null | undefined, event: Event): Promise<void> {
    if (!file || !file.type.startsWith('image/')) return;
    event.preventDefault();
    if (!this.info()?.vision) {
      this.push({ text: WB.inputNoImage, status: 'failed' });
      return;
    }
    const base64 = await new Promise<string>((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result).replace(/^data:[^,]*,/, ''));
      reader.onerror = () => reject(reader.error);
      reader.readAsDataURL(file);
    });
    const line = this.push({ text: WB.imageRead, status: 'running' });
    try {
      const r = await firstValueFrom(this.analysis.transcribe(file.type, base64));
      this.text.set(r.text);
      this.settle(line, 'done', `${WB.imageRead} ${r.model}`);
    } catch (e) {
      const f = e as ApiFailure;
      this.settle(line, 'failed', `${WB.imageFailed}: ${f.code} ${f.message}`);
    }
  }

  protected onDrop(event: DragEvent): void {
    void this.takeImage(event.dataTransfer?.files?.[0], event);
  }

  protected onPaste(event: ClipboardEvent): void {
    const item = Array.from(event.clipboardData?.items ?? []).find((i) => i.type.startsWith('image/'));
    if (item) void this.takeImage(item.getAsFile(), event);
  }

  private async analyzeOnce(text: string): Promise<AnalyzeResponse> {
    return firstValueFrom(this.analysis.analyze(text) as Observable<AnalyzeResponse>);
  }

  /** Each paragraph in turn through POST /v1/analyze; unusable answers are asked again. */
  protected async run(): Promise<void> {
    if (this.running()) return;
    this.running.set(true);
    this.entries.set([]);
    this.solved.set({});
    this.twins.set({});
    this.selected.set(0);
    this.log.set(this.log().filter((l) => l.text.startsWith(WB.imageRead)));
    const text = this.text();
    const parts = paragraphs(text);
    for (const [k, p] of parts.entries()) {
      const head = `${WB.progressParagraph} ${k + 1} ${WB.progressOf} ${parts.length}`;
      const line = this.push({ text: `${head}: ${p.text}`, status: 'running' });
      let attempt = 0;
      for (;;) {
        attempt += 1;
        try {
          const r = await this.analyzeOnce(p.text);
          const claims = r.claims.map((c) => shiftClaim(c, p.start));
          this.entries.update((e) => [...e, ...claims.map((claim) => ({ kind: 'claim' as const, claim }))]);
          this.settle(line, 'done', `${head}: ${claims.length} claim(s)`);
          break;
        } catch (e) {
          const f = e as ApiFailure;
          if (f.code === 'ExtractionFailed' && attempt < ATTEMPTS) {
            this.settle(line, 'running', `${head}: ${WB.progressRetry} (${attempt})`);
            continue;
          }
          this.entries.update((x) => [
            ...x,
            { kind: 'failure', paragraph: k + 1, text: p.text, failure: f, attempts: attempt },
          ]);
          this.settle(line, 'failed', `${head}: ${f.code}: ${f.message}`);
          break;
        }
      }
    }
    this.running.set(false);
    this.analysed.set(true);
    this.loadLimits();
    if (this.entries().length) void this.enrich(0);
  }

  /** The solver sentence and, for COP without a mode, the verdict of each mode. */
  private async enrich(i: number): Promise<void> {
    const e = this.entries()[i];
    if (e?.kind !== 'claim') return;
    const c = e.claim;
    const verdict = c.result?.verdict;
    if (!c.limit_id || c.value === null || this.solved()[i] !== undefined) return;
    this.solved.update((m) => ({ ...m, [i]: '' }));
    const known = Object.entries(c.parameters).filter(([, p]) => p.value !== null);
    const constraints = Object.fromEntries(Object.entries(c.constraints).map(([a, o]) => [a, [o]]));
    if (verdict === 'IMPOSSIBLE' || verdict === 'CONDITIONAL') {
      for (const [free] of known) {
        const parameters = Object.fromEntries(
          known.filter(([id]) => id !== free).map(([id, p]) => [id, { value: p.value as number, unit: p.unit }]),
        );
        try {
          const r = await firstValueFrom(
            this.analysis.solve({ limit_id: c.limit_id, claim: { value: c.value, unit: c.unit }, constraints, parameters, free_parameter: free }),
          );
          this.solved.update((m) => ({ ...m, [i]: solveSentence(r.sentence) }));
          break;
        } catch {
          continue;
        }
      }
    }
    if (c.limit_id === 'cop' && !c.constraints['mode']) {
      const parameters = Object.fromEntries(known.map(([id, p]) => [id, { value: p.value as number, unit: p.unit }]));
      const twin: TwinVerdicts = { heating: null, cooling: null };
      for (const mode of ['heating', 'cooling'] as const) {
        try {
          const r = await firstValueFrom(
            this.api.evaluate({ limit_id: 'cop', claim: { value: c.value, unit: c.unit }, constraints: { mode: [mode] }, parameters }),
          );
          twin[mode] = r.verdict;
        } catch {
          twin[mode] = null;
        }
      }
      this.twins.update((m) => ({ ...m, [i]: twin }));
    }
  }
}
