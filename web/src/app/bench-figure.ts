/**
 * Layout shared by every apparatus on the bench.
 *
 * The figure keeps its aspect ratio and is limited to about 60vh, and never
 * to less than the height left on a 1080 px screen once the page header,
 * the case select and the control row are drawn, so the figure and its
 * controls are visible together. The minimum width and height, set inline
 * from each component, keep every text at 12 px or more; below that width
 * the figure scrolls horizontally. The controls sit in one compact row
 * directly under the figure.
 */
export const FIGURE_MAX_HEIGHT = 'min(60vh, calc(100vh - 26rem))';

/** Smallest scale at which a figure is drawn, so 15-unit text renders at 12 px. */
export const MIN_FIGURE_SCALE = 0.8;

export const BENCH_FIGURE_STYLES = `
  .apparatus-scroll {
    overflow-x: auto;
  }

  .apparatus-figure {
    display: block;
    width: 100%;
    height: auto;
    max-height: min(60vh, calc(100vh - 26rem));
    margin: 0 auto;
  }

  .apparatus-controls {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
    gap: 0.5rem 1rem;
    align-items: end;
    margin-top: 0.5rem;
  }
`;
