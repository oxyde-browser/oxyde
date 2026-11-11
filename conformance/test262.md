# Résultats test262

Suite [tc39/test262](https://github.com/tc39/test262), révision `102924a26277`, dossiers `language`, `built-ins` et `annexB`. Généré par `cargo run -p oxyde_test262 --release -- --publier`.

Un test n'est réussi que s'il passe dans tous ses modes (strict et non strict). Aucun test n'est écarté : ceux qui demandent une fonctionnalité absente échouent.

**Total : 37782 réussis sur 48795 (77.4 %).**

| Dossier | Tests | Réussis | Taux |
|---|---:|---:|---:|
| `annexB/built-ins` | 241 | 177 | 73.4 % |
| `annexB/language` | 845 | 769 | 91.0 % |
| `built-ins/AbstractModuleSource` | 8 | 0 | 0.0 % |
| `built-ins/AggregateError` | 25 | 0 | 0.0 % |
| `built-ins/Array` | 3083 | 2795 | 90.7 % |
| `built-ins/ArrayBuffer` | 221 | 164 | 74.2 % |
| `built-ins/ArrayIteratorPrototype` | 27 | 23 | 85.2 % |
| `built-ins/AsyncDisposableStack` | 104 | 0 | 0.0 % |
| `built-ins/AsyncFromSyncIteratorPrototype` | 38 | 10 | 26.3 % |
| `built-ins/AsyncFunction` | 18 | 10 | 55.6 % |
| `built-ins/AsyncGeneratorFunction` | 23 | 8 | 34.8 % |
| `built-ins/AsyncGeneratorPrototype` | 48 | 28 | 58.3 % |
| `built-ins/AsyncIteratorPrototype` | 13 | 0 | 0.0 % |
| `built-ins/Atomics` | 389 | 0 | 0.0 % |
| `built-ins/BigInt` | 77 | 74 | 96.1 % |
| `built-ins/Boolean` | 51 | 50 | 98.0 % |
| `built-ins/DataView` | 560 | 493 | 88.0 % |
| `built-ins/Date` | 610 | 583 | 95.6 % |
| `built-ins/DisposableStack` | 93 | 0 | 0.0 % |
| `built-ins/Error` | 93 | 45 | 48.4 % |
| `built-ins/FinalizationRegistry` | 47 | 0 | 0.0 % |
| `built-ins/Function` | 509 | 363 | 71.3 % |
| `built-ins/GeneratorFunction` | 23 | 8 | 34.8 % |
| `built-ins/GeneratorPrototype` | 61 | 47 | 77.0 % |
| `built-ins/Infinity` | 6 | 2 | 33.3 % |
| `built-ins/Iterator` | 684 | 462 | 67.5 % |
| `built-ins/JSON` | 166 | 101 | 60.8 % |
| `built-ins/Map` | 204 | 135 | 66.2 % |
| `built-ins/MapIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/Math` | 327 | 258 | 78.9 % |
| `built-ins/NaN` | 6 | 2 | 33.3 % |
| `built-ins/NativeErrors` | 94 | 82 | 87.2 % |
| `built-ins/Number` | 340 | 295 | 86.8 % |
| `built-ins/Object` | 3411 | 3328 | 97.6 % |
| `built-ins/Promise` | 732 | 383 | 52.3 % |
| `built-ins/Proxy` | 311 | 272 | 87.5 % |
| `built-ins/Reflect` | 153 | 152 | 99.3 % |
| `built-ins/RegExp` | 1879 | 1082 | 57.6 % |
| `built-ins/RegExpStringIteratorPrototype` | 17 | 17 | 100.0 % |
| `built-ins/Set` | 383 | 357 | 93.2 % |
| `built-ins/SetIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/ShadowRealm` | 64 | 0 | 0.0 % |
| `built-ins/SharedArrayBuffer` | 104 | 0 | 0.0 % |
| `built-ins/String` | 1223 | 1139 | 93.1 % |
| `built-ins/StringIteratorPrototype` | 7 | 5 | 71.4 % |
| `built-ins/SuppressedError` | 22 | 0 | 0.0 % |
| `built-ins/Symbol` | 98 | 67 | 68.4 % |
| `built-ins/Temporal` | 4614 | 0 | 0.0 % |
| `built-ins/ThrowTypeError` | 14 | 0 | 0.0 % |
| `built-ins/TypedArray` | 1453 | 1217 | 83.8 % |
| `built-ins/TypedArrayConstructors` | 738 | 633 | 85.8 % |
| `built-ins/Uint8Array` | 70 | 8 | 11.4 % |
| `built-ins/WeakMap` | 141 | 101 | 71.6 % |
| `built-ins/WeakRef` | 29 | 0 | 0.0 % |
| `built-ins/WeakSet` | 85 | 73 | 85.9 % |
| `built-ins/decodeURI` | 55 | 48 | 87.3 % |
| `built-ins/decodeURIComponent` | 56 | 54 | 96.4 % |
| `built-ins/encodeURI` | 31 | 26 | 83.9 % |
| `built-ins/encodeURIComponent` | 31 | 26 | 83.9 % |
| `built-ins/eval` | 10 | 10 | 100.0 % |
| `built-ins/global` | 29 | 27 | 93.1 % |
| `built-ins/isFinite` | 15 | 15 | 100.0 % |
| `built-ins/isNaN` | 15 | 15 | 100.0 % |
| `built-ins/parseFloat` | 54 | 54 | 100.0 % |
| `built-ins/parseInt` | 55 | 55 | 100.0 % |
| `built-ins/undefined` | 8 | 3 | 37.5 % |
| `language/arguments-object` | 263 | 232 | 88.2 % |
| `language/asi` | 102 | 102 | 100.0 % |
| `language/block-scope` | 145 | 129 | 89.0 % |
| `language/comments` | 52 | 52 | 100.0 % |
| `language/computed-property-names` | 48 | 44 | 91.7 % |
| `language/destructuring` | 19 | 16 | 84.2 % |
| `language/directive-prologue` | 62 | 62 | 100.0 % |
| `language/eval-code` | 347 | 142 | 40.9 % |
| `language/export` | 3 | 3 | 100.0 % |
| `language/expressions` | 11156 | 10201 | 91.4 % |
| `language/function-code` | 217 | 216 | 99.5 % |
| `language/future-reserved-words` | 55 | 55 | 100.0 % |
| `language/global-code` | 42 | 26 | 61.9 % |
| `language/identifier-resolution` | 14 | 13 | 92.9 % |
| `language/identifiers` | 268 | 208 | 77.6 % |
| `language/import` | 129 | 15 | 11.6 % |
| `language/keywords` | 25 | 25 | 100.0 % |
| `language/line-terminators` | 41 | 34 | 82.9 % |
| `language/literals` | 536 | 528 | 98.5 % |
| `language/module-code` | 599 | 531 | 88.6 % |
| `language/punctuators` | 11 | 11 | 100.0 % |
| `language/reserved-words` | 27 | 27 | 100.0 % |
| `language/rest-parameters` | 11 | 11 | 100.0 % |
| `language/source-text` | 1 | 1 | 100.0 % |
| `language/statementList` | 80 | 80 | 100.0 % |
| `language/statements` | 9399 | 8677 | 92.3 % |
| `language/types` | 113 | 105 | 92.9 % |
| `language/white-space` | 67 | 67 | 100.0 % |
