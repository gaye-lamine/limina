# Tasks: public-hosting

- [x] 1. Forge: normalize a wrong `kind`, exponents as text or in `units`, a dimension that disagrees with the units; prompt example, rules and repair hints; unit tests.
- [x] 2. Cap the authored limits at 50 (oldest first, with their agreement status); test.
- [x] 3. `limiter.rs`: per-address limit on the three model routes, `LLM_RATE_LIMIT_PER_HOUR`, `TRUST_PROXY`, `Retry-After`; tests.
- [x] 4. `static_files.rs`: the built web app with a fallback to `index.html`, safe paths; tests.
- [x] 5. `main.rs`: `PORT`, `LIMINA_API_ADDR`, `STATIC_DIR`, connection addresses; test.
- [x] 6. `Dockerfile` and `.dockerignore`.
- [x] 7. README: quick start, Docker, hosting, the laboratory, tests, 35 laws.
- [x] 8. Web: the 429 of the hourly limit explained in plain words (analysis and Forge).
- [ ] 9. Build the image on a machine with a running Docker daemon (this workstation had none): `docker build -t limina .` then `docker run -p 3000:3000 limina`.
