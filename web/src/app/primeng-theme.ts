/**
 * PrimeNG theme: the Aura preset mapped onto Limina's tokens (dark only,
 * one accent). PrimeNG is used for the "How this was checked" drawer
 * (Drawer, Accordion, Timeline). Values mirror web/src/tokens.css.
 */
import { definePreset } from '@primeuix/themes';
import Aura from '@primeuix/themes/aura';

export const LiminaPreset = definePreset(Aura, {
  primitive: {
    borderRadius: { none: '0', xs: '4px', sm: '8px', md: '12px', lg: '16px', xl: '16px' },
  },
  semantic: {
    primary: {
      50: '#eef3ff',
      100: '#dce6ff',
      200: '#c2d4ff',
      300: '#9dbbff',
      400: '#7aa2ff',
      500: '#7aa2ff',
      600: '#5f88f0',
      700: '#4a70d6',
      800: '#3c5bb0',
      900: '#32498c',
      950: '#1f2c55',
    },
    colorScheme: {
      dark: {
        surface: {
          0: '#ffffff',
          50: '#e7eaee',
          100: '#cfd4db',
          200: '#a3acb8',
          300: '#8d98a8',
          400: '#6f7885',
          500: '#4a525d',
          600: '#2a3039',
          700: '#1e232b',
          800: '#171b22',
          900: '#11141a',
          950: '#0a0c10',
        },
        primary: { color: '#7aa2ff', contrastColor: '#0a0c10', hoverColor: '#9dbbff', activeColor: '#9dbbff' },
        text: { color: '#e7eaee', hoverColor: '#ffffff', mutedColor: '#a3acb8', hoverMutedColor: '#e7eaee' },
        content: { background: '#11141a', hoverBackground: '#171b22', borderColor: 'rgba(255, 255, 255, 0.08)', color: '#e7eaee', hoverColor: '#ffffff' },
        overlay: {
          modal: { background: '#11141a', borderColor: 'rgba(255, 255, 255, 0.16)', color: '#e7eaee' },
        },
        focusRing: { width: '2px', style: 'solid', color: '#7aa2ff', offset: '2px' },
      },
    },
  },
});

/** PrimeNG options: always dark, styles in their own cascade layer so the app's styles win. */
export const PRIMENG_OPTIONS = {
  theme: {
    preset: LiminaPreset,
    options: { darkModeSelector: ':root', cssLayer: { name: 'primeng', order: 'theme, base, primeng, components, utilities' } },
  },
  ripple: false,
};
