// API Web Animations, en version réduite : `Element.animate()`,
// `Animation`, `KeyframeEffect`, `AnimationPlaybackEvent`,
// `document.timeline`, `getAnimations()`.
//
// Le script rend une fabrique, appelée avec deux opérations du moteur :
//   animate(élément, images, durée, délai, itérations, direction, remplissage,
//           temporisation) -> identifiant de l'animation ;
//   control(identifiant, opération, valeur) -> selon l'opération (`state`,
//           `time`, `seek`, `pause`, `play`, `finish`, `cancel`).
// Elle rend la fonction que le moteur appelle quand une animation se termine.
//
// LIMITES CONNUES : pas de `playbackRate`, de `reverse()`, de
// `commitStyles()` ni de `setKeyframes()` ; `composite`, `iterationStart`,
// `endDelay` et la temporisation propre à une image clé sont ignorés ; les
// positions (`offset`) d'une description par propriété sont réparties
// régulièrement ; `getAnimations()` ne rend pas les animations et
// transitions CSS.
(function (animate, control) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const expose = (name, value) => {
    define(value.prototype, Symbol.toStringTag, { value: name, configurable: true });
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };
  // Animations que le moteur fait tourner, par identifiant.
  const live = new Map();
  const effects = new WeakMap();
  const states = new WeakMap();
  const SKIPPED = new Set(["offset", "easing", "composite"]);
  const cssName = (name) => {
    if (name === "cssFloat") return "float";
    if (name === "cssOffset") return "offset";
    if (name.startsWith("--")) return name;
    return name.replace(/[A-Z]/g, (letter) => "-" + letter.toLowerCase());
  };

  // Images clés, sous l'une des deux formes admises, en liste triée de
  // `{ offset, props }`.
  function normalize(input) {
    if (input === null || input === undefined) return [];
    if (typeof input !== "object") throw new TypeError("images clés : un objet ou une liste est attendu");
    if (typeof input[Symbol.iterator] === "function") {
      const frames = [];
      let latest = 0;
      for (const frame of input) {
        if (frame === null || frame === undefined) {
          frames.push({ offset: null, props: {} });
          continue;
        }
        if (typeof frame !== "object") throw new TypeError("image clé : un objet est attendu");
        const offset = frame.offset === undefined || frame.offset === null ? null : Number(frame.offset);
        if (offset !== null) {
          if (!(offset >= 0 && offset <= 1)) throw new TypeError("image clé : position hors de [0, 1]");
          if (offset < latest) throw new TypeError("images clés : positions en désordre");
          latest = offset;
        }
        const props = {};
        for (const key of Object.keys(frame)) {
          if (!SKIPPED.has(key)) props[key] = String(frame[key]);
        }
        frames.push({ offset, props });
      }
      // Positions manquantes : 0 au début, 1 à la fin, réparties entre deux.
      if (frames.length > 0) {
        const last = frames[frames.length - 1];
        if (last.offset === null) last.offset = 1;
        if (frames.length > 1 && frames[0].offset === null) frames[0].offset = 0;
        let index = 1;
        while (index < frames.length) {
          if (frames[index].offset !== null) {
            index += 1;
            continue;
          }
          let next = index;
          while (frames[next].offset === null) next += 1;
          const before = frames[index - 1].offset;
          const after = frames[next].offset;
          for (let at = index; at < next; at++) {
            frames[at].offset = before + ((after - before) * (at - index + 1)) / (next - index + 1);
          }
          index = next;
        }
      }
      return frames;
    }
    // Description par propriété : `{ opacity: [0, 1], transform: [...] }`.
    const byOffset = new Map();
    for (const key of Object.keys(input)) {
      if (SKIPPED.has(key)) continue;
      const raw = input[key];
      const values = Array.isArray(raw) ? raw : [raw];
      values.forEach((value, index) => {
        const offset = values.length === 1 ? 1 : index / (values.length - 1);
        if (!byOffset.has(offset)) byOffset.set(offset, { offset, props: {} });
        byOffset.get(offset).props[key] = String(value);
      });
    }
    return [...byOffset.values()].sort((a, b) => a.offset - b.offset);
  }

  function timingOf(options) {
    const out = {
      duration: 0, delay: 0, iterations: 1, direction: "normal", fill: "auto", easing: "linear", id: "",
    };
    if (options === undefined || options === null) return out;
    if (typeof options === "object") {
      if (options.duration !== undefined && options.duration !== "auto") out.duration = Number(options.duration);
      if (options.delay !== undefined) out.delay = Number(options.delay);
      if (options.iterations !== undefined) out.iterations = Number(options.iterations);
      for (const key of ["direction", "fill", "easing", "id"]) {
        if (options[key] !== undefined) out[key] = String(options[key]);
      }
    } else {
      out.duration = Number(options);
    }
    if (!(out.duration >= 0)) throw new TypeError("animation : durée invalide");
    if (!(out.iterations >= 0)) throw new TypeError("animation : nombre d'itérations invalide");
    if (!Number.isFinite(out.delay)) throw new TypeError("animation : délai invalide");
    if (!["normal", "reverse", "alternate", "alternate-reverse"].includes(out.direction)) {
      throw new TypeError("animation : direction invalide");
    }
    if (!["none", "forwards", "backwards", "both", "auto"].includes(out.fill)) {
      throw new TypeError("animation : remplissage invalide");
    }
    return out;
  }

  // ---- KeyframeEffect ------------------------------------------------------------------------------------------
  const effectOf = (effect) => {
    const state = effects.get(effect);
    if (state === undefined) throw new TypeError("un KeyframeEffect est attendu");
    return state;
  };
  class KeyframeEffect {
    constructor(target, keyframes, options) {
      if (arguments.length < 2) throw new TypeError("KeyframeEffect : la cible et les images clés sont attendues");
      if (target !== null && (typeof target !== "object" || target.nodeType !== 1)) {
        throw new TypeError("KeyframeEffect : la cible est un élément ou null");
      }
      effects.set(this, { target, frames: normalize(keyframes), timing: timingOf(options) });
    }
    get target() { return effectOf(this).target; }
    getKeyframes() {
      return effectOf(this).frames.map((frame) => ({
        offset: frame.offset, computedOffset: frame.offset, easing: "linear", composite: "auto",
        ...frame.props,
      }));
    }
    getTiming() {
      const { duration, delay, iterations, direction, fill, easing } = effectOf(this).timing;
      return { delay, direction, duration, easing, endDelay: 0, fill, iterationStart: 0, iterations };
    }
    getComputedTiming() {
      const timing = this.getTiming();
      const activeDuration = timing.duration * timing.iterations;
      return { ...timing, activeDuration, endTime: Math.max(timing.delay + activeDuration, 0) };
    }
  }
  expose("KeyframeEffect", KeyframeEffect);

  // ---- Chronologie et événement --------------------------------------------------------------------------------
  class AnimationTimeline {
    get currentTime() { return global.performance.now(); }
  }
  expose("AnimationTimeline", AnimationTimeline);
  class DocumentTimeline extends AnimationTimeline {}
  expose("DocumentTimeline", DocumentTimeline);
  const timeline = new DocumentTimeline();
  define(global.Document.prototype, "timeline", {
    get() { return timeline; }, enumerable: true, configurable: true,
  });
  class AnimationPlaybackEvent extends global.Event {
    #currentTime;
    #timelineTime;
    constructor(type, init = {}) {
      super(type, init);
      this.#currentTime = init?.currentTime ?? null;
      this.#timelineTime = init?.timelineTime ?? null;
    }
    get currentTime() { return this.#currentTime; }
    get timelineTime() { return this.#timelineTime; }
  }
  expose("AnimationPlaybackEvent", AnimationPlaybackEvent);

  // ---- Animation -----------------------------------------------------------------------------------------------
  const deferred = () => {
    const out = {};
    out.promise = new Promise((resolve, reject) => {
      out.resolve = resolve;
      out.reject = reject;
    });
    // Le rejet d'une animation annulée n'est pas une erreur de la page.
    out.promise.catch(() => {});
    return out;
  };
  const stateOf = (animation) => {
    const state = states.get(animation);
    if (state === undefined) throw new TypeError("une Animation est attendue");
    return state;
  };
  const RUNNING = 1;
  const PAUSED = 2;
  const FINISHED = 3;
  // État que le moteur donne à l'animation, ou 0 s'il ne la connaît plus.
  const nativeState = (state) => {
    if (state.native === null) return 0;
    const code = control(state.native, "state");
    if (code === 0) {
      live.delete(state.native);
      state.native = null;
    }
    return code;
  };
  // Remet l'animation au moteur, depuis le début.
  function start(animation, state) {
    if (state.effect === null) return false;
    const effect = effects.get(state.effect);
    if (effect.target === null) return false;
    const frames = effect.frames.map((frame) => [
      frame.offset,
      Object.entries(frame.props).map(([name, value]) => cssName(name) + ":" + value).join(";"),
    ]);
    const timing = effect.timing;
    state.native = animate(effect.target, frames, timing.duration, timing.delay, timing.iterations,
      timing.direction, timing.fill, timing.easing);
    live.set(state.native, animation);
    return true;
  }
  // L'animation est terminée : promesse `finished`, puis événement `finish`.
  function settle(animation, state) {
    state.done = true;
    if (state.settled) return;
    state.settled = true;
    state.finished.resolve(animation);
    const currentTime = animation.currentTime;
    global.queueMicrotask(() => {
      animation.dispatchEvent(new AnimationPlaybackEvent("finish", {
        currentTime, timelineTime: timeline.currentTime,
      }));
    });
  }
  const renew = (state) => {
    if (!state.settled) return;
    state.finished = deferred();
    state.settled = false;
  };
  class Animation extends global.EventTarget {
    constructor(effect = null) {
      super();
      if (effect !== null && !effects.has(effect)) throw new TypeError("Animation : un KeyframeEffect est attendu");
      states.set(this, {
        effect, native: null, finished: deferred(), settled: false, done: false,
        id: effect === null ? "" : effects.get(effect).timing.id,
      });
    }
    get id() { return stateOf(this).id; }
    set id(value) { stateOf(this).id = String(value); }
    get effect() { return stateOf(this).effect; }
    get timeline() { return timeline; }
    get playbackRate() { return 1; }
    get pending() { return false; }
    get replaceState() { return "active"; }
    get ready() { return Promise.resolve(this); }
    get finished() { return stateOf(this).finished.promise; }
    get playState() {
      const state = stateOf(this);
      const code = nativeState(state);
      if (code === RUNNING) return "running";
      if (code === PAUSED) return "paused";
      if (code === FINISHED) return "finished";
      return state.done ? "finished" : "idle";
    }
    get currentTime() {
      const state = stateOf(this);
      if (nativeState(state) !== 0) return control(state.native, "time");
      if (!state.done || state.effect === null) return null;
      return state.effect.getComputedTiming().endTime;
    }
    set currentTime(value) {
      const state = stateOf(this);
      if (value === null) return;
      const time = Number(value);
      if (!Number.isFinite(time)) throw new TypeError("currentTime : un nombre fini est attendu");
      if (nativeState(state) === 0) {
        if (!start(this, state)) return;
        control(state.native, "pause");
      }
      renew(state);
      state.done = false;
      control(state.native, "seek", time);
    }
    get startTime() {
      const state = stateOf(this);
      if (nativeState(state) !== RUNNING) return null;
      return timeline.currentTime - control(state.native, "time");
    }
    play() {
      const state = stateOf(this);
      const code = nativeState(state);
      if (code === 0) {
        renew(state);
        state.done = false;
        start(this, state);
        return;
      }
      if (code === FINISHED) {
        renew(state);
        state.done = false;
      }
      control(state.native, "play");
    }
    pause() {
      const state = stateOf(this);
      if (nativeState(state) === 0 && !start(this, state)) return;
      control(state.native, "pause");
    }
    finish() {
      const state = stateOf(this);
      if (state.effect !== null && !Number.isFinite(state.effect.getComputedTiming().endTime)) {
        throw new DOMException("une animation sans fin ne se termine pas", "InvalidStateError");
      }
      if (nativeState(state) === 0 && !start(this, state)) {
        settle(this, state);
        return;
      }
      control(state.native, "finish");
      // Sans remplissage, le moteur l'oublie aussitôt.
      nativeState(state);
      settle(this, state);
    }
    cancel() {
      const state = stateOf(this);
      const idle = nativeState(state) === 0 && !state.done;
      if (state.native !== null) {
        control(state.native, "cancel");
        live.delete(state.native);
        state.native = null;
      }
      state.done = false;
      if (idle) return;
      if (!state.settled) state.finished.reject(new DOMException("l'animation est annulée", "AbortError"));
      state.finished = deferred();
      state.settled = false;
      global.queueMicrotask(() => {
        this.dispatchEvent(new AnimationPlaybackEvent("cancel", {
          currentTime: null, timelineTime: timeline.currentTime,
        }));
      });
    }
    persist() {}
  }
  // Gestionnaires `onfinish`, `oncancel`, `onremove` : `null` tant qu'aucun
  // n'est posé ; le moteur appelle celui que la page affecte.
  for (const kind of ["finish", "cancel", "remove"]) {
    define(Animation.prototype, "on" + kind, {
      value: null, writable: true, enumerable: true, configurable: true,
    });
  }
  expose("Animation", Animation);

  // ---- Element.animate() et getAnimations() ----------------------------------------------------------------------
  const method = (target, name, value) => define(target, name, {
    value, writable: true, enumerable: true, configurable: true,
  });
  const relevant = () => [...live.values()].filter((animation) => nativeState(stateOf(animation)) !== 0);
  method(global.Element.prototype, "animate", function animate(keyframes, options) {
    const animation = new Animation(new KeyframeEffect(this, keyframes, options));
    animation.play();
    return animation;
  });
  method(global.Element.prototype, "getAnimations", function getAnimations(options) {
    const subtree = Boolean(options?.subtree);
    return relevant().filter((animation) => {
      const target = animation.effect.target;
      return target === this || (subtree && this.contains(target));
    });
  });
  method(global.Document.prototype, "getAnimations", function getAnimations() {
    return relevant().filter((animation) => animation.effect.target.ownerDocument === this);
  });

  // Le moteur signale la fin de l'animation `id`.
  return function finished(id) {
    const animation = live.get(id);
    if (animation === undefined) return;
    const state = states.get(animation);
    nativeState(state);
    settle(animation, state);
  };
})
