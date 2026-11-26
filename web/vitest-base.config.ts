import { defineConfig } from 'vitest/config';

/**
 * Base Vitest options merged by the Angular unit-test builder
 * (`runnerConfig` in angular.json). Two workers at most: on Windows, more
 * workers under load made the jsdom setup exceed the 5 s test timeout and
 * left some files reported as "No test suite found". The timeouts leave
 * room for the first render of a component, which compiles it.
 */
export default defineConfig({
  test: {
    maxWorkers: 2,
    testTimeout: 15_000,
    hookTimeout: 15_000,
  },
});
