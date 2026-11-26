import { Injectable } from '@angular/core';
import type { TDocumentDefinitions } from 'pdfmake/interfaces';
import { downloadReviewPdf } from './review-pdf';

/** Saves the review as a PDF on the user's device; replaced by a fake in tests. */
@Injectable({ providedIn: 'root' })
export class ReviewExport {
  download(doc: TDocumentDefinitions, fileName: string): Promise<void> {
    return downloadReviewPdf(doc, fileName);
  }
}
