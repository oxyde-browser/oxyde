// Observables (proposition WICG « Observable ») : `Observable`, `Subscriber`,
// les opérateurs (`map`, `filter`, `take`, `drop`, `takeUntil`, `flatMap`,
// `switchMap`, `inspect`, `catch`, `finally`), les opérateurs à promesse
// (`toArray`, `forEach`, `every`, `some`, `find`, `first`, `last`, `reduce`),
// `Observable.from()` et `EventTarget.prototype.when()`.
//
// Le script rend une fabrique, appelée avec les algorithmes d'abandon du
// prélude : `add(signal, algorithme)` et `remove(signal, algorithme)`.
//
// LIMITE CONNUE : pas de contrôle « document pleinement actif » ; un
// observable d'un cadre détaché continue d'émettre.
(function (abort) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const report = global.reportError;
  const AbortController = global.AbortController;
  const AbortSignal = global.AbortSignal;
  const addListener = global.EventTarget.prototype.addEventListener;
  const key = {};
  const subscribers = new WeakMap();
  const observables = new WeakMap();
  const isObject = (value) => value !== null && (typeof value === "object" || typeof value === "function");
  const noop = () => {};

  // ---- Subscriber ----------------------------------------------------------------------------------------------
  const stateOf = (subscriber) => {
    const state = subscribers.get(subscriber);
    if (state === undefined) throw new TypeError("un Subscriber est attendu");
    return state;
  };
  // Ferme l'abonnement : le signal est abandonné, puis les fonctions de
  // nettoyage s'exécutent, de la dernière posée à la première.
  function close(state, reason) {
    if (!state.active) return;
    state.active = false;
    state.controller.abort(reason);
    for (let i = state.teardowns.length - 1; i >= 0; i--) {
      try {
        state.teardowns[i]();
      } catch (error) {
        report(error);
      }
    }
    state.teardowns.length = 0;
  }
  class Subscriber {
    constructor(guard) {
      if (guard !== key) throw new TypeError("Subscriber : constructeur illégal");
      subscribers.set(this, {
        active: true, observers: [], teardowns: [], controller: new AbortController(),
      });
    }
    next(value) {
      const state = stateOf(this);
      if (arguments.length === 0) throw new TypeError("next : un argument est attendu");
      if (!state.active) return;
      for (const observer of [...state.observers]) observer.next(value);
    }
    error(error) {
      const state = stateOf(this);
      if (arguments.length === 0) throw new TypeError("error : un argument est attendu");
      if (!state.active) {
        report(error);
        return;
      }
      close(state, error);
      for (const observer of [...state.observers]) observer.error(error);
    }
    complete() {
      const state = stateOf(this);
      if (!state.active) return;
      close(state);
      for (const observer of [...state.observers]) observer.complete();
    }
    addTeardown(teardown) {
      const state = stateOf(this);
      if (typeof teardown !== "function") throw new TypeError("addTeardown : une fonction est attendue");
      if (state.active) {
        state.teardowns.push(teardown);
        return;
      }
      try {
        teardown();
      } catch (error) {
        report(error);
      }
    }
    get active() { return stateOf(this).active; }
    get signal() { return stateOf(this).controller.signal; }
  }

  // ---- Abonnement ----------------------------------------------------------------------------------------------
  const callable = (value, name) => {
    if (value !== undefined && typeof value !== "function") {
      throw new TypeError(name + " : une fonction est attendue");
    }
    return value;
  };
  const guarded = (callback) => (value) => {
    try {
      callback(value);
    } catch (error) {
      report(error);
    }
  };
  // Observateur interne tiré de l'argument de `subscribe()` : une fonction
  // (`next`) ou un dictionnaire `{ next, error, complete }`.
  function internalObserver(observer) {
    let next;
    let error;
    let complete;
    if (typeof observer === "function") next = observer;
    else if (observer !== undefined && observer !== null) {
      if (!isObject(observer)) throw new TypeError("subscribe : un observateur est attendu");
      complete = callable(observer.complete, "complete");
      error = callable(observer.error, "error");
      next = callable(observer.next, "next");
    }
    return {
      next: next === undefined ? noop : guarded(next),
      error: error === undefined ? (reason) => report(reason) : guarded(error),
      complete: complete === undefined ? noop : guarded(() => complete()),
    };
  }
  const signalOf = (options) => {
    if (options === undefined || options === null) return undefined;
    if (!isObject(options)) throw new TypeError("les options sont un dictionnaire");
    const signal = options.signal;
    if (signal !== undefined && !(signal instanceof AbortSignal)) {
      throw new TypeError("signal : un AbortSignal est attendu");
    }
    return signal;
  };
  // Abonne l'observateur interne `observer` à `observable`. Tant qu'un
  // abonné est actif, les abonnements suivants le partagent : le producteur
  // ne tourne qu'une fois, et s'arrête au départ du dernier observateur.
  function subscribeTo(observable, observer, signal) {
    const source = observables.get(observable);
    const leave = (state) => () => {
      if (!state.active) return;
      const at = state.observers.indexOf(observer);
      if (at >= 0) state.observers.splice(at, 1);
      if (state.observers.length === 0) close(state, signal.reason);
    };
    if (source.subscriber !== null && subscribers.get(source.subscriber).active) {
      const state = subscribers.get(source.subscriber);
      state.observers.push(observer);
      if (signal !== undefined) {
        if (signal.aborted) state.observers.pop();
        else abort.add(signal, leave(state));
      }
      return;
    }
    const subscriber = new Subscriber(key);
    const state = subscribers.get(subscriber);
    state.observers.push(observer);
    source.subscriber = subscriber;
    if (signal !== undefined) {
      if (signal.aborted) close(state, signal.reason);
      else abort.add(signal, leave(state));
    }
    if (source.steps !== null) {
      source.steps(subscriber);
      return;
    }
    try {
      source.callback.call(undefined, subscriber);
    } catch (error) {
      subscriber.error(error);
    }
  }
  const thisObservable = (value) => {
    if (!observables.has(value)) throw new TypeError("un Observable est attendu");
    return value;
  };
  // Observable dont l'abonnement exécute `steps(subscriber)`.
  const derived = (steps) => {
    const observable = Object.create(Observable.prototype);
    observables.set(observable, { callback: null, steps, subscriber: null });
    return observable;
  };
  const fn = (value, name) => {
    if (typeof value !== "function") throw new TypeError(name + " : une fonction est attendue");
    return value;
  };
  // Entier « unsigned long long » de Web IDL, sans contrôle d'intervalle.
  const count = (value) => {
    let n = Math.trunc(Number(value));
    if (!Number.isFinite(n)) n = 0;
    return n < 0 ? 2 ** 64 + n : n;
  };

  // ---- Itérateurs (Observable.from) ------------------------------------------------------------------------------
  const getMethod = (value, name) => {
    const method = value[name];
    if (method === undefined || method === null) return undefined;
    if (typeof method !== "function") throw new TypeError("la méthode d'itération n'est pas une fonction");
    return method;
  };
  const openIterator = (value, method) => {
    const iterator = method.call(value);
    if (!isObject(iterator)) throw new TypeError("l'itérateur n'est pas un objet");
    return { iterator, next: iterator.next };
  };
  // Itérateur asynchrone posé sur un itérateur synchrone.
  const asyncFromSync = (record) => {
    const settle = (result) => {
      if (!isObject(result)) throw new TypeError("le résultat d'itération n'est pas un objet");
      const done = Boolean(result.done);
      return Promise.resolve(result.value).then((value) => ({ value, done }));
    };
    const iterator = {
      next() {
        try {
          return settle(record.next.call(record.iterator));
        } catch (error) {
          return Promise.reject(error);
        }
      },
      return() {
        try {
          const method = getMethod(record.iterator, "return");
          if (method === undefined) return Promise.resolve({ value: undefined, done: true });
          return settle(method.call(record.iterator));
        } catch (error) {
          return Promise.reject(error);
        }
      },
    };
    return { iterator, next: iterator.next };
  };
  function getIterator(value, isAsync) {
    if (!isAsync) {
      const method = getMethod(value, Symbol.iterator);
      if (method === undefined) throw new TypeError("la valeur n'est pas itérable");
      return openIterator(value, method);
    }
    const method = getMethod(value, Symbol.asyncIterator);
    if (method !== undefined) {
      const iterator = method.call(value);
      if (!isObject(iterator)) throw new TypeError("l'itérateur n'est pas un objet");
      // Une erreur à la lecture de `next` attend le premier pas.
      try {
        return { iterator, next: iterator.next };
      } catch (error) {
        return { iterator, next: undefined, failure: { error } };
      }
    }
    const fallback = getMethod(value, Symbol.iterator);
    if (fallback === undefined) throw new TypeError("la valeur n'est pas itérable");
    return asyncFromSync(openIterator(value, fallback));
  }
  function fromIterable(value) {
    return derived((subscriber) => {
      const signal = subscriber.signal;
      if (signal.aborted) return;
      let record;
      try {
        record = getIterator(value, false);
      } catch (error) {
        subscriber.error(error);
        return;
      }
      if (signal.aborted) return;
      // Abandon en cours d'itération : `return()` de l'itérateur ; son
      // erreur remonte à qui abandonne. Rien après la fin de l'itération.
      let finished = false;
      abort.add(signal, () => {
        if (finished) return;
        const method = getMethod(record.iterator, "return");
        if (method === undefined) return;
        const result = method.call(record.iterator, signal.reason);
        if (!isObject(result)) throw new TypeError("return() doit rendre un Object");
      });
      for (;;) {
        let done;
        let item;
        try {
          const result = record.next.call(record.iterator);
          if (!isObject(result)) throw new TypeError("le résultat d'itération n'est pas un objet");
          done = Boolean(result.done);
          if (!done) item = result.value;
        } catch (error) {
          finished = true;
          subscriber.error(error);
          return;
        }
        if (done) {
          finished = true;
          subscriber.complete();
          return;
        }
        subscriber.next(item);
        if (signal.aborted) return;
      }
    });
  }
  function fromAsyncIterable(value) {
    return derived((subscriber) => {
      const signal = subscriber.signal;
      if (signal.aborted) return;
      let record;
      try {
        record = getIterator(value, true);
      } catch (error) {
        subscriber.error(error);
        return;
      }
      if (signal.aborted) return;
      // Abandon en cours d'itération : `return()` de l'itérateur ; son
      // erreur devient une promesse rejetée, que personne ne rattrape.
      let finished = false;
      abort.add(signal, () => {
        if (finished) return;
        let result;
        try {
          const method = getMethod(record.iterator, "return");
          if (method === undefined) return;
          result = method.call(record.iterator, signal.reason);
        } catch (error) {
          Promise.reject(error);
          return;
        }
        Promise.resolve(result).then((settled) => {
          if (!isObject(settled)) throw new TypeError("return() doit rendre un Object");
        });
      });
      const step = () => {
        if (signal.aborted) return;
        let pending;
        try {
          if (record.failure !== undefined) throw record.failure.error;
          pending = Promise.resolve(record.next.call(record.iterator));
        } catch (error) {
          pending = Promise.reject(error);
        }
        pending.then((result) => {
          let done;
          let item;
          try {
            if (!isObject(result)) throw new TypeError("le résultat d'itération n'est pas un objet");
            done = Boolean(result.done);
            if (!done) item = result.value;
          } catch (error) {
            finished = true;
            subscriber.error(error);
            return;
          }
          if (done) {
            finished = true;
            subscriber.complete();
            return;
          }
          subscriber.next(item);
          step();
        }, (error) => {
          finished = true;
          subscriber.error(error);
        });
      };
      step();
    });
  }
  function from(value) {
    if (!isObject(value)) throw new TypeError("Observable.from : un objet est attendu");
    if (observables.has(value)) return value;
    if (getMethod(value, Symbol.asyncIterator) !== undefined) return fromAsyncIterable(value);
    if (getMethod(value, Symbol.iterator) !== undefined) return fromIterable(value);
    if (value instanceof Promise) {
      return derived((subscriber) => {
        value.then((result) => {
          subscriber.next(result);
          subscriber.complete();
        }, (error) => subscriber.error(error));
      });
    }
    throw new TypeError("Observable.from : la valeur n'est ni observable, ni itérable, ni une promesse");
  }

  // ---- Opérateurs à promesse -----------------------------------------------------------------------------------
  // Promesse d'un opérateur : `run(resolve, reject, stop)` rend l'observateur
  // interne ; `stop(raison)` met fin à l'abonnement.
  function promised(source, options, run, stops = true) {
    let signal;
    try {
      thisObservable(source);
      signal = signalOf(options);
    } catch (error) {
      return Promise.reject(error);
    }
    return new Promise((resolve, reject) => {
      const controller = new AbortController();
      // Sans arrêt interne, le signal de l'appelant sert tel quel :
      // l'abonnement se ferme avant ses propres écouteurs `abort`.
      let both = controller.signal;
      if (signal !== undefined) both = stops ? AbortSignal.any([controller.signal, signal]) : signal;
      if (both.aborted) {
        reject(both.reason);
        return;
      }
      abort.add(both, () => reject(both.reason));
      const observer = run(resolve, reject, (reason) => controller.abort(reason));
      subscribeTo(source, {
        next: observer.next ?? noop,
        error: observer.error ?? reject,
        complete: observer.complete ?? noop,
      }, both);
    });
  }
  // Opérateur qui interroge `predicate` sur chaque valeur.
  const testing = (source, predicate, options, name, onValue, onComplete) => {
    if (typeof predicate !== "function") {
      return Promise.reject(new TypeError(name + " : une fonction est attendue"));
    }
    return promised(source, options, (resolve, reject, stop) => {
      let index = 0;
      return {
        next(value) {
          let passed;
          try {
            passed = predicate(value, index);
          } catch (error) {
            reject(error);
            stop(error);
            return;
          }
          index += 1;
          onValue(Boolean(passed), value, resolve, stop);
        },
        complete() { onComplete(resolve); },
      };
    });
  };

  // ---- Observable ----------------------------------------------------------------------------------------------
  // Abonnement qui fait suivre erreurs et fin à `subscriber`.
  const forward = (subscriber, next) => ({
    next,
    error: (error) => subscriber.error(error),
    complete: () => subscriber.complete(),
  });
  class Observable {
    constructor(callback) {
      fn(callback, "Observable");
      observables.set(this, { callback, steps: null, subscriber: null });
    }
    subscribe(observer = {}, options = {}) {
      thisObservable(this);
      const internal = internalObserver(observer);
      subscribeTo(this, internal, signalOf(options));
    }
    static from(value) { return from(value); }
    takeUntil(value) {
      const source = thisObservable(this);
      const notifier = from(value);
      return derived((subscriber) => {
        subscribeTo(notifier, {
          next: () => subscriber.complete(),
          error: () => subscriber.complete(),
          complete: noop,
        }, subscriber.signal);
        if (!subscriber.active) return;
        subscribeTo(source, forward(subscriber, (item) => subscriber.next(item)), subscriber.signal);
      });
    }
    map(mapper) {
      const source = thisObservable(this);
      fn(mapper, "map");
      return derived((subscriber) => {
        let index = 0;
        subscribeTo(source, forward(subscriber, (value) => {
          let mapped;
          try {
            mapped = mapper(value, index);
          } catch (error) {
            subscriber.error(error);
            return;
          }
          index += 1;
          subscriber.next(mapped);
        }), subscriber.signal);
      });
    }
    filter(predicate) {
      const source = thisObservable(this);
      fn(predicate, "filter");
      return derived((subscriber) => {
        let index = 0;
        subscribeTo(source, forward(subscriber, (value) => {
          let matches;
          try {
            matches = predicate(value, index);
          } catch (error) {
            subscriber.error(error);
            return;
          }
          index += 1;
          if (matches) subscriber.next(value);
        }), subscriber.signal);
      });
    }
    take(amount) {
      const source = thisObservable(this);
      const total = count(amount);
      return derived((subscriber) => {
        let remaining = total;
        if (remaining === 0) {
          subscriber.complete();
          return;
        }
        subscribeTo(source, forward(subscriber, (value) => {
          subscriber.next(value);
          remaining -= 1;
          if (remaining === 0) subscriber.complete();
        }), subscriber.signal);
      });
    }
    drop(amount) {
      const source = thisObservable(this);
      const total = count(amount);
      return derived((subscriber) => {
        let remaining = total;
        subscribeTo(source, forward(subscriber, (value) => {
          if (remaining > 0) {
            remaining -= 1;
            return;
          }
          subscriber.next(value);
        }), subscriber.signal);
      });
    }
    flatMap(mapper) {
      const source = thisObservable(this);
      fn(mapper, "flatMap");
      return derived((subscriber) => {
        let index = 0;
        let outerDone = false;
        let innerActive = false;
        const queue = [];
        const process = (value) => {
          let inner;
          try {
            inner = from(mapper(value, index));
          } catch (error) {
            subscriber.error(error);
            return;
          }
          index += 1;
          subscribeTo(inner, {
            next: (item) => subscriber.next(item),
            error: (error) => subscriber.error(error),
            complete: () => {
              if (queue.length > 0) {
                process(queue.shift());
                return;
              }
              innerActive = false;
              if (outerDone) subscriber.complete();
            },
          }, subscriber.signal);
        };
        subscribeTo(source, {
          next: (value) => {
            if (innerActive) {
              queue.push(value);
              return;
            }
            innerActive = true;
            process(value);
          },
          error: (error) => subscriber.error(error),
          complete: () => {
            outerDone = true;
            if (!innerActive && queue.length === 0) subscriber.complete();
          },
        }, subscriber.signal);
      });
    }
    switchMap(mapper) {
      const source = thisObservable(this);
      fn(mapper, "switchMap");
      return derived((subscriber) => {
        let index = 0;
        let outerDone = false;
        let inner = null;
        subscribeTo(source, {
          next: (value) => {
            if (inner !== null) inner.abort();
            inner = new AbortController();
            const mine = inner;
            let mapped;
            try {
              mapped = from(mapper(value, index));
            } catch (error) {
              subscriber.error(error);
              return;
            }
            index += 1;
            subscribeTo(mapped, {
              next: (item) => subscriber.next(item),
              error: (error) => subscriber.error(error),
              complete: () => {
                if (outerDone) subscriber.complete();
                else if (inner === mine) inner = null;
              },
            }, AbortSignal.any([mine.signal, subscriber.signal]));
          },
          error: (error) => subscriber.error(error),
          complete: () => {
            outerDone = true;
            if (inner === null) subscriber.complete();
          },
        }, subscriber.signal);
      });
    }
    inspect(inspector = {}) {
      const source = thisObservable(this);
      let hooks = {};
      if (typeof inspector === "function") hooks.next = inspector;
      else if (inspector !== undefined && inspector !== null) {
        if (!isObject(inspector)) throw new TypeError("inspect : un dictionnaire est attendu");
        hooks = {
          abort: callable(inspector.abort, "abort"),
          complete: callable(inspector.complete, "complete"),
          error: callable(inspector.error, "error"),
          next: callable(inspector.next, "next"),
          subscribe: callable(inspector.subscribe, "subscribe"),
        };
      }
      return derived((subscriber) => {
        if (hooks.subscribe !== undefined) {
          try {
            hooks.subscribe();
          } catch (error) {
            subscriber.error(error);
            return;
          }
        }
        const signal = subscriber.signal;
        const aborted = () => {
          try {
            hooks.abort(signal.reason);
          } catch (error) {
            report(error);
          }
        };
        if (hooks.abort !== undefined) abort.add(signal, aborted);
        subscribeTo(source, {
          next: (value) => {
            if (hooks.next !== undefined) {
              try {
                hooks.next(value);
              } catch (error) {
                abort.remove(signal, aborted);
                subscriber.error(error);
                return;
              }
            }
            subscriber.next(value);
          },
          error: (error) => {
            abort.remove(signal, aborted);
            if (hooks.error !== undefined) {
              try {
                hooks.error(error);
              } catch (thrown) {
                subscriber.error(thrown);
                return;
              }
            }
            subscriber.error(error);
          },
          complete: () => {
            abort.remove(signal, aborted);
            if (hooks.complete !== undefined) {
              try {
                hooks.complete();
              } catch (error) {
                subscriber.error(error);
                return;
              }
            }
            subscriber.complete();
          },
        }, signal);
      });
    }
    catch(callback) {
      const source = thisObservable(this);
      fn(callback, "catch");
      return derived((subscriber) => {
        subscribeTo(source, {
          next: (value) => subscriber.next(value),
          complete: () => subscriber.complete(),
          error: (error) => {
            let inner;
            try {
              inner = from(callback(error));
            } catch (thrown) {
              subscriber.error(thrown);
              return;
            }
            subscribeTo(inner, forward(subscriber, (item) => subscriber.next(item)), subscriber.signal);
          },
        }, subscriber.signal);
      });
    }
    finally(callback) {
      const source = thisObservable(this);
      fn(callback, "finally");
      return derived((subscriber) => {
        subscriber.addTeardown(() => callback());
        subscribeTo(source, forward(subscriber, (item) => subscriber.next(item)), subscriber.signal);
      });
    }
    toArray(options = {}) {
      return promised(this, options, (resolve) => {
        const values = [];
        return {
          next(value) { values.push(value); },
          complete() { resolve(values); },
        };
      }, false);
    }
    forEach(callback, options = {}) {
      return testing(this, callback, options, "forEach", noop, (resolve) => resolve(undefined));
    }
    every(predicate, options = {}) {
      return testing(this, predicate, options, "every", (passed, value, resolve, stop) => {
        if (!passed) {
          resolve(false);
          stop();
        }
      }, (resolve) => resolve(true));
    }
    some(predicate, options = {}) {
      return testing(this, predicate, options, "some", (passed, value, resolve, stop) => {
        if (passed) {
          resolve(true);
          stop();
        }
      }, (resolve) => resolve(false));
    }
    find(predicate, options = {}) {
      return testing(this, predicate, options, "find", (passed, value, resolve, stop) => {
        if (passed) {
          resolve(value);
          stop();
        }
      }, (resolve) => resolve(undefined));
    }
    first(options = {}) {
      return promised(this, options, (resolve, reject, stop) => ({
        next(value) {
          resolve(value);
          stop();
        },
        complete() { reject(new RangeError("first : l'observable n'a émis aucune valeur")); },
      }));
    }
    last(options = {}) {
      return promised(this, options, (resolve, reject) => {
        let found = false;
        let latest;
        return {
          next(value) {
            found = true;
            latest = value;
          },
          complete() {
            if (found) resolve(latest);
            else reject(new RangeError("last : l'observable n'a émis aucune valeur"));
          },
        };
      }, false);
    }
    reduce(reducer, initialValue, options = {}) {
      if (typeof reducer !== "function") {
        return Promise.reject(new TypeError("reduce : une fonction est attendue"));
      }
      let seeded = arguments.length > 1;
      let accumulator = initialValue;
      return promised(this, options, (resolve, reject, stop) => {
        let index = 0;
        return {
          next(value) {
            if (!seeded) {
              seeded = true;
              accumulator = value;
              index += 1;
              return;
            }
            try {
              accumulator = reducer(accumulator, value, index);
            } catch (error) {
              reject(error);
              stop(error);
              return;
            }
            index += 1;
          },
          complete() {
            if (seeded) resolve(accumulator);
            else reject(new TypeError("reduce : observable vide, sans valeur initiale"));
          },
        };
      });
    }
  }

  for (const [name, value] of [["Observable", Observable], ["Subscriber", Subscriber]]) {
    define(value.prototype, Symbol.toStringTag, { value: name, configurable: true });
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  }

  // ---- EventTarget.when() ----------------------------------------------------------------------------------------
  define(global.EventTarget.prototype, "when", {
    value: function when(type, options = {}) {
      const target = this === undefined || this === null ? global : this;
      const kind = String(type);
      const capture = Boolean(options?.capture);
      const passive = options?.passive;
      return derived((subscriber) => {
        if (subscriber.signal.aborted) return;
        const init = { capture, signal: subscriber.signal };
        if (passive !== undefined) init.passive = Boolean(passive);
        addListener.call(target, kind, (event) => subscriber.next(event), init);
      });
    },
    writable: true, enumerable: true, configurable: true,
  });
})
