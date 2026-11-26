import { Routes } from '@angular/router';

/**
 * One route: the 3D laboratory, loaded on demand so Three.js never reaches
 * the home page bundle. The analysis screen is the shell's own content.
 */
export const routes: Routes = [
  {
    path: 'laboratory',
    loadComponent: () => import('./laboratory/laboratory').then((m) => m.Laboratory),
  },
];
