import { Component, ElementRef, inject, output, signal, viewChild } from '@angular/core';
import { AnalysisService } from '../analysis.service';
import { ApiFailure } from '../api.models';
import { LAB } from '../strings';
import { FORGE_MAX, ForgeStep, canForge, forgeDone, forgeFailure, forgeRunning } from './forge';

/**
 * "Forge a new law" (spec laboratory-3d, R-16): a panel of the laboratory
 * with a sentence box, three examples, the progress of the request and its
 * refusal in plain words. On success it emits the draft's id; the page
 * reloads the library and opens it. Escape closes the panel and gives the
 * focus back to its button.
 */
@Component({
  selector: 'app-lab-forge',
  templateUrl: './lab-forge.html',
})
export class LabForge {
  protected readonly t = LAB.forge;
  protected readonly max = FORGE_MAX;
  private readonly analysis = inject(AnalysisService);
  private readonly box = viewChild<ElementRef<HTMLTextAreaElement>>('box');
  private readonly opener = viewChild<ElementRef<HTMLButtonElement>>('opener');

  /** The id of the draft just written and registered. */
  readonly forged = output<string>();

  protected readonly open = signal(false);
  protected readonly text = signal('');
  protected readonly busy = signal(false);
  protected readonly steps = signal<ForgeStep[]>([]);
  protected readonly error = signal('');
  protected readonly note = signal('');

  protected toggle(): void {
    this.open.update((o) => !o);
    if (this.open()) setTimeout(() => this.box()?.nativeElement.focus());
  }

  protected close(): void {
    this.open.set(false);
    setTimeout(() => this.opener()?.nativeElement.focus());
  }

  protected onKey(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.stopPropagation();
      this.close();
    }
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLTextAreaElement).value);
    this.error.set('');
  }

  protected useExample(s: string): void {
    this.text.set(s);
    this.error.set('');
    this.box()?.nativeElement.focus();
  }

  protected chars(): number {
    return [...this.text()].length;
  }

  protected ready(): boolean {
    return canForge(this.text()) && !this.busy();
  }

  protected submit(): void {
    if (!this.ready()) return;
    this.busy.set(true);
    this.error.set('');
    this.note.set('');
    this.steps.set(forgeRunning());
    this.analysis.author(this.text()).subscribe({
      next: (r) => {
        this.busy.set(false);
        this.steps.set(forgeDone(r));
        this.forged.emit(r.limit_id);
      },
      error: (f: ApiFailure) => {
        this.busy.set(false);
        this.steps.set([]);
        this.error.set(forgeFailure(f));
      },
    });
  }

  /** Set by the page once the draft is open in the scene. */
  announce(title: string): void {
    this.note.set(this.t.opened(title));
  }
}
