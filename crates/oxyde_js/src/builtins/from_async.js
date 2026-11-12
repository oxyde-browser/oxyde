// `Array.fromAsync` (ECMA-262, proposition « Array.fromAsync »), écrit en
// JavaScript : c'est une fonction async, ce que le moteur sait déjà exécuter.
// Les intégrés dont elle a besoin sont capturés ici, avant tout script.
(function (isConstructor, asyncFromSync) {
  "use strict";
  const apply = Reflect.apply;
  const defineProperty = Reflect.defineProperty;
  const ObjectCtor = Object;
  const ArrayCtor = Array;
  const TypeErr = TypeError;
  const trunc = Math.trunc;
  const iteratorSymbol = Symbol.iterator;
  const asyncIteratorSymbol = Symbol.asyncIterator;
  const MAX = 9007199254740991;

  function isObject(value) {
    return (typeof value === "object" && value !== null) || typeof value === "function";
  }

  // `GetMethod`.
  function method(value) {
    if (value === undefined || value === null) return undefined;
    if (typeof value !== "function") throw new TypeErr("la méthode d'itération n'est pas une fonction");
    return value;
  }

  // `CreateDataPropertyOrThrow`.
  function define(target, key, value) {
    const descriptor = { __proto__: null, value, writable: true, enumerable: true, configurable: true };
    if (!defineProperty(target, key, descriptor)) {
      throw new TypeErr("l'élément ne peut pas être défini");
    }
  }

  // `ToLength`.
  function toLength(value) {
    let n = +value;
    if (n !== n) return 0;
    n = trunc(n);
    if (n <= 0) return 0;
    return n > MAX ? MAX : n;
  }

  // `AsyncIteratorClose` sur une sortie par exception : `error` l'emporte.
  async function close(iterator, error) {
    try {
      const finish = method(iterator.return);
      if (finish !== undefined) await apply(finish, iterator, []);
    } catch {}
    throw error;
  }

  return async function fromAsync(asyncItems, mapper = undefined, thisArg = undefined) {
    const C = this;
    const mapping = mapper !== undefined;
    if (mapping && typeof mapper !== "function") {
      throw new TypeErr("Array.fromAsync : la fonction de transformation n'est pas appelable");
    }
    const usingAsync = method(asyncItems[asyncIteratorSymbol]);
    let iterator;
    if (usingAsync !== undefined) {
      iterator = apply(usingAsync, asyncItems, []);
      if (!isObject(iterator)) throw new TypeErr("Symbol.asyncIterator doit renvoyer un objet itérateur");
    } else {
      const usingSync = method(asyncItems[iteratorSymbol]);
      if (usingSync !== undefined) {
        const sync = apply(usingSync, asyncItems, []);
        if (!isObject(sync)) throw new TypeErr("Symbol.iterator doit renvoyer un objet itérateur");
        iterator = asyncFromSync(sync);
      }
    }

    if (iterator !== undefined) {
      const next = iterator.next;
      const A = isConstructor(C) ? new C() : [];
      let k = 0;
      for (;;) {
        const result = await apply(next, iterator, []);
        if (!isObject(result)) throw new TypeErr("le résultat de « next » doit être un objet");
        if (result.done) {
          A.length = k;
          return A;
        }
        let value = result.value;
        if (mapping) {
          try {
            value = await apply(mapper, thisArg, [value, k]);
          } catch (error) {
            await close(iterator, error);
          }
        }
        try {
          define(A, k, value);
        } catch (error) {
          await close(iterator, error);
        }
        k++;
      }
    }

    const arrayLike = ObjectCtor(asyncItems);
    const len = toLength(arrayLike.length);
    const A = isConstructor(C) ? new C(len) : new ArrayCtor(len);
    for (let k = 0; k < len; k++) {
      let value = await arrayLike[k];
      if (mapping) value = await apply(mapper, thisArg, [value, k]);
      define(A, k, value);
    }
    A.length = len;
    return A;
  };
})
