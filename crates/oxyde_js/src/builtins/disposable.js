// Gestion explicite des ressources (proposition « Explicit Resource
// Management ») : `DisposableStack`, `AsyncDisposableStack`, les méthodes de
// libération des itérateurs, et les opérations dont se servent `using` et
// `await using`. Écrit en JavaScript : la libération asynchrone est une
// fonction async, ce que le moteur sait déjà exécuter. Les intégrés utilisés
// sont capturés ici, avant tout script.
(function () {
  "use strict";
  const apply = Reflect.apply;
  const defineProperty = Reflect.defineProperty;
  const TypeErr = TypeError;
  const RefErr = ReferenceError;
  const Suppressed = SuppressedError;
  const PromiseCtor = Promise;
  const promiseResolve = Promise.resolve;
  const promiseThen = Promise.prototype.then;
  const disposeSymbol = Symbol.dispose;
  const asyncDisposeSymbol = Symbol.asyncDispose;
  const toStringTag = Symbol.toStringTag;

  // `GetMethod`.
  function getMethod(value, key) {
    const method = value[key];
    if (method === undefined || method === null) return undefined;
    if (typeof method !== "function") throw new TypeErr("la méthode de libération n'est pas une fonction");
    return method;
  }

  // Une pile de ressources est une liste chaînée, la plus récente en tête :
  // aucun script ne peut s'interposer dans ses lectures ni ses écritures.
  function newStack() {
    return { __proto__: null, top: null };
  }

  function push(stack, value, isAsync, method) {
    stack.top = { __proto__: null, value, isAsync, method, next: stack.top };
  }

  // `AddDisposableResource(pile, valeur, indication)`.
  function addResource(stack, value, isAsync) {
    if (value === null || value === undefined) {
      // `await using x = null` : rien à libérer, mais une attente est due.
      if (isAsync) push(stack, undefined, true, undefined);
      return;
    }
    if (typeof value !== "object" && typeof value !== "function") {
      throw new TypeErr("une ressource doit être un objet, null ou undefined");
    }
    let method;
    if (isAsync) {
      method = getMethod(value, asyncDisposeSymbol);
      if (method === undefined) {
        const sync = getMethod(value, disposeSymbol);
        if (sync !== undefined) {
          // Libération synchrone dans un contexte asynchrone : son résultat
          // est ignoré, son exception devient un rejet.
          method = function () {
            const self = this;
            return new PromiseCtor(function (resolve) {
              apply(sync, self, []);
              resolve(undefined);
            });
          };
        }
      }
    } else {
      method = getMethod(value, disposeSymbol);
    }
    if (method === undefined) throw new TypeErr("la ressource n'a pas de méthode de libération");
    push(stack, value, isAsync, method);
  }

  // `DisposeResources` quand aucune ressource n'est asynchrone. `hasError`
  // et `error` décrivent l'exception en cours, s'il y en a une.
  function disposeSync(stack, hasError, error) {
    let node = stack.top;
    stack.top = null;
    while (node !== null) {
      try {
        apply(node.method, node.value, []);
      } catch (raised) {
        if (hasError) {
          error = new Suppressed(raised, error);
        } else {
          error = raised;
          hasError = true;
        }
      }
      node = node.next;
    }
    if (hasError) throw error;
  }

  // `DisposeResources` en général : les libérations asynchrones sont attendues.
  async function disposeAsync(stack, hasError, error) {
    let needsAwait = false;
    let hasAwaited = false;
    let node = stack.top;
    stack.top = null;
    while (node !== null) {
      const method = node.method;
      if (!node.isAsync && needsAwait && !hasAwaited) {
        await undefined;
        needsAwait = false;
      }
      if (method !== undefined) {
        try {
          const result = apply(method, node.value, []);
          if (node.isAsync) {
            hasAwaited = true;
            await result;
          }
        } catch (raised) {
          if (hasError) {
            error = new Suppressed(raised, error);
          } else {
            error = raised;
            hasError = true;
          }
        }
      } else {
        needsAwait = true;
      }
      node = node.next;
    }
    if (needsAwait && !hasAwaited) await undefined;
    if (hasError) throw error;
  }

  class DisposableStack {
    #disposed = false;
    #stack = newStack();

    get disposed() {
      return this.#disposed;
    }

    dispose() {
      if (this.#disposed) return undefined;
      this.#disposed = true;
      return disposeSync(this.#stack, false, undefined);
    }

    use(value) {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      addResource(this.#stack, value, false);
      return value;
    }

    adopt(value, onDispose) {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      if (typeof onDispose !== "function") throw new TypeErr("une fonction de libération est attendue");
      push(this.#stack, undefined, false, function () {
        return apply(onDispose, undefined, [value]);
      });
      return value;
    }

    defer(onDispose) {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      if (typeof onDispose !== "function") throw new TypeErr("une fonction de libération est attendue");
      push(this.#stack, undefined, false, onDispose);
    }

    move() {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      const moved = new DisposableStack();
      moved.#stack = this.#stack;
      this.#stack = newStack();
      this.#disposed = true;
      return moved;
    }
  }

  class AsyncDisposableStack {
    #disposed = false;
    #stack = newStack();

    get disposed() {
      return this.#disposed;
    }

    disposeAsync() {
      // Un mauvais receveur donne une promesse rejetée, pas une exception.
      let disposed;
      try {
        disposed = this.#disposed;
      } catch (error) {
        return new PromiseCtor(function (resolve, reject) {
          reject(error);
        });
      }
      if (disposed) {
        return new PromiseCtor(function (resolve) {
          resolve(undefined);
        });
      }
      this.#disposed = true;
      return disposeAsync(this.#stack, false, undefined);
    }

    use(value) {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      addResource(this.#stack, value, true);
      return value;
    }

    adopt(value, onDisposeAsync) {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      if (typeof onDisposeAsync !== "function") throw new TypeErr("une fonction de libération est attendue");
      push(this.#stack, undefined, true, function () {
        return apply(onDisposeAsync, undefined, [value]);
      });
      return value;
    }

    defer(onDisposeAsync) {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      if (typeof onDisposeAsync !== "function") throw new TypeErr("une fonction de libération est attendue");
      push(this.#stack, undefined, true, onDisposeAsync);
    }

    move() {
      if (this.#disposed) throw new RefErr("la pile est déjà libérée");
      const moved = new AsyncDisposableStack();
      moved.#stack = this.#stack;
      this.#stack = newStack();
      this.#disposed = true;
      return moved;
    }
  }

  function hidden(target, key, value, writable) {
    defineProperty(target, key, { __proto__: null, value, writable, enumerable: false, configurable: true });
  }
  hidden(DisposableStack.prototype, disposeSymbol, DisposableStack.prototype.dispose, true);
  hidden(DisposableStack.prototype, toStringTag, "DisposableStack", false);
  hidden(AsyncDisposableStack.prototype, asyncDisposeSymbol, AsyncDisposableStack.prototype.disposeAsync, true);
  hidden(AsyncDisposableStack.prototype, toStringTag, "AsyncDisposableStack", false);

  // `%IteratorPrototype%[Symbol.dispose]` : ferme l'itérateur.
  const iteratorDispose = {
    [disposeSymbol]() {
      const method = getMethod(this, "return");
      if (method !== undefined) apply(method, this, []);
    },
  }[disposeSymbol];

  // `%AsyncIteratorPrototype%[Symbol.asyncDispose]` : ferme l'itérateur et
  // rend une promesse réglée quand c'est fait.
  const asyncIteratorDispose = {
    [asyncDisposeSymbol]() {
      const self = this;
      return new PromiseCtor(function (resolve, reject) {
        const method = getMethod(self, "return");
        if (method === undefined) {
          resolve(undefined);
          return;
        }
        const result = apply(method, self, []);
        const settled = apply(promiseResolve, PromiseCtor, [result]);
        apply(promiseThen, settled, [
          function () {
            resolve(undefined);
          },
          reject,
        ]);
      });
    },
  }[asyncDisposeSymbol];

  function descriptor(target, key) {
    return Object.getOwnPropertyDescriptor(target, key);
  }
  // Toutes les fonctions visibles des scripts : elles se montrent natives.
  const natives = [DisposableStack, AsyncDisposableStack, iteratorDispose, asyncIteratorDispose];
  for (const constructor of [DisposableStack, AsyncDisposableStack]) {
    const prototype = constructor.prototype;
    for (const name of ["use", "adopt", "defer", "move", "dispose", "disposeAsync"]) {
      const method = descriptor(prototype, name);
      if (method !== undefined) natives[natives.length] = method.value;
    }
    natives[natives.length] = descriptor(prototype, "disposed").get;
  }

  return {
    __proto__: null,
    DisposableStack,
    AsyncDisposableStack,
    iteratorDispose,
    asyncIteratorDispose,
    natives,
    newStack,
    addResource,
    disposeSync,
    disposeAsync,
  };
})();
