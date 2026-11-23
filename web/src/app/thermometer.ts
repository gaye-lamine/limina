import { Component, computed, input, output } from '@angular/core';
import { parseDecimal } from './decimal';
import { STRINGS } from './strings';

/** Tube geometry, in viewBox units. The SVG is drawn at 1 unit = 1 px. */
const TUBE_TOP = 8;
const TUBE_BOTTOM = 104;
const GRADUATION_STEP = 100;
const LABEL_STEP = 200;
/** Font size of the graduation labels, in px at the rendered size (never below 12). */
export const GRADUATION_FONT = 13;

/**
 * A vertical thermometer drawn as a graduated glass tube. It is backed by a
 * native range input and a decimal text field, both labelled. The field
 * accepts a decimal point or comma and keeps the text as typed.
 */
@Component({
  selector: 'app-thermometer',
  templateUrl: './thermometer.html',
})
export class Thermometer {
  /** Parameter id, for example "Th"; the inputs get the ids param-Th-value and param-Th-range. */
  readonly paramId = input.required<string>();
  /** Main letter of the symbol, for example "T". */
  readonly symbol = input<string>('T');
  /** Subscript of the symbol, for example "h". */
  readonly subscript = input.required<string>();
  readonly value = input<string>('');
  readonly unit = input<string>('K');
  /** Top of the scale. */
  readonly max = input<number>(1000);
  readonly valueChange = output<string>();

  protected readonly s = STRINGS;
  protected readonly graduationFont = GRADUATION_FONT;

  protected readonly numeric = computed(() => parseDecimal(this.value()));

  /** Value shown by the slider: the field value inside the scale, else 0. */
  protected readonly sliderValue = computed(() => {
    const v = this.numeric();
    return v === null ? 0 : Math.min(Math.max(v, 0), this.max());
  });

  /** Top of the liquid column; the column is empty without a value. */
  protected readonly levelY = computed(() => {
    const v = this.numeric();
    if (v === null) return TUBE_BOTTOM;
    const f = Math.min(Math.max(v, 0), this.max()) / this.max();
    return TUBE_BOTTOM - f * (TUBE_BOTTOM - TUBE_TOP);
  });

  protected readonly graduations = computed(() => {
    const marks: { y: number; label: string | null }[] = [];
    for (let v = 0; v <= this.max(); v += GRADUATION_STEP) {
      const y = TUBE_BOTTOM - (v / this.max()) * (TUBE_BOTTOM - TUBE_TOP);
      marks.push({ y, label: v % LABEL_STEP === 0 ? String(v) : null });
    }
    return marks;
  });

  protected readonly tubeBottom = TUBE_BOTTOM;
  protected readonly sliderLabel = computed(() => `${this.paramId()} ${STRINGS.thermometerRange}`);
}
