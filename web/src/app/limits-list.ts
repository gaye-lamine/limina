import { Component, input } from '@angular/core';
import { ApiFailure, LimitInfo } from './api.models';
import { STRINGS } from './strings';

/** The library: one card per limit, with display labels only, never raw ids. */
@Component({
  selector: 'app-limits-list',
  templateUrl: './limits-list.html',
})
export class LimitsList {
  readonly limits = input<LimitInfo[]>([]);
  readonly failure = input<ApiFailure | null>(null);
  protected readonly s = STRINGS;

  /** Display label of a law classification. */
  protected classificationLabel(id: string): string {
    return this.s.classification[id] ?? id;
  }

  /** Display label of an id from the metadata; the id itself when the API gives none. */
  protected label(labels: Record<string, string>, id: string): string {
    return labels[id] ?? id;
  }
}
