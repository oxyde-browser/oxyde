# Résultats test262

Suite [tc39/test262](https://github.com/tc39/test262), révision `102924a26277`, dossiers `language`, `built-ins` et `annexB`. Généré par `cargo run -p oxyde_test262 --release -- --publier`.

Un test n'est réussi que s'il passe dans tous ses modes (strict et non strict). Aucun test n'est écarté : ceux qui demandent une fonctionnalité absente échouent.

**Total : 48128 réussis sur 48795 (98.6 %).**

| Dossier | Tests | Réussis | Taux |
|---|---:|---:|---:|
| `annexB/built-ins` | 241 | 232 | 96.3 % |
| `annexB/language` | 845 | 815 | 96.4 % |
| `built-ins/AbstractModuleSource` | 8 | 0 | 0.0 % |
| `built-ins/AggregateError` | 25 | 25 | 100.0 % |
| `built-ins/Array` | 3083 | 3033 | 98.4 % |
| `built-ins/ArrayBuffer` | 221 | 221 | 100.0 % |
| `built-ins/ArrayIteratorPrototype` | 27 | 24 | 88.9 % |
| `built-ins/AsyncDisposableStack` | 104 | 104 | 100.0 % |
| `built-ins/AsyncFromSyncIteratorPrototype` | 38 | 32 | 84.2 % |
| `built-ins/AsyncFunction` | 18 | 18 | 100.0 % |
| `built-ins/AsyncGeneratorFunction` | 23 | 22 | 95.7 % |
| `built-ins/AsyncGeneratorPrototype` | 48 | 45 | 93.8 % |
| `built-ins/AsyncIteratorPrototype` | 13 | 13 | 100.0 % |
| `built-ins/Atomics` | 389 | 389 | 100.0 % |
| `built-ins/BigInt` | 77 | 76 | 98.7 % |
| `built-ins/Boolean` | 51 | 51 | 100.0 % |
| `built-ins/DataView` | 560 | 560 | 100.0 % |
| `built-ins/Date` | 610 | 595 | 97.5 % |
| `built-ins/DisposableStack` | 93 | 93 | 100.0 % |
| `built-ins/Error` | 93 | 93 | 100.0 % |
| `built-ins/FinalizationRegistry` | 47 | 47 | 100.0 % |
| `built-ins/Function` | 509 | 504 | 99.0 % |
| `built-ins/GeneratorFunction` | 23 | 22 | 95.7 % |
| `built-ins/GeneratorPrototype` | 61 | 61 | 100.0 % |
| `built-ins/Infinity` | 6 | 6 | 100.0 % |
| `built-ins/Iterator` | 684 | 681 | 99.6 % |
| `built-ins/JSON` | 166 | 161 | 97.0 % |
| `built-ins/Map` | 204 | 199 | 97.5 % |
| `built-ins/MapIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/Math` | 327 | 314 | 96.0 % |
| `built-ins/NaN` | 6 | 6 | 100.0 % |
| `built-ins/NativeErrors` | 94 | 88 | 93.6 % |
| `built-ins/Number` | 340 | 338 | 99.4 % |
| `built-ins/Object` | 3411 | 3386 | 99.3 % |
| `built-ins/Promise` | 732 | 732 | 100.0 % |
| `built-ins/Proxy` | 311 | 310 | 99.7 % |
| `built-ins/Reflect` | 153 | 153 | 100.0 % |
| `built-ins/RegExp` | 1879 | 1764 | 93.9 % |
| `built-ins/RegExpStringIteratorPrototype` | 17 | 17 | 100.0 % |
| `built-ins/Set` | 383 | 378 | 98.7 % |
| `built-ins/SetIteratorPrototype` | 11 | 9 | 81.8 % |
| `built-ins/ShadowRealm` | 64 | 64 | 100.0 % |
| `built-ins/SharedArrayBuffer` | 104 | 104 | 100.0 % |
| `built-ins/String` | 1223 | 1186 | 97.0 % |
| `built-ins/StringIteratorPrototype` | 7 | 5 | 71.4 % |
| `built-ins/SuppressedError` | 22 | 22 | 100.0 % |
| `built-ins/Symbol` | 98 | 97 | 99.0 % |
| `built-ins/Temporal` | 4614 | 4614 | 100.0 % |
| `built-ins/ThrowTypeError` | 14 | 0 | 0.0 % |
| `built-ins/TypedArray` | 1453 | 1453 | 100.0 % |
| `built-ins/TypedArrayConstructors` | 738 | 738 | 100.0 % |
| `built-ins/Uint8Array` | 70 | 70 | 100.0 % |
| `built-ins/WeakMap` | 141 | 141 | 100.0 % |
| `built-ins/WeakRef` | 29 | 29 | 100.0 % |
| `built-ins/WeakSet` | 85 | 85 | 100.0 % |
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
| `language/expressions` | 11156 | 11022 | 98.8 % |
| `language/function-code` | 217 | 217 | 100.0 % |
| `language/future-reserved-words` | 55 | 55 | 100.0 % |
| `language/global-code` | 42 | 42 | 100.0 % |
| `language/identifier-resolution` | 14 | 13 | 92.9 % |
| `language/identifiers` | 268 | 268 | 100.0 % |
| `language/import` | 129 | 120 | 93.0 % |
| `language/keywords` | 25 | 25 | 100.0 % |
| `language/line-terminators` | 41 | 34 | 82.9 % |
| `language/literals` | 536 | 535 | 99.8 % |
| `language/module-code` | 599 | 568 | 94.8 % |
| `language/punctuators` | 11 | 11 | 100.0 % |
| `language/reserved-words` | 27 | 27 | 100.0 % |
| `language/rest-parameters` | 11 | 11 | 100.0 % |
| `language/source-text` | 1 | 1 | 100.0 % |
| `language/statementList` | 80 | 80 | 100.0 % |
| `language/statements` | 9399 | 9317 | 99.1 % |
| `language/types` | 113 | 113 | 100.0 % |
| `language/white-space` | 67 | 67 | 100.0 % |
