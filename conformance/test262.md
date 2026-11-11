# Résultats test262

Suite [tc39/test262](https://github.com/tc39/test262), révision `102924a26277`, dossiers `language`, `built-ins` et `annexB`. Généré par `cargo run -p oxyde_test262 --release -- --publier`.

Un test n'est réussi que s'il passe dans tous ses modes (strict et non strict). Aucun test n'est écarté : ceux qui demandent une fonctionnalité absente échouent.

**Total : 28064 réussis sur 48795 (57.5 %).**

| Dossier | Tests | Réussis | Taux |
|---|---:|---:|---:|
| `annexB/built-ins` | 241 | 52 | 21.6 % |
| `annexB/language` | 845 | 408 | 48.3 % |
| `built-ins/AbstractModuleSource` | 8 | 0 | 0.0 % |
| `built-ins/AggregateError` | 25 | 0 | 0.0 % |
| `built-ins/Array` | 3083 | 2570 | 83.4 % |
| `built-ins/ArrayBuffer` | 221 | 160 | 72.4 % |
| `built-ins/ArrayIteratorPrototype` | 27 | 23 | 85.2 % |
| `built-ins/AsyncDisposableStack` | 104 | 0 | 0.0 % |
| `built-ins/AsyncFromSyncIteratorPrototype` | 38 | 10 | 26.3 % |
| `built-ins/AsyncFunction` | 18 | 9 | 50.0 % |
| `built-ins/AsyncGeneratorFunction` | 23 | 5 | 21.7 % |
| `built-ins/AsyncGeneratorPrototype` | 48 | 25 | 52.1 % |
| `built-ins/AsyncIteratorPrototype` | 13 | 0 | 0.0 % |
| `built-ins/Atomics` | 389 | 0 | 0.0 % |
| `built-ins/BigInt` | 77 | 74 | 96.1 % |
| `built-ins/Boolean` | 51 | 49 | 96.1 % |
| `built-ins/DataView` | 560 | 491 | 87.7 % |
| `built-ins/Date` | 610 | 581 | 95.2 % |
| `built-ins/DisposableStack` | 93 | 0 | 0.0 % |
| `built-ins/Error` | 93 | 44 | 47.3 % |
| `built-ins/FinalizationRegistry` | 47 | 0 | 0.0 % |
| `built-ins/Function` | 509 | 301 | 59.1 % |
| `built-ins/GeneratorFunction` | 23 | 5 | 21.7 % |
| `built-ins/GeneratorPrototype` | 61 | 42 | 68.9 % |
| `built-ins/Infinity` | 6 | 2 | 33.3 % |
| `built-ins/Iterator` | 684 | 13 | 1.9 % |
| `built-ins/JSON` | 166 | 96 | 57.8 % |
| `built-ins/Map` | 204 | 133 | 65.2 % |
| `built-ins/MapIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/Math` | 327 | 258 | 78.9 % |
| `built-ins/NaN` | 6 | 2 | 33.3 % |
| `built-ins/NativeErrors` | 94 | 82 | 87.2 % |
| `built-ins/Number` | 340 | 295 | 86.8 % |
| `built-ins/Object` | 3411 | 2530 | 74.2 % |
| `built-ins/Promise` | 732 | 375 | 51.2 % |
| `built-ins/Proxy` | 311 | 0 | 0.0 % |
| `built-ins/Reflect` | 153 | 130 | 85.0 % |
| `built-ins/RegExp` | 1879 | 708 | 37.7 % |
| `built-ins/RegExpStringIteratorPrototype` | 17 | 0 | 0.0 % |
| `built-ins/Set` | 383 | 204 | 53.3 % |
| `built-ins/SetIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/ShadowRealm` | 64 | 0 | 0.0 % |
| `built-ins/SharedArrayBuffer` | 104 | 0 | 0.0 % |
| `built-ins/String` | 1223 | 1092 | 89.3 % |
| `built-ins/StringIteratorPrototype` | 7 | 5 | 71.4 % |
| `built-ins/SuppressedError` | 22 | 0 | 0.0 % |
| `built-ins/Symbol` | 98 | 49 | 50.0 % |
| `built-ins/Temporal` | 4614 | 0 | 0.0 % |
| `built-ins/ThrowTypeError` | 14 | 0 | 0.0 % |
| `built-ins/TypedArray` | 1453 | 1197 | 82.4 % |
| `built-ins/TypedArrayConstructors` | 738 | 538 | 72.9 % |
| `built-ins/Uint8Array` | 70 | 8 | 11.4 % |
| `built-ins/WeakMap` | 141 | 101 | 71.6 % |
| `built-ins/WeakRef` | 29 | 0 | 0.0 % |
| `built-ins/WeakSet` | 85 | 73 | 85.9 % |
| `built-ins/decodeURI` | 55 | 47 | 85.5 % |
| `built-ins/decodeURIComponent` | 56 | 53 | 94.6 % |
| `built-ins/encodeURI` | 31 | 25 | 80.6 % |
| `built-ins/encodeURIComponent` | 31 | 25 | 80.6 % |
| `built-ins/eval` | 10 | 8 | 80.0 % |
| `built-ins/global` | 29 | 27 | 93.1 % |
| `built-ins/isFinite` | 15 | 15 | 100.0 % |
| `built-ins/isNaN` | 15 | 15 | 100.0 % |
| `built-ins/parseFloat` | 54 | 54 | 100.0 % |
| `built-ins/parseInt` | 55 | 55 | 100.0 % |
| `built-ins/undefined` | 8 | 3 | 37.5 % |
| `language/arguments-object` | 263 | 176 | 66.9 % |
| `language/asi` | 102 | 102 | 100.0 % |
| `language/block-scope` | 145 | 48 | 33.1 % |
| `language/comments` | 52 | 43 | 82.7 % |
| `language/computed-property-names` | 48 | 40 | 83.3 % |
| `language/destructuring` | 19 | 16 | 84.2 % |
| `language/directive-prologue` | 62 | 28 | 45.2 % |
| `language/eval-code` | 347 | 132 | 38.0 % |
| `language/export` | 3 | 3 | 100.0 % |
| `language/expressions` | 11156 | 7015 | 62.9 % |
| `language/function-code` | 217 | 160 | 73.7 % |
| `language/future-reserved-words` | 55 | 39 | 70.9 % |
| `language/global-code` | 42 | 15 | 35.7 % |
| `language/identifier-resolution` | 14 | 8 | 57.1 % |
| `language/identifiers` | 268 | 154 | 57.5 % |
| `language/import` | 129 | 14 | 10.9 % |
| `language/keywords` | 25 | 25 | 100.0 % |
| `language/line-terminators` | 41 | 34 | 82.9 % |
| `language/literals` | 536 | 294 | 54.9 % |
| `language/module-code` | 599 | 246 | 41.1 % |
| `language/punctuators` | 11 | 11 | 100.0 % |
| `language/reserved-words` | 27 | 26 | 96.3 % |
| `language/rest-parameters` | 11 | 11 | 100.0 % |
| `language/source-text` | 1 | 1 | 100.0 % |
| `language/statementList` | 80 | 80 | 100.0 % |
| `language/statements` | 9399 | 6093 | 64.8 % |
| `language/types` | 113 | 98 | 86.7 % |
| `language/white-space` | 67 | 67 | 100.0 % |
