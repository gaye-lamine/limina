/**
 * The downloadable review as a PDF a non-specialist can read. The document
 * definition is built here from engine results only (pure and testable);
 * pdfmake is loaded on demand when the user downloads.
 */
import type { Content, TDocumentDefinitions } from 'pdfmake/interfaces';
import { LimitInfo } from './api.models';
import {
  Entry,
  VerdictKind,
  entryTitle,
  noVerdict,
  plainSentence,
  summary,
  verdictKind,
  verdictText,
} from './review-logic';
import {
  OptionResult,
  boundDisplay,
  calculation,
  doiUrl,
  focusOption,
  formalStatement,
  inClaimUnit,
  prettyFormula,
  ratioDisplay,
  valuesUsed,
} from './review-format';
import { STRINGS, UI } from './strings';
import { badge } from './workbench-logic';

/** Verdict colors for paper: darker than the screen tokens, for contrast on white. */
export const PRINT: Record<VerdictKind, { ink: string; tint: string }> = {
  impossible: { ink: '#B42332', tint: '#FCE8EA' },
  conditional: { ink: '#8A5A00', tint: '#FBF1DE' },
  compatible: { ink: '#1E7A4C', tint: '#E3F4EA' },
  insufficient: { ink: '#4F5B6B', tint: '#EEF0F3' },
  needs: { ink: '#4F5B6B', tint: '#EEF0F3' },
  disputed: { ink: '#6040C8', tint: '#EEE9FD' },
};
const INK = '#111827';
const MUTED = '#4B5563';
const RULE = '#E5E7EB';
const ACCENT = '#3B5BDB';

/** What the review adds to each claim: the solver's condition and the result per open option. */
export interface ReviewExtras {
  solved: Record<number, string>;
  options: Record<number, OptionResult[]>;
}

function pill(label: string, kind: VerdictKind): Content {
  return {
    table: { body: [[{ text: label, color: PRINT[kind].ink, bold: true, fontSize: 9, characterSpacing: 0.6, margin: [6, 2, 6, 2] }]] },
    layout: { fillColor: () => PRINT[kind].tint, hLineWidth: () => 0, vLineWidth: () => 0 },
  };
}

function metrics(cells: [string, string, string?][], kind: VerdictKind): Content {
  return {
    table: {
      widths: ['*', '*', '*'],
      body: [
        cells.map(([label]) => ({ text: label.toUpperCase(), fontSize: 8, color: MUTED, bold: true, characterSpacing: 0.6, margin: [8, 6, 8, 0] })),
        cells.map(([, value, note], i) => ({
          stack: [
            { text: value, fontSize: 13, bold: true, color: i === 2 ? PRINT[kind].ink : INK },
            ...(note ? [{ text: note, fontSize: 8, color: MUTED }] : []),
          ],
          margin: [8, 2, 8, 6],
        })),
      ],
    },
    layout: { fillColor: () => '#F7F8FA', hLineWidth: () => 0, vLineWidth: (i: number) => (i === 1 || i === 2 ? 1 : 0), vLineColor: () => '#FFFFFF' },
    margin: [0, 6, 0, 0],
  };
}

function labelled(label: string, text: string): Content {
  return { text: [{ text: `${label}  `, bold: true, color: MUTED, fontSize: 9 }, { text, fontSize: 9, color: INK }], margin: [0, 3, 0, 0] };
}

function reference(r: string): Content {
  const url = doiUrl(r);
  return url ? { text: r, link: url, color: ACCENT, fontSize: 8.5 } : { text: r, fontSize: 8.5, color: MUTED };
}

function claimBlock(e: Entry, i: number, limits: LimitInfo[], extras: ReviewExtras): Content {
  const kind = verdictKind(e);
  const limit = e.kind === 'claim' ? limits.find((l) => l.id === e.claim.limit_id) : undefined;
  const parts: Content[] = [
    {
      columns: [
        { text: `${i + 1}. ${entryTitle(e)}`, style: 'claimTitle', width: '*' },
        { width: 'auto', stack: [pill(verdictText(e), kind)] },
      ],
      columnGap: 12,
    },
    { text: plainSentence(e, limit?.direction ?? 'Max', limit), style: 'sentence' },
  ];
  const quote = e.kind === 'claim' ? e.claim.evidence?.text : e.text;
  if (quote) {
    parts.push({
      table: { widths: ['*'], body: [[{ text: `“${quote.replace(/\s+/g, ' ')}”`, italics: true, color: MUTED, fontSize: 10, margin: [10, 2, 0, 2] }]] },
      layout: { hLineWidth: () => 0, vLineWidth: (j: number) => (j === 0 ? 2 : 0), vLineColor: () => PRINT[kind].ink },
      margin: [0, 6, 0, 0],
    });
  }
  const card = noVerdict(e, limits);
  if (e.kind === 'claim' && !card) {
    const c = e.claim;
    const direction = limit?.direction ?? 'Max';
    const options = extras.options[i] ?? [];
    const focus = focusOption(options);
    const shown = focus?.result ?? c.result;
    parts.push(
      metrics(
        [
          [UI.metricClaim, c.value !== null ? inClaimUnit(c.value, c) : `${c.raw_value} ${c.raw_unit}`],
          [UI.metricBound, boundDisplay(c, shown, direction), focus?.option],
          [UI.metricRatio, ratioDisplay(c, shown, direction) ?? UI.metricNone, focus?.option],
        ],
        kind,
      ),
    );
    const used = valuesUsed(c, limit);
    if (used) parts.push(labelled(UI.valuesUsed, used));
    const calc = calculation(focus ? { ...c, result: focus.result } : c, limit);
    if (calc) parts.push(labelled(UI.calculation, calc));
    if (extras.solved[i]) parts.push(labelled(UI.condition, extras.solved[i]));
    if (options.length) {
      parts.push({ text: `${UI.byAssumption} · ${options[0].assumption}`, bold: true, color: MUTED, fontSize: 9, margin: [0, 5, 0, 1] });
      for (const o of options) {
        const r = ratioDisplay(c, o.result, direction);
        parts.push({
          text: [
            { text: `${o.option}: `, fontSize: 9, color: INK },
            { text: `${UI.optionBound(boundDisplay(c, o.result, direction))}${r ? ` · ${UI.optionRatio(r)}` : ''} · `, fontSize: 9, color: MUTED },
            { text: STRINGS.verdictLabel[o.result.verdict], fontSize: 9, bold: true, color: PRINT[o.result.verdict === 'IMPOSSIBLE' ? 'impossible' : o.result.verdict === 'COMPATIBLE' ? 'compatible' : 'conditional'].ink },
          ],
          margin: [8, 1, 0, 0],
        });
      }
    }
    if (limit) parts.push(labelled(UI.reportLaw, `${limit.title} (${prettyFormula(limit.formula)})`));
    parts.push(labelled(UI.reportStatus, badge(c)));
    if (c.result) parts.push(labelled(UI.engineWording, formalStatement(c.result.wording, limit)));
    const refs = limit?.references ?? [];
    if (refs.length) parts.push({ text: UI.sourceReferences, bold: true, color: MUTED, fontSize: 9, margin: [0, 4, 0, 1] }, { ul: refs.map(reference) });
  } else if (card) {
    parts.push(labelled(UI.settle, card.settlePrint));
    for (const k of card.candidates) parts.push(labelled(k.model, `${k.formula}${k.bound !== null ? `  →  ${k.bound.toPrecision(3)}` : ''}`));
    for (const src of card.sources) parts.push({ text: `• ${src}`, fontSize: 9, color: MUTED, margin: [0, 2, 0, 0] });
  }
  return { stack: parts, unbreakable: true, margin: [0, 14, 0, 0] };
}

/** The PDF document of a review: summary, how to read it, then one block per claim. */
export function reviewDocument(
  entries: Entry[],
  paragraphs: number,
  limits: LimitInfo[],
  date: string,
  version: string,
  extras: ReviewExtras = { solved: {}, options: {} },
): TDocumentDefinitions {
  const s = summary(entries, paragraphs);
  const aiWritten = entries.some((e) => e.kind === 'claim' && (e.claim.law?.source ?? '').startsWith('draft'));
  const counts: Content = {
    columns: s.counts.map((c) => ({
      width: 'auto',
      table: { body: [[{ text: [{ text: `${c.count}  `, bold: true, fontSize: 14 }, { text: c.label.toUpperCase(), fontSize: 8.5, bold: true, characterSpacing: 0.6 }], color: PRINT[c.kind].ink, margin: [10, 5, 10, 5] }]] },
      layout: { fillColor: () => PRINT[c.kind].tint, hLineWidth: () => 0, vLineWidth: () => 0 },
    })),
    columnGap: 8,
    margin: [0, 12, 0, 0],
  };
  // Every status the review uses is defined, the four verdicts first.
  const shown = new Set(entries.map(verdictText));
  const extra = Object.entries(UI.legendMore).filter(([label]) => shown.has(label));
  const legendLine = (label: string, meaning: string): Content => ({
    text: [{ text: `${label}  `, bold: true, fontSize: 9 }, { text: meaning, fontSize: 9, color: MUTED }],
    margin: [0, 2, 0, 0],
  });
  const legend: Content = {
    stack: [
      { text: UI.pdfHowToRead, style: 'h2' },
      ...(['IMPOSSIBLE', 'CONDITIONAL', 'COMPATIBLE'] as const).map((v) => legendLine(STRINGS.verdictLabel[v], STRINGS.legendMeaning[v])),
      legendLine(STRINGS.verdictLabel.INSUFFICIENT_DATA, UI.legendMore['INSUFFICIENT DATA']),
      ...extra.filter(([label]) => label !== 'INSUFFICIENT DATA').map(([label, meaning]) => legendLine(label, meaning)),
    ],
    margin: [0, 18, 0, 0],
  };
  const footerText = aiWritten ? `Generated by Limina; ${UI.aiFooter}. Limina ${version}.` : `Generated by Limina ${version}.`;
  return {
    pageSize: 'A4',
    pageMargins: [48, 56, 48, 56],
    info: { title: UI.reportTitle, creator: 'Limina' },
    defaultStyle: { font: 'Roboto', fontSize: 10, color: INK, lineHeight: 1.25 },
    header: () => ({
      columns: [
        { text: 'Limina', bold: true, color: ACCENT, fontSize: 11 },
        { text: date, alignment: 'right', color: MUTED, fontSize: 9 },
      ],
      margin: [48, 24, 48, 0],
    }),
    footer: (page: number, pages: number) => ({
      columns: [
        { text: footerText, color: MUTED, fontSize: 8 },
        { text: `${page} / ${pages}`, alignment: 'right', color: MUTED, fontSize: 8 },
      ],
      margin: [48, 16, 48, 0],
    }),
    content: [
      { text: UI.reportTitle, style: 'h1' },
      { text: `${UI.claimsCount(s.claims)} checked in ${UI.paragraphsCount(s.paragraphs)}.`, color: MUTED, margin: [0, 4, 0, 0] },
      counts,
      legend,
      { canvas: [{ type: 'line', x1: 0, y1: 0, x2: 499, y2: 0, lineWidth: 1, lineColor: RULE }], margin: [0, 16, 0, 0] },
      ...entries.map((e, i) => claimBlock(e, i, limits, extras)),
      { text: STRINGS.footerNote, fontSize: 8.5, color: MUTED, italics: true, margin: [0, 24, 0, 0] },
    ],
    styles: {
      h1: { fontSize: 22, bold: true, color: INK },
      h2: { fontSize: 11, bold: true, color: INK, margin: [0, 0, 0, 4] },
      claimTitle: { fontSize: 12.5, bold: true, color: INK },
      sentence: { fontSize: 11, color: INK, margin: [0, 6, 0, 0] },
    },
  };
}

/** Builds the PDF in the browser and downloads it; pdfmake is loaded only now. */
export async function downloadReviewPdf(doc: TDocumentDefinitions, fileName: string): Promise<void> {
  const [pdfMake, fonts] = await Promise.all([import('pdfmake/build/pdfmake'), import('pdfmake/build/vfs_fonts')]);
  const make = (pdfMake as unknown as { default?: typeof pdfMake }).default ?? pdfMake;
  const vfs = (fonts as unknown as { default?: Record<string, string> }).default ?? (fonts as unknown as Record<string, string>);
  make.addVirtualFileSystem(vfs);
  await make.createPdf(doc).download(fileName);
}
