import { A11yModule } from '@angular/cdk/a11y';
import { Component, ElementRef, computed, input, output, signal, viewChild, AfterViewInit } from '@angular/core';
import { LucideAngularModule, Search, CornerDownLeft } from 'lucide-angular';
import { UI } from './strings';

/** One command of the palette. */
export interface Command {
  id: string;
  group: 'claims' | 'actions';
  label: string;
  /** Short text on the right, for example the verdict of a claim. */
  hint?: string;
  /** Verdict family of a claim command, to color its hint (the hint text is always shown). */
  kind?: string;
}

/** Filters commands by every word of the query, case-insensitive. */
export function matchCommands(commands: Command[], query: string): Command[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  return commands.filter((c) => {
    const hay = `${c.label} ${c.hint ?? ''}`.toLowerCase();
    return words.every((w) => hay.includes(w));
  });
}

/** The command palette, shown in a CDK overlay: a search field and a list of commands. */
@Component({
  selector: 'app-palette',
  imports: [A11yModule, LucideAngularModule],
  template: `
    <div class="palette" role="dialog" aria-modal="true" [attr.aria-label]="u.palette" cdkTrapFocus>
      <div class="palette-search">
        <lucide-icon [img]="SearchIcon" [size]="16" aria-hidden="true" />
        <input
          #search
          class="palette-input"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-controls="palette-list"
          [attr.aria-activedescendant]="results().length ? 'cmd-' + active() : null"
          [attr.aria-label]="u.palette"
          [placeholder]="u.palettePlaceholder"
          [value]="query()"
          (input)="setQuery($any($event.target).value)"
          (keydown)="onKey($event)"
          autocomplete="off"
          spellcheck="false"
        />
        <kbd class="kbd">Esc</kbd>
      </div>
      <ul id="palette-list" class="palette-list" role="listbox" [attr.aria-label]="u.palette">
        @for (c of results(); track c.id; let i = $index) {
          @if (i === 0 || results()[i - 1].group !== c.group) {
            <li class="palette-group" role="presentation">{{ c.group === 'claims' ? u.paletteClaims : u.paletteActions }}</li>
          }
          <li
            [id]="'cmd-' + i"
            class="palette-item"
            role="option"
            tabindex="-1"
            [attr.aria-selected]="i === active()"
            (click)="run.emit(c.id)"
            (keydown.enter)="run.emit(c.id)"
            (mousemove)="active.set(i)"
          >
            <span class="palette-label">{{ c.label }}</span>
            @if (c.hint) {
              <span class="palette-hint palette-hint--{{ c.kind ?? '' }}">{{ c.hint }}</span>
            }
            @if (i === active()) {
              <lucide-icon class="palette-enter" [img]="EnterIcon" [size]="14" aria-hidden="true" />
            }
          </li>
        } @empty {
          <li class="palette-empty" role="presentation">{{ u.paletteEmpty }}</li>
        }
      </ul>
    </div>
  `,
})
export class Palette implements AfterViewInit {
  readonly commands = input<Command[]>([]);
  readonly run = output<string>();
  readonly closed = output<void>();
  protected readonly u = UI;
  protected readonly SearchIcon = Search;
  protected readonly EnterIcon = CornerDownLeft;
  protected readonly query = signal('');
  protected readonly active = signal(0);
  protected readonly results = computed(() => matchCommands(this.commands(), this.query()));
  private readonly search = viewChild<ElementRef<HTMLInputElement>>('search');

  ngAfterViewInit(): void {
    setTimeout(() => this.search()?.nativeElement.focus());
  }

  protected setQuery(q: string): void {
    this.query.set(q);
    this.active.set(0);
  }

  protected onKey(e: KeyboardEvent): void {
    const n = this.results().length;
    if (e.key === 'ArrowDown' && n) this.active.set((this.active() + 1) % n);
    else if (e.key === 'ArrowUp' && n) this.active.set((this.active() - 1 + n) % n);
    else if (e.key === 'Enter' && n) this.run.emit(this.results()[this.active()].id);
    else if (e.key === 'Escape') this.closed.emit();
    else return;
    e.preventDefault();
  }
}
