// Flux (Streams Standard), en version réduite : `ReadableStream`,
// `WritableStream`, `TransformStream`, `TextEncoderStream`,
// `TextDecoderStream`.
//
// LIMITES CONNUES : pas de flux d'octets (`type: "bytes"`, lecteurs BYOB) ;
// pas de contre-pression (les stratégies de file sont acceptées et ignorées,
// `desiredSize` vaut 1 tant que le flux est ouvert) ; `pipeTo` ignore ses
// options ; `Response.body` n'est pas un flux.
(function () {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const expose = (name, value) => {
    define(value.prototype, Symbol.toStringTag, { value: name, configurable: true });
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };
  const deferred = () => {
    const out = {};
    out.promise = new Promise((resolve, reject) => {
      out.resolve = resolve;
      out.reject = reject;
    });
    // Une promesse d'état que personne ne lit ne doit pas signaler son rejet.
    out.promise.catch(() => {});
    return out;
  };

  // ---- ReadableStream ------------------------------------------------------------------------------------------
  const readable = new WeakMap();
  function pull(state) {
    if (state.pulling || state.status !== "readable" || typeof state.source.pull !== "function") return;
    if (state.queue.length > 0 && state.waiting.length === 0) return;
    state.pulling = true;
    Promise.resolve()
      .then(() => state.source.pull(state.controller))
      .then(() => {
        state.pulling = false;
        if (state.waiting.length > 0) pull(state);
      }, (reason) => fail(state, reason));
  }
  function fail(state, reason) {
    if (state.status !== "readable") return;
    state.status = "errored";
    state.reason = reason;
    state.queue.length = 0;
    for (const request of state.waiting.splice(0)) request.reject(reason);
    state.closed.reject(reason);
  }
  function finish(state) {
    state.status = "closed";
    for (const request of state.waiting.splice(0)) request.resolve({ value: undefined, done: true });
    state.closed.resolve(undefined);
  }
  class ReadableStreamDefaultController {
    #state;
    constructor(state) {
      this.#state = state;
    }
    get desiredSize() {
      const state = this.#state;
      return state.status === "errored" ? null : state.status === "closed" ? 0 : 1;
    }
    enqueue(chunk) {
      const state = this.#state;
      if (state.status !== "readable" || state.closing) {
        throw new TypeError("ReadableStream : le flux est fermé");
      }
      const request = state.waiting.shift();
      if (request !== undefined) request.resolve({ value: chunk, done: false });
      else state.queue.push(chunk);
    }
    close() {
      const state = this.#state;
      if (state.status !== "readable" || state.closing) {
        throw new TypeError("ReadableStream : le flux est déjà fermé");
      }
      state.closing = true;
      if (state.queue.length === 0) finish(state);
    }
    error(reason) {
      fail(this.#state, reason);
    }
  }
  class ReadableStreamDefaultReader {
    #state;
    constructor(stream) {
      const state = readable.get(stream);
      if (state === undefined) throw new TypeError("ReadableStreamDefaultReader : un flux est attendu");
      if (state.locked) throw new TypeError("ReadableStream : le flux a déjà un lecteur");
      state.locked = true;
      this.#state = state;
    }
    get closed() {
      return this.#state === null ? Promise.reject(new TypeError("lecteur libéré")) : this.#state.closed.promise;
    }
    read() {
      const state = this.#state;
      if (state === null) return Promise.reject(new TypeError("ReadableStream : lecteur libéré"));
      if (state.queue.length > 0) {
        const value = state.queue.shift();
        if (state.closing && state.queue.length === 0) finish(state);
        else pull(state);
        return Promise.resolve({ value, done: false });
      }
      if (state.status === "closed") return Promise.resolve({ value: undefined, done: true });
      if (state.status === "errored") return Promise.reject(state.reason);
      const request = deferred();
      state.waiting.push(request);
      pull(state);
      return request.promise;
    }
    cancel(reason) {
      const state = this.#state;
      if (state === null) return Promise.reject(new TypeError("ReadableStream : lecteur libéré"));
      return cancel(state, reason);
    }
    releaseLock() {
      if (this.#state !== null) this.#state.locked = false;
      this.#state = null;
    }
  }
  function cancel(state, reason) {
    if (state.status !== "readable") return Promise.resolve(undefined);
    state.queue.length = 0;
    finish(state);
    return Promise.resolve()
      .then(() => (typeof state.source.cancel === "function" ? state.source.cancel(reason) : undefined))
      .then(() => undefined);
  }
  class ReadableStream {
    constructor(source = {}, strategy = {}) {
      if (source === null || typeof source !== "object") throw new TypeError("ReadableStream : une source est attendue");
      if (source.type !== undefined) throw new RangeError("ReadableStream : les flux d'octets ne sont pas pris en charge");
      const state = {
        source, queue: [], waiting: [], status: "readable", closing: false, locked: false,
        pulling: false, reason: undefined, closed: deferred(), controller: null,
      };
      state.controller = new ReadableStreamDefaultController(state);
      readable.set(this, state);
      if (typeof source.start === "function") {
        try {
          Promise.resolve(source.start(state.controller)).then(() => pull(state), (reason) => fail(state, reason));
        } catch (reason) {
          fail(state, reason);
        }
      } else {
        Promise.resolve().then(() => pull(state));
      }
    }
    static from(iterable) {
      const iterator = iterable[Symbol.asyncIterator]?.() ?? iterable[Symbol.iterator]();
      return new ReadableStream({
        async pull(controller) {
          const { value, done } = await iterator.next();
          if (done) controller.close();
          else controller.enqueue(value);
        },
        cancel(reason) {
          return iterator.return?.(reason);
        },
      });
    }
    get locked() {
      return readable.get(this).locked;
    }
    getReader(options) {
      if (options?.mode !== undefined) throw new TypeError("ReadableStream : les lecteurs BYOB ne sont pas pris en charge");
      return new ReadableStreamDefaultReader(this);
    }
    cancel(reason) {
      const state = readable.get(this);
      if (state.locked) return Promise.reject(new TypeError("ReadableStream : le flux a un lecteur"));
      return cancel(state, reason);
    }
    tee() {
      const reader = this.getReader();
      const controllers = [];
      const branch = () => new ReadableStream({
        start(controller) {
          controllers.push(controller);
        },
      });
      const branches = [branch(), branch()];
      (async () => {
        try {
          for (;;) {
            const { value, done } = await reader.read();
            if (done) break;
            for (const controller of controllers) controller.enqueue(value);
          }
          for (const controller of controllers) controller.close();
        } catch (reason) {
          for (const controller of controllers) controller.error(reason);
        }
      })();
      return branches;
    }
    async pipeTo(destination) {
      const reader = this.getReader();
      const writer = destination.getWriter();
      try {
        for (;;) {
          const { value, done } = await reader.read();
          if (done) break;
          await writer.write(value);
        }
        await writer.close();
      } catch (reason) {
        await writer.abort(reason).catch(() => {});
        throw reason;
      } finally {
        reader.releaseLock();
      }
    }
    pipeThrough(transform) {
      this.pipeTo(transform.writable).catch(() => {});
      return transform.readable;
    }
    values() {
      const reader = this.getReader();
      return {
        next: () => reader.read(),
        return: async (value) => {
          await reader.cancel(value);
          return { value, done: true };
        },
        [Symbol.asyncIterator]() {
          return this;
        },
      };
    }
    [Symbol.asyncIterator]() {
      return this.values();
    }
  }
  expose("ReadableStream", ReadableStream);
  expose("ReadableStreamDefaultReader", ReadableStreamDefaultReader);
  expose("ReadableStreamDefaultController", ReadableStreamDefaultController);

  // ---- WritableStream ------------------------------------------------------------------------------------------
  const writable = new WeakMap();
  class WritableStreamDefaultWriter {
    #state;
    constructor(stream) {
      const state = writable.get(stream);
      if (state === undefined) throw new TypeError("WritableStreamDefaultWriter : un flux est attendu");
      if (state.locked) throw new TypeError("WritableStream : le flux a déjà un rédacteur");
      state.locked = true;
      this.#state = state;
    }
    get closed() { return this.#state.closed.promise; }
    get ready() { return Promise.resolve(undefined); }
    get desiredSize() { return this.#state.status === "writable" ? 1 : 0; }
    // Les écritures se suivent : chacune attend la précédente.
    write(chunk) {
      const state = this.#state;
      if (state.status !== "writable") return Promise.reject(state.reason ?? new TypeError("WritableStream : le flux est fermé"));
      const done = state.tail.then(() => state.sink.write?.(chunk, state.controller));
      state.tail = done.catch((reason) => {
        state.status = "errored";
        state.reason = reason;
        state.closed.reject(reason);
      });
      return done;
    }
    close() {
      const state = this.#state;
      if (state.status !== "writable") return Promise.reject(new TypeError("WritableStream : le flux est déjà fermé"));
      state.status = "closing";
      return state.tail.then(() => state.sink.close?.()).then(() => {
        state.status = "closed";
        state.closed.resolve(undefined);
      });
    }
    abort(reason) {
      const state = this.#state;
      if (state.status === "closed" || state.status === "errored") return Promise.resolve(undefined);
      state.status = "errored";
      state.reason = reason;
      state.closed.reject(reason);
      return Promise.resolve().then(() => state.sink.abort?.(reason)).then(() => undefined);
    }
    releaseLock() {
      this.#state.locked = false;
    }
  }
  class WritableStream {
    constructor(sink = {}, strategy = {}) {
      const state = {
        sink, status: "writable", locked: false, reason: undefined, closed: deferred(),
        tail: Promise.resolve(), controller: null,
      };
      state.controller = {
        signal: new AbortController().signal,
        error(reason) {
          if (state.status !== "writable") return;
          state.status = "errored";
          state.reason = reason;
          state.closed.reject(reason);
        },
      };
      writable.set(this, state);
      if (typeof sink.start === "function") {
        state.tail = Promise.resolve(sink.start(state.controller)).catch((reason) => state.controller.error(reason));
      }
    }
    get locked() { return writable.get(this).locked; }
    getWriter() { return new WritableStreamDefaultWriter(this); }
    abort(reason) {
      const writer = this.getWriter();
      return writer.abort(reason).finally(() => writer.releaseLock());
    }
    close() {
      const writer = this.getWriter();
      return writer.close().finally(() => writer.releaseLock());
    }
  }
  expose("WritableStream", WritableStream);
  expose("WritableStreamDefaultWriter", WritableStreamDefaultWriter);

  // ---- TransformStream ------------------------------------------------------------------------------------------
  class TransformStream {
    constructor(transformer = {}, writableStrategy = {}, readableStrategy = {}) {
      let output;
      const readableSide = new ReadableStream({
        start(controller) {
          output = controller;
        },
      });
      const control = {
        get desiredSize() { return output.desiredSize; },
        enqueue: (chunk) => output.enqueue(chunk),
        error: (reason) => output.error(reason),
        terminate: () => output.close(),
      };
      const writableSide = new WritableStream({
        start: () => transformer.start?.(control),
        write: (chunk) => (typeof transformer.transform === "function"
          ? transformer.transform(chunk, control) : control.enqueue(chunk)),
        close: async () => {
          await transformer.flush?.(control);
          if (output.desiredSize !== 0 && output.desiredSize !== null) output.close();
        },
        abort: (reason) => output.error(reason),
      });
      define(this, "readable", { value: readableSide, enumerable: true });
      define(this, "writable", { value: writableSide, enumerable: true });
    }
  }
  expose("TransformStream", TransformStream);

  class TextEncoderStream extends TransformStream {
    constructor() {
      const encoder = new TextEncoder();
      super({ transform: (chunk, control) => control.enqueue(encoder.encode(String(chunk))) });
    }
    get encoding() { return "utf-8"; }
  }
  expose("TextEncoderStream", TextEncoderStream);
  class TextDecoderStream extends TransformStream {
    #decoder;
    constructor(label = "utf-8", options = {}) {
      const decoder = new TextDecoder(label, options);
      super({
        transform(chunk, control) {
          const text = decoder.decode(chunk, { stream: true });
          if (text !== "") control.enqueue(text);
        },
        flush(control) {
          const text = decoder.decode();
          if (text !== "") control.enqueue(text);
        },
      });
      this.#decoder = decoder;
    }
    get encoding() { return this.#decoder.encoding; }
  }
  expose("TextDecoderStream", TextDecoderStream);
  // Stratégies de file : acceptées, sans effet (voir les limites).
  class CountQueuingStrategy {
    constructor(init) { this.highWaterMark = Number(init?.highWaterMark); }
    size() { return 1; }
  }
  expose("CountQueuingStrategy", CountQueuingStrategy);
  class ByteLengthQueuingStrategy {
    constructor(init) { this.highWaterMark = Number(init?.highWaterMark); }
    size(chunk) { return chunk?.byteLength; }
  }
  expose("ByteLengthQueuingStrategy", ByteLengthQueuingStrategy);
})();
