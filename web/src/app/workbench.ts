import { A11yModule } from '@angular/cdk/a11y';
import { Component, ElementRef, HostListener, computed, inject, signal, viewChild } from '@angular/core';
import { LucideAngularModule, ChevronDown, ChevronLeft, ChevronRight, Copy, Download, ImageUp, X } from 'lucide-angular';
import { Observable, firstValueFrom } from 'rxjs';
import { AnalyzeResponse, AnalyzedClaim } from './analysis.models';
import { AnalysisService } from './analysis.service';
import { ApiFailure, EvaluateResponse, LimitInfo } from './api.models';
import { ApiService } from './api.service';
import { CarnotApparatus } from './carnot-apparatus';
import { CopApparatus, TwinVerdicts } from './cop-apparatus';
import { CountUp } from './count-up';
import { Explorer } from './explorer';
import { Gauge } from './gauge';
import {
  Entry,
  Segment,
  VerdictKind,
  boundText,
  claimValueText,
  entryTitle,
  filtered,
  plainSentence,
  ratioText,
  readValueText,
  reviewMarkdown,
  segments,
  step,
  summary,
  verdictKind,
  verdictText,
} from './review-logic';
import { SAMPLES, Sample, evaluateRequest, sampleClaims, sampleOf } from './samples';
import { StatusService } from './status.service';
import { EXPLAIN, STRINGS, UI, WB } from './strings';
import { ToastService } from './toast.service';
import { Paragraph, badge, explain, paragraphs, rememberDirection, shiftClaim, solveSentence } from './workbench-logic';

export { ILLUSTRATIVE_TEXT } from './samples';
export type { Entry } from './review-logic';

/** Answers tried per paragraph when the model answer is unusable. */
export const ATTEMPTS = 3;

/** A line of the run log, shown in the trace drawer. */
export interface LogLine {
  text: string;
  status: 'running' | 'done' | 'failed';
}

/** A paragraph of the reading panel, cut at the evidence of its claims. */
interface ReadingParagraph {
  segments: Segment[];
}

/**
 * The analysis screen. Input: a hero with the text card and samples.
 * Running: a compact bar, a stepper and claim cards streaming in.
 * Reviewed: a summary strip, a claim list, the text with each claim
 * highlighted and the instrument card of the selected claim. The verdicts
 * and numbers come from the API; this component only lays them out.
 */
@Component({
  selector: 'app-workbench',
  imports: [Explorer, Gauge, CarnotApparatus, CopApparatus, CountUp, LucideAngularModule, A11yModule],
  templateUrl: './workbench.html',
  host: { '[attr.data-state]': 'state()' },
})
export class Workbench {
  private readonly analysis = inject(AnalysisService);
  private readonly api = inject(ApiService);
  private readonly toast = inject(ToastService);
  protected readonly status = inject(StatusService);
  protected readonly s = WB;
  protected readonly u = UI;
  protected readonly samples = SAMPLES;
  protected readonly icons = { ChevronDown, ChevronLeft, ChevronRight, Copy, Download, ImageUp, X };

  readonly text = signal(SAMPLES[0].text);
  readonly running = signal(false);
  readonly analysed = signal(false);
  readonly entries = signal<Entry[]>([]);
  readonly selected = signal(0);
  readonly howOpen = signal(false);
  protected readonly log = signal<LogLine[]>([]);
  protected readonly parts = signal<Paragraph[]>([]);
  protected readonly answered = signal(0);
  /** Active step of the stepper: 0 Reading, 1 Identifying laws, 2 Researching sources, 3 Computing, 4 done. */
  protected readonly stage = signal(0);
  protected readonly filter = signal<VerdictKind | null>(null);
  protected readonly sourceOpen = signal(false);
  protected readonly analysedAt = signal('');
  protected readonly limits = signal<LimitInfo[]>([]);
  protected readonly solved = signal<Record<number, string>>({});
  protected readonly twins = signal<Record<number, TwinVerdicts>>({});
  private readonly howDrawer = viewChild<ElementRef<HTMLElement>>('how');

  readonly state = computed<'input' | 'running' | 'reviewed'>(() =>
    this.running() ? 'running' : this.analysed() ? 'reviewed' : 'input',
  );
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
    return c ? explain(c).filter((l) => l === EXPLAIN.aiDraft || l === EXPLAIN.disputed) : [];
  });
  protected readonly board = computed(() => summary(this.entries(), this.parts().length));
  protected readonly visible = computed(() => filtered(this.entries(), this.filter()));
  protected readonly reading = computed<ReadingParagraph[]>(() => {
    const claims = this.entries().flatMap((e, index) => (e.kind === 'claim' ? [{ index, claim: e.claim }] : []));
    return this.parts().map((p) => ({
      segments: segments(p.text, claims.map((c) => ({ ...c, claim: shiftClaim(c.claim, -p.start) }))),
    }));
  });
  /** What the instrument draws: the gauge (a Max law with no parameter), the explorer, or nothing. */
  protected readonly visual = computed<'gauge' | 'explorer' | 'none'>(() => {
    const c = this.claim();
    const l = this.limit();
    if (!c || !l || !c.limit_id) return 'none';
    if (l.parameters.length === 0 && l.direction === 'Max' && c.value !== null && c.result?.bounds) return 'gauge';
    return 'explorer';
  });
  protected readonly gaugeBounds = computed(() => {
    const b = this.claim()?.result?.bounds;
    return b ? Array.from(new Set([b.permissive.b_max, b.restrictive.b_max])) : [];
  });
  protected readonly charCount = computed(() => Array.from(this.text()).length);

  constructor() {
    this.loadLimits();
  }

  private loadLimits(): Promise<void> {
    return new Promise((resolve) => {
      this.api.getLimits().subscribe({
        next: (r) => {
          for (const l of r.limits) rememberDirection(l.id, l.direction === 'Min' ? 'Min' : 'Max');
          this.limits.set(r.limits);
          resolve();
        },
        error: () => resolve(),
      });
    });
  }

  private direction(e: Entry): 'Max' | 'Min' | 'Equal' {
    if (e.kind !== 'claim') return 'Max';
    return this.limits().find((l) => l.id === e.claim.limit_id)?.direction ?? 'Max';
  }

  protected kindOf(i: number): VerdictKind {
    const e = this.entries()[i];
    return e ? verdictKind(e) : 'needs';
  }
  protected labelOf(i: number): string {
    const e = this.entries()[i];
    return e ? verdictText(e) : '';
  }
  protected sentenceOf(i: number): string {
    const e = this.entries()[i];
    return e ? plainSentence(e, this.direction(e)) : '';
  }
  protected titleOf(e: Entry): string {
    return entryTitle(e);
  }
  protected quantityOf(e: Entry): string {
    return e.kind === 'claim' ? e.claim.quantity : `${WB.progressParagraph} ${e.paragraph}`;
  }
  protected valueOf(e: Entry): string {
    return e.kind === 'claim' ? readValueText(e.claim) : UI.notAnalysed;
  }
  protected isVisible(i: number): boolean {
    return this.visible().includes(i);
  }
  protected claimText(c: AnalyzedClaim): string {
    return claimValueText(c);
  }
  protected bound(c: AnalyzedClaim): string {
    return boundText(c);
  }
  protected ratio(c: AnalyzedClaim): string {
    return ratioText(c) ?? UI.metricNone;
  }
  protected badgeOf(c: AnalyzedClaim): string {
    return badge(c);
  }
  protected classOf(c: AnalyzedClaim): string {
    const id = this.limits().find((l) => l.id === c.limit_id)?.classification;
    return id ? (STRINGS.classification[id] ?? id) : '';
  }
  protected number(v: number | null | undefined): string {
    return v === null || v === undefined ? '' : String(v);
  }
  protected paramText(c: AnalyzedClaim, id: string): string {
    const v = c.parameters[id]?.value;
    return v === null || v === undefined ? '' : String(v);
  }
  protected twinOf(i: number): TwinVerdicts {
    const t: TwinVerdicts | undefined = this.twins()[i];
    return t ?? { heating: null, cooling: null };
  }
  protected stepState(i: number): 'done' | 'active' | 'waiting' {
    const s = this.stage();
    return i < s ? 'done' : i === s ? 'active' : 'waiting';
  }
  protected pending(): number[] {
    const n = Math.min(3, Math.max(0, this.parts().length - this.answered()));
    return Array.from({ length: n }, (_, i) => i);
  }

  select(i: number): void {
    if (i < 0 || i >= this.entries().length) return;
    this.selected.set(i);
    void this.enrich(i);
  }

  protected move(by: 1 | -1): void {
    this.select(step(this.visible(), this.selected(), by));
  }

  protected setFilter(kind: VerdictKind | null): void {
    this.filter.set(this.filter() === kind ? null : kind);
    const v = this.visible();
    if (v.length && !v.includes(this.selected())) this.select(v[0]);
  }

  protected useSample(s: Sample): void {
    this.text.set(s.text);
  }

  toggleHow(): void {
    if (this.howOpen()) this.closeHow();
    else this.openHow();
  }
  protected openHow(): void {
    this.howOpen.set(true);
    setTimeout(() => this.howDrawer()?.nativeElement.focus());
  }
  protected closeHow(): void {
    this.howOpen.set(false);
  }

  newAnalysis(): void {
    this.analysed.set(false);
    this.howOpen.set(false);
    this.filter.set(null);
  }

  @HostListener('document:keydown.escape')
  protected onEscape(): void {
    this.closeHow();
  }

  /** J / K and the arrow keys move between claims, outside form controls. */
  @HostListener('document:keydown', ['$event'])
  protected onKey(event: KeyboardEvent): void {
    if (this.state() !== 'reviewed' || this.howOpen() || event.ctrlKey || event.metaKey || event.altKey) return;
    const t = event.target as HTMLElement | null;
    if (t && (/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) || t.isContentEditable)) return;
    if (t instanceof Element && t.closest('.cdk-overlay-container')) return;
    const k = event.key;
    if (k === 'j' || k === 'J' || k === 'ArrowDown' || k === 'ArrowRight') this.move(1);
    else if (k === 'k' || k === 'K' || k === 'ArrowUp' || k === 'ArrowLeft') this.move(-1);
    else return;
    event.preventDefault();
  }

  /** The review as Markdown, built and downloaded in the browser. */
  download(): void {
    const md = reviewMarkdown(this.entries(), this.parts().length, this.limits(), this.analysedAt());
    const url = URL.createObjectURL(new Blob([md], { type: 'text/markdown;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = UI.reportFile;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url));
    this.toast.show(UI.downloaded);
  }

  protected copyQuote(text: string): void {
    void navigator.clipboard?.writeText(text).then(
      () => this.toast.show(UI.copied),
      () => undefined,
    );
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
    if (!this.status.vision()) {
      this.toast.show(WB.inputNoImage, 'error');
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
      this.toast.show(`${UI.toastImage}: ${f.code}`, 'error');
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

  private paragraphOf(start: number): number {
    const parts = this.parts();
    let k = 0;
    parts.forEach((p, i) => {
      if (p.start <= start) k = i;
    });
    return k + 1;
  }

  /** Runs the pipeline: each paragraph through POST /v1/analyze, or a built-in sample through the engine. */
  async run(): Promise<void> {
    if (this.running() || !this.text().trim()) return;
    this.running.set(true);
    this.entries.set([]);
    this.solved.set({});
    this.twins.set({});
    this.selected.set(0);
    this.filter.set(null);
    this.sourceOpen.set(false);
    this.answered.set(0);
    this.stage.set(0);
    this.log.set(this.log().filter((l) => l.text.startsWith(WB.imageRead)));
    const text = this.text();
    this.parts.set(paragraphs(text));
    const sample = sampleOf(text);
    if (sample && !this.status.extraction()) await this.runSample(sample);
    else await this.runModel();
    this.stage.set(1);
    await Promise.resolve();
    this.stage.set(2);
    await this.loadLimits();
    this.stage.set(3);
    if (this.entries().length) await this.enrich(0);
    this.stage.set(4);
    this.analysedAt.set(new Date().toISOString().slice(0, 16).replace('T', ' ') + ' UTC');
    this.running.set(false);
    this.analysed.set(true);
  }

  private async runModel(): Promise<void> {
    const parts = this.parts();
    for (const [k, p] of parts.entries()) {
      const head = `${WB.progressParagraph} ${k + 1} ${WB.progressOf} ${parts.length}`;
      const line = this.push({ text: `${head}: ${p.text}`, status: 'running' });
      let attempt = 0;
      for (;;) {
        attempt += 1;
        try {
          const r = await this.analyzeOnce(p.text);
          const claims = r.claims.map((c) => shiftClaim(c, p.start));
          this.entries.update((e) => [...e, ...claims.map((claim) => ({ kind: 'claim' as const, claim, paragraph: k + 1 }))]);
          this.settle(line, 'done', `${head}: ${claims.length} claim(s), model ${r.extraction.model}`);
          break;
        } catch (e) {
          const f = e as ApiFailure;
          if (f.code === 'ExtractionFailed' && attempt < ATTEMPTS) {
            this.settle(line, 'running', `${head}: ${WB.progressRetry} (${attempt})`);
            continue;
          }
          if (f.code === 'NetworkError') this.status.markOffline();
          this.entries.update((x) => [...x, { kind: 'failure', paragraph: k + 1, text: p.text, failure: f, attempts: attempt }]);
          this.settle(line, 'failed', `${head}: ${f.code}: ${f.message}`);
          this.toast.show(UI.toastParagraph(k + 1, f.code), 'error');
          break;
        }
      }
      this.answered.set(k + 1);
    }
  }

  /** A built-in sample: its quantities go straight to POST /v1/evaluate, no language model. */
  private async runSample(sample: Sample): Promise<void> {
    const line = this.push({ text: UI.sampleTrace, status: 'running' });
    const results: (EvaluateResponse | null)[] = [];
    for (const c of sample.claims) {
      const req = evaluateRequest(c);
      let r: EvaluateResponse | null = null;
      if (req) {
        try {
          r = await firstValueFrom(this.api.evaluate(req));
        } catch (e) {
          const f = e as ApiFailure;
          if (f.code === 'NetworkError') this.status.markOffline();
          this.toast.show(`${c.quantity}: ${f.code}`, 'error');
        }
      }
      results.push(r);
      const claims = sampleClaims(sample, results);
      const claim = claims[results.length - 1];
      this.entries.update((e) => [...e, { kind: 'claim', claim, paragraph: this.paragraphOf(claim.evidence?.start ?? 0) }]);
      this.answered.set(this.paragraphOf(claim.evidence?.start ?? 0));
    }
    this.answered.set(this.parts().length);
    this.settle(line, 'done');
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
