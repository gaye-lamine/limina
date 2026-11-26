/**
 * Colours of the 3D scene, read at run time from the design tokens of
 * `web/src/tokens.css`, so no colour is written in the 3D code.
 */
import { Color } from 'three';

export interface LabTheme {
  bg: Color;
  surface1: Color;
  surface2: Color;
  surface3: Color;
  accent: Color;
  textPrimary: Color;
  textSecondary: Color;
  impossible: Color;
  conditional: Color;
  compatible: Color;
}

function token(style: CSSStyleDeclaration, name: string, fallback: Color): Color {
  const raw = style.getPropertyValue(name).trim();
  if (!/^#[0-9a-f]{3,8}$/i.test(raw)) return fallback.clone();
  return new Color(raw.slice(0, 7));
}

/** The theme from the document's computed tokens; a missing token keeps a neutral grey. */
export function readTheme(root: Element = document.documentElement): LabTheme {
  const style = getComputedStyle(root);
  const grey = new Color(0x808080);
  return {
    bg: token(style, '--bg', grey),
    surface1: token(style, '--surface-1', grey),
    surface2: token(style, '--surface-2', grey),
    surface3: token(style, '--surface-3', grey),
    accent: token(style, '--accent', grey),
    textPrimary: token(style, '--text-primary', grey),
    textSecondary: token(style, '--text-secondary', grey),
    impossible: token(style, '--v-impossible', grey),
    conditional: token(style, '--v-conditional', grey),
    compatible: token(style, '--v-compatible', grey),
  };
}
