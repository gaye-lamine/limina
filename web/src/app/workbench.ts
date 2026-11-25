import { Component, ElementRef, HostListener, computed, inject, signal, viewChild } from '@angular/core';
import { Observable, firstValueFrom } from 'rxjs';
import { AnalyzeResponse, AnalyzedClaim, PipelineInfo } from './analysis.models';
import { AnalysisService } from './analysis.service';
import { ApiFailure, LimitInfo } from './api.models';
import { ApiService } from './api.service';
import { CarnotApparatus } from './carnot-apparatus';
import { formatValue } from './chart-geometry';
import { CopApparatus, TwinVerdicts } from './cop-apparatus';
import { Explorer } from './explorer';
import {
  Entry,
  Segment,
  StampKind,
  boundText,
  claimText,
  reviewMarkdown,
  score,
  scoreLine,
  segments,
  stampKind,
  stampLabel,
  stampRotation,
  stampSeed,
  stampSentence,
} from './review-logic';
import { REVIEW, STRINGS, WB } from './strings';
import {
  Paragraph,
  badge,
  explain,
  paragraphs,
  rememberDirection,
  shiftClaim,
  solveSentence,
} from './workbench-logic';

/** The pre-filled text: synthetic claims about imaginary products, one per discipline. */
export const ILLUSTRATIVE_TEXT = [
  'The Helios probe cruises at a speed of 3.6e8 m/s relative to the Sun.',
  'The Q-Scope locates an electron within 1e-12 m while its momentum is known to within 1e-24 kg*m/s.',
  'The Lumen microscope resolves details of 100 nm using light of 500 nm wavelength and a numerical aperture of 1.4.',
  'The Aurora heat pump delivers heat with a COP of 14.5, hot reservoir at 300 K, cold reservoir at 280 K.',
  'The Quanta logic chip erases one bit at 300 K while dissipating 1e-22 J.',
  'The Volta cell stores 5 kWh of energy in a mass of 1 kg.',
].join('\n\n');

/** Answers tried per paragraph when the model answer is unusable. */
export const ATTEMPTS = 3;

export type { Entry } from './review-logic';

/** A stamp in the margin of the document. */
export interface Stamp {
  index: number;
  kind: StampKind;
  label: string;
  sentence: string;
  rotation: number;
  order: number;
}

/** A paragraph of the document with its underlined numbers and its stamps. */
export interface DocParagraph {
  segments: Segment[];
  stamps: Stamp[];
}

/** A line of the run log, shown above the trace while and after analysing. */
export interface LogLine {
  text: string;
  status: 'running' | 'done' | 'failed';
}

/**
 * The analysis screen as a stamped document. Three states: the input
 * text; the running document with a scan line and a ticker; the reviewed
 * document (stamps in the margin, scoreboard above) beside the instrument
 * panel of the selected claim. The pipeline trace is in a drawer.
 */
@Component({
  selector: 'app-workbench',
  imports: [Explorer, CarnotApparatus, CopApparatus],
  templateUrl: './workbench.html',
  host: { '[attr.data-state]': 'state()' },
})
export class Workbench {
  private readonly analysis = inject(AnalysisService);
  private readonly api = inject(ApiService);
  protected readonly s = WB;
  protected readonly r = REVIEW;
  protected readonly parts = signal<Paragraph[]>([]);
  protected readonly current = signal(-1);
  protected readonly highlight = signal<string | null>(null);
  protected readonly howOpen = signal(false);
  private readonly howDrawer = viewChild<ElementRef<HTMLElement>>('how');

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
  /** input: the text box; running: the document being scanned; reviewed: document and instrument. */
  protected readonly state = computed<'input' | 'running' | 'reviewed'>(() =>
    this.running() ? 'running' : this.analysed() ? 'reviewed' : 'input',
  );
  protected readonly stamps = computed<Stamp[]>(() => {
    let order = 0;
    return this.entries().map((e, index) => ({
      index,
      kind: stampKind(e),
      label: stampLabel(e),
      sentence: stampSentence(e, this.directionOf(e)),
      rotation: stampRotation(stampSeed(e, index)),
      order: order++,
    }));
  });
  protected readonly doc = computed<DocParagraph[]>(() => {
    const entries = this.entries();
    const stamps = this.stamps();
    return this.parts().map((p, k) => {
      const mine = entries.map((e, index) => ({ e, index })).filter(({ e }) => e.paragraph === k + 1);
      const claims = mine.flatMap(({ e, index }) => (e.kind === 'claim' ? [{ index, claim: e.claim }] : []));
      return { segments: segments(p.text, p.start, claims), stamps: mine.map(({ index }) => stamps[index]) };
    });
  });
  protected readonly board = computed(() => score(this.entries(), this.parts().length));
  protected readonly boardLine = computed(() => scoreLine(this.board()));
  protected readonly selectedStamp = computed(() => this.stamps()[this.selected()] ?? null);
  protected readonly selectedParagraph = computed(() => (this.entry()?.paragraph ?? 0) - 1);
  protected readonly ticker = computed(() => {
    const l = this.log();
    const last = l[l.length - 1];
    return last ? last.text : REVIEW.tickerIdle;
  });
  protected readonly references = computed(() => this.limit()?.references ?? []);

  private directionOf(e: Entry): 'Max' | 'Min' | 'Equal' {
    if (e.kind !== 'claim') return 'Max';
    return this.limits().find((l) => l.id === e.claim.limit_id)?.direction ?? 'Max';
  }

  constructor() {
    this.loadLimits();
    this.analysis.pipeline().subscribe({ next: (i) => this.info.set(i), error: () => this.info.set(null) });
  }

  private loadLimits(): void {
    this.api.getLimits().subscribe({
      next: (r) => {
        for (const l of r.limits) rememberDirection(l.id, l.direction === 'Min' ? 'Min' : 'Max');
        this.limits.set(r.limits);
      },
      error: () => undefined,
    });
  }

  protected claimLine(c: AnalyzedClaim): string {
    return claimText(c);
  }

  protected entryTitle(e: Entry): string {
    return e.kind === 'claim' ? claimText(e.claim) : `${WB.progressParagraph} ${e.paragraph}`;
  }

  protected isLit(i: number): boolean {
    const h = this.highlight();
    return h !== null && this.stamps()[i]?.label === h;
  }

  protected toggleHighlight(label: string): void {
    const next = this.highlight() === label ? null : label;
    this.highlight.set(next);
    const first = this.stamps().find((s) => s.label === next);
    if (first) this.select(first.index);
  }

  protected edit(): void {
    this.analysed.set(false);
    this.howOpen.set(false);
  }

  protected openHow(): void {
    this.howOpen.set(true);
    setTimeout(() => this.howDrawer()?.nativeElement.focus());
  }

  protected closeHow(): void {
    this.howOpen.set(false);
  }

  @HostListener('document:keydown.escape')
  protected onEscape(): void {
    this.closeHow();
  }

  /** Arrow keys move between claims in the reviewed state, except inside form controls. */
  @HostListener('document:keydown', ['$event'])
  protected onDocKey(event: KeyboardEvent): void {
    if (this.state() !== 'reviewed' || this.howOpen()) return;
    const t = event.target as HTMLElement | null;
    if (t && (/^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName) || t.isContentEditable)) return;
    const n = this.entries().length;
    if (n === 0) return;
    const i = this.selected();
    if (event.key === 'ArrowDown' || event.key === 'ArrowRight') this.select((i + 1) % n);
    else if (event.key === 'ArrowUp' || event.key === 'ArrowLeft') this.select((i - 1 + n) % n);
    else return;
    event.preventDefault();
  }

  /** The review as Markdown, downloaded from the browser. */
  protected download(): void {
    const md = reviewMarkdown(this.entries(), this.parts().length, this.limits());
    const url = URL.createObjectURL(new Blob([md], { type: 'text/markdown;charset=utf-8' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = REVIEW.reportFile;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url));
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
    return boundText(c);
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
    this.highlight.set(null);
    this.log.set(this.log().filter((l) => l.text.startsWith(WB.imageRead)));
    const text = this.text();
    const parts = paragraphs(text);
    this.parts.set(parts);
    for (const [k, p] of parts.entries()) {
      this.current.set(k);
      const head = `${WB.progressParagraph} ${k + 1} ${WB.progressOf} ${parts.length}`;
      const line = this.push({ text: `${head}: ${p.text}`, status: 'running' });
      let attempt = 0;
      for (;;) {
        attempt += 1;
        try {
          const r = await this.analyzeOnce(p.text);
          const claims = r.claims.map((c) => shiftClaim(c, p.start));
          this.entries.update((e) => [...e, ...claims.map((claim) => ({ kind: 'claim' as const, claim, paragraph: k + 1 }))]);
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
    this.current.set(-1);
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
