# Résultats test262

Suite [tc39/test262](https://github.com/tc39/test262), révision `102924a26277`, dossiers `language`, `built-ins` et `annexB`. Généré par `cargo run -p oxyde_test262 --release -- --publier`.

Un test n'est réussi que s'il passe dans tous ses modes (strict et non strict). Aucun test n'est écarté : ceux qui demandent une fonctionnalité absente échouent.

**Total : 41466 réussis sur 48795 (85.0 %).**

| Dossier | Tests | Réussis | Taux |
|---|---:|---:|---:|
| `annexB/built-ins` | 241 | 182 | 75.5 % |
| `annexB/language` | 845 | 815 | 96.4 % |
| `built-ins/AbstractModuleSource` | 8 | 0 | 0.0 % |
| `built-ins/AggregateError` | 25 | 23 | 92.0 % |
| `built-ins/Array` | 3083 | 3023 | 98.1 % |
| `built-ins/ArrayBuffer` | 221 | 191 | 86.4 % |
| `built-ins/ArrayIteratorPrototype` | 27 | 23 | 85.2 % |
| `built-ins/AsyncDisposableStack` | 104 | 0 | 0.0 % |
| `built-ins/AsyncFromSyncIteratorPrototype` | 38 | 32 | 84.2 % |
| `built-ins/AsyncFunction` | 18 | 17 | 94.4 % |
| `built-ins/AsyncGeneratorFunction` | 23 | 21 | 91.3 % |
| `built-ins/AsyncGeneratorPrototype` | 48 | 45 | 93.8 % |
| `built-ins/AsyncIteratorPrototype` | 13 | 0 | 0.0 % |
| `built-ins/Atomics` | 389 | 275 | 70.7 % |
| `built-ins/BigInt` | 77 | 76 | 98.7 % |
| `built-ins/Boolean` | 51 | 50 | 98.0 % |
| `built-ins/DataView` | 560 | 534 | 95.4 % |
| `built-ins/Date` | 610 | 583 | 95.6 % |
| `built-ins/DisposableStack` | 93 | 0 | 0.0 % |
| `built-ins/Error` | 93 | 92 | 98.9 % |
| `built-ins/FinalizationRegistry` | 47 | 45 | 95.7 % |
| `built-ins/Function` | 509 | 498 | 97.8 % |
| `built-ins/GeneratorFunction` | 23 | 21 | 91.3 % |
| `built-ins/GeneratorPrototype` | 61 | 61 | 100.0 % |
| `built-ins/Infinity` | 6 | 6 | 100.0 % |
| `built-ins/Iterator` | 684 | 464 | 67.8 % |
| `built-ins/JSON` | 166 | 161 | 97.0 % |
| `built-ins/Map` | 204 | 198 | 97.1 % |
| `built-ins/MapIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/Math` | 327 | 314 | 96.0 % |
| `built-ins/NaN` | 6 | 6 | 100.0 % |
| `built-ins/NativeErrors` | 94 | 82 | 87.2 % |
| `built-ins/Number` | 340 | 337 | 99.1 % |
| `built-ins/Object` | 3411 | 3385 | 99.2 % |
| `built-ins/Promise` | 732 | 731 | 99.9 % |
| `built-ins/Proxy` | 311 | 307 | 98.7 % |
| `built-ins/Reflect` | 153 | 153 | 100.0 % |
| `built-ins/RegExp` | 1879 | 1099 | 58.5 % |
| `built-ins/RegExpStringIteratorPrototype` | 17 | 17 | 100.0 % |
| `built-ins/Set` | 383 | 377 | 98.4 % |
| `built-ins/SetIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/ShadowRealm` | 64 | 0 | 0.0 % |
| `built-ins/SharedArrayBuffer` | 104 | 101 | 97.1 % |
| `built-ins/String` | 1223 | 1179 | 96.4 % |
| `built-ins/StringIteratorPrototype` | 7 | 5 | 71.4 % |
| `built-ins/SuppressedError` | 22 | 20 | 90.9 % |
| `built-ins/Symbol` | 98 | 93 | 94.9 % |
| `built-ins/Temporal` | 4614 | 0 | 0.0 % |
| `built-ins/ThrowTypeError` | 14 | 0 | 0.0 % |
| `built-ins/TypedArray` | 1453 | 1418 | 97.6 % |
| `built-ins/TypedArrayConstructors` | 738 | 711 | 96.3 % |
| `built-ins/Uint8Array` | 70 | 8 | 11.4 % |
| `built-ins/WeakMap` | 141 | 140 | 99.3 % |
| `built-ins/WeakRef` | 29 | 27 | 93.1 % |
| `built-ins/WeakSet` | 85 | 84 | 98.8 % |
| `built-ins/decodeURI` | 55 | 48 | 87.3 % |
| `built-ins/decodeURIComponent` | 56 | 54 | 96.4 % |
| `built-ins/encodeURI` | 31 | 26 | 83.9 % |
| `built-ins/encodeURIComponent` | 31 | 26 | 83.9 % |
| `built-ins/eval` | 10 | 10 | 100.0 % |
| `built-ins/global` | 29 | 29 | 100.0 % |
| `built-ins/isFinite` | 15 | 15 | 100.0 % |
| `built-ins/isNaN` | 15 | 15 | 100.0 % |
| `built-ins/parseFloat` | 54 | 54 | 100.0 % |
| `built-ins/parseInt` | 55 | 55 | 100.0 % |
| `built-ins/undefined` | 8 | 8 | 100.0 % |
| `language/arguments-object` | 263 | 256 | 97.3 % |
| `language/asi` | 102 | 102 | 100.0 % |
| `language/block-scope` | 145 | 145 | 100.0 % |
| `language/comments` | 52 | 52 | 100.0 % |
| `language/computed-property-names` | 48 | 48 | 100.0 % |
| `language/destructuring` | 19 | 18 | 94.7 % |
| `language/directive-prologue` | 62 | 62 | 100.0 % |
| `language/eval-code` | 347 | 342 | 98.6 % |
| `language/export` | 3 | 3 | 100.0 % |
| `language/expressions` | 11156 | 10832 | 97.1 % |
| `language/function-code` | 217 | 217 | 100.0 % |
| `language/future-reserved-words` | 55 | 55 | 100.0 % |
| `language/global-code` | 42 | 42 | 100.0 % |
| `language/identifier-resolution` | 14 | 13 | 92.9 % |
| `language/identifiers` | 268 | 268 | 100.0 % |
| `language/import` | 129 | 15 | 11.6 % |
| `language/keywords` | 25 | 25 | 100.0 % |
| `language/line-terminators` | 41 | 34 | 82.9 % |
| `language/literals` | 536 | 528 | 98.5 % |
| `language/module-code` | 599 | 532 | 88.8 % |
| `language/punctuators` | 11 | 11 | 100.0 % |
| `language/reserved-words` | 27 | 27 | 100.0 % |
| `language/rest-parameters` | 11 | 11 | 100.0 % |
| `language/source-text` | 1 | 1 | 100.0 % |
| `language/statementList` | 80 | 80 | 100.0 % |
| `language/statements` | 9399 | 9179 | 97.7 % |
| `language/types` | 113 | 113 | 100.0 % |
| `language/white-space` | 67 | 67 | 100.0 % |
