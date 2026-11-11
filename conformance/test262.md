# Résultats test262

Suite [tc39/test262](https://github.com/tc39/test262), révision `102924a26277`, dossiers `language`, `built-ins` et `annexB`. Généré par `cargo run -p oxyde_test262 --release -- --publier`.

Un test n'est réussi que s'il passe dans tous ses modes (strict et non strict). Aucun test n'est écarté : ceux qui demandent une fonctionnalité absente échouent.

**Total : 20076 réussis sur 48795 (41.1 %).**

| Dossier | Tests | Réussis | Taux |
|---|---:|---:|---:|
| `annexB/built-ins` | 241 | 30 | 12.4 % |
| `annexB/language` | 845 | 407 | 48.2 % |
| `built-ins/AbstractModuleSource` | 8 | 0 | 0.0 % |
| `built-ins/AggregateError` | 25 | 0 | 0.0 % |
| `built-ins/Array` | 3083 | 1159 | 37.6 % |
| `built-ins/ArrayBuffer` | 221 | 0 | 0.0 % |
| `built-ins/ArrayIteratorPrototype` | 27 | 14 | 51.9 % |
| `built-ins/AsyncDisposableStack` | 104 | 0 | 0.0 % |
| `built-ins/AsyncFromSyncIteratorPrototype` | 38 | 0 | 0.0 % |
| `built-ins/AsyncFunction` | 18 | 9 | 50.0 % |
| `built-ins/AsyncGeneratorFunction` | 23 | 0 | 0.0 % |
| `built-ins/AsyncGeneratorPrototype` | 48 | 0 | 0.0 % |
| `built-ins/AsyncIteratorPrototype` | 13 | 0 | 0.0 % |
| `built-ins/Atomics` | 389 | 0 | 0.0 % |
| `built-ins/BigInt` | 77 | 0 | 0.0 % |
| `built-ins/Boolean` | 51 | 49 | 96.1 % |
| `built-ins/DataView` | 560 | 0 | 0.0 % |
| `built-ins/Date` | 610 | 61 | 10.0 % |
| `built-ins/DisposableStack` | 93 | 0 | 0.0 % |
| `built-ins/Error` | 93 | 44 | 47.3 % |
| `built-ins/FinalizationRegistry` | 47 | 0 | 0.0 % |
| `built-ins/Function` | 509 | 298 | 58.5 % |
| `built-ins/GeneratorFunction` | 23 | 5 | 21.7 % |
| `built-ins/GeneratorPrototype` | 61 | 42 | 68.9 % |
| `built-ins/Infinity` | 6 | 2 | 33.3 % |
| `built-ins/Iterator` | 684 | 12 | 1.8 % |
| `built-ins/JSON` | 166 | 95 | 57.2 % |
| `built-ins/Map` | 204 | 131 | 64.2 % |
| `built-ins/MapIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/Math` | 327 | 258 | 78.9 % |
| `built-ins/NaN` | 6 | 2 | 33.3 % |
| `built-ins/NativeErrors` | 94 | 82 | 87.2 % |
| `built-ins/Number` | 340 | 291 | 85.6 % |
| `built-ins/Object` | 3411 | 2459 | 72.1 % |
| `built-ins/Promise` | 732 | 375 | 51.2 % |
| `built-ins/Proxy` | 311 | 0 | 0.0 % |
| `built-ins/Reflect` | 153 | 130 | 85.0 % |
| `built-ins/RegExp` | 1879 | 699 | 37.2 % |
| `built-ins/RegExpStringIteratorPrototype` | 17 | 0 | 0.0 % |
| `built-ins/Set` | 383 | 196 | 51.2 % |
| `built-ins/SetIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/ShadowRealm` | 64 | 0 | 0.0 % |
| `built-ins/SharedArrayBuffer` | 104 | 0 | 0.0 % |
| `built-ins/String` | 1223 | 1064 | 87.0 % |
| `built-ins/StringIteratorPrototype` | 7 | 5 | 71.4 % |
| `built-ins/SuppressedError` | 22 | 0 | 0.0 % |
| `built-ins/Symbol` | 98 | 49 | 50.0 % |
| `built-ins/Temporal` | 4614 | 0 | 0.0 % |
| `built-ins/ThrowTypeError` | 14 | 0 | 0.0 % |
| `built-ins/TypedArray` | 1453 | 0 | 0.0 % |
| `built-ins/TypedArrayConstructors` | 738 | 0 | 0.0 % |
| `built-ins/Uint8Array` | 70 | 0 | 0.0 % |
| `built-ins/WeakMap` | 141 | 101 | 71.6 % |
| `built-ins/WeakRef` | 29 | 0 | 0.0 % |
| `built-ins/WeakSet` | 85 | 73 | 85.9 % |
| `built-ins/decodeURI` | 55 | 47 | 85.5 % |
| `built-ins/decodeURIComponent` | 56 | 53 | 94.6 % |
| `built-ins/encodeURI` | 31 | 25 | 80.6 % |
| `built-ins/encodeURIComponent` | 31 | 25 | 80.6 % |
| `built-ins/eval` | 10 | 8 | 80.0 % |
| `built-ins/global` | 29 | 27 | 93.1 % |
| `built-ins/isFinite` | 15 | 9 | 60.0 % |
| `built-ins/isNaN` | 15 | 9 | 60.0 % |
| `built-ins/parseFloat` | 54 | 54 | 100.0 % |
| `built-ins/parseInt` | 55 | 55 | 100.0 % |
| `built-ins/undefined` | 8 | 3 | 37.5 % |
| `language/arguments-object` | 263 | 116 | 44.1 % |
| `language/asi` | 102 | 102 | 100.0 % |
| `language/block-scope` | 145 | 67 | 46.2 % |
| `language/comments` | 52 | 42 | 80.8 % |
| `language/computed-property-names` | 48 | 40 | 83.3 % |
| `language/destructuring` | 19 | 13 | 68.4 % |
| `language/directive-prologue` | 62 | 28 | 45.2 % |
| `language/eval-code` | 347 | 128 | 36.9 % |
| `language/export` | 3 | 0 | 0.0 % |
| `language/expressions` | 11156 | 5766 | 51.7 % |
| `language/function-code` | 217 | 160 | 73.7 % |
| `language/future-reserved-words` | 55 | 39 | 70.9 % |
| `language/global-code` | 42 | 15 | 35.7 % |
| `language/identifier-resolution` | 14 | 8 | 57.1 % |
| `language/identifiers` | 268 | 154 | 57.5 % |
| `language/import` | 129 | 6 | 4.7 % |
| `language/keywords` | 25 | 25 | 100.0 % |
| `language/line-terminators` | 41 | 34 | 82.9 % |
| `language/literals` | 536 | 287 | 53.5 % |
| `language/module-code` | 599 | 1 | 0.2 % |
| `language/punctuators` | 11 | 11 | 100.0 % |
| `language/reserved-words` | 27 | 26 | 96.3 % |
| `language/rest-parameters` | 11 | 11 | 100.0 % |
| `language/source-text` | 1 | 1 | 100.0 % |
| `language/statementList` | 80 | 80 | 100.0 % |
| `language/statements` | 9399 | 4276 | 45.5 % |
| `language/types` | 113 | 98 | 86.7 % |
| `language/white-space` | 67 | 67 | 100.0 % |
