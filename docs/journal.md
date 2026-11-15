# Journal

One entry per working session. Newest first.

## 2026-11-15

Tranche 1 : 27 tests, 3 écarts au design, tous consignés.

Tranche 2 : 40 tests, 2 écarts au design consignés.

Tranche 3 : 58 tests, 3 écarts consignés, trou de couverture R-07.5 repéré par Kiro (doc strings).

Tranche 4 : 89 tests, 5 écarts consignés, tous liés à la visibilité Rust.

Tranche 5 : 131 tests, tests écrits avant le code, oracle indépendant, expérience de mutation (garde cassé détecté par P-02 et P-03(a)), aucun bogue réel trouvé.

Tranche 6 : 149 tests, 18 propriétés à 1024 cas en CI, aucune défaillance.

### Expérience de mutation (tranche 5)

Bogue injecté volontairement, pas un bogue trouvé dans le code.

- Mutation : dans `rules_max`, la règle IMPOSSIBLE `lo > b + g(b)` a été remplacée par `lo > b` (garde supprimé). Le `debug_assert!` de cohérence a été neutralisé pour l'expérience.
- Détection : P-02 et P-03(a) ont échoué. E-01 est resté vert, ce qui est attendu (E-01 tourne sans garde).
- Contre-exemple minimisé par proptest (P-03(a)) :
  `Max-bound | B(0)=0.0 | claim v=1e-12 u=0.0 k=1.0 -> lo=1e-12 hi=1e-12 | guard Default (rel=1e-9, abs=1e-12)`
  Le moteur muté répond IMPOSSIBLE, l'oracle CONDITIONAL : `v` est dans le garde absolu de la borne 0.
- Contre-exemple de P-02 : `Max-bound | B(0)=0.0 | claim v=5.498937935469428e-13 u=0.0 k=1.0 | guard Default`.
- Code rétabli à l'identique, puis 131 tests verts, en local (256 cas) et avec `CI=true` (1024 cas).

### Décisions (tranche 5)

- Une limite qui renvoie un intervalle non ponctuel alors que ses paramètres sont ponctuels viole le contrat de `Limit::bound`. Dans ce cas `decide` renvoie `InsufficientData` avec une liste vide, jamais IMPOSSIBLE. Noté ici ; la spec n'est pas rouverte.
- Marge : si un des quatre rapports n'est pas fini, la marge vaut `None`, comme pour `B_min = 0`. Le verdict n'est pas affecté (décision D-23, appliquée en tranche 6).

## YYYY-MM-DD

### Decision

(What was decided and why. Alternatives considered.)

### Prompt used

(Exact prompt given to the agent.)

### Result

(What was produced. What worked, what did not.)

### Bug found by property test

(Property that failed, shrunk counterexample, fix. Write "none" if no bug.)
