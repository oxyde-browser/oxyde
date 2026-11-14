// Remplace `/resources/testdriver-vendor.js` : les gestes que les tests
// demandent par `testdriver.js` (WebDriver) sont remis au lanceur par la
// console, une ligne par geste, et il les joue sur la page comme le ferait
// la fenêtre du navigateur :
//   WPT-A  move   x y           pointeur déplacé (repère de la fenêtre)
//   WPT-A  down   bouton        bouton enfoncé
//   WPT-A  up     bouton        bouton relâché (suivi du clic, s'il y a lieu)
//   WPT-A  wheel  x y dx dy     molette
//   WPT-A  key    down|up nom   touche
// Puis le script demande `/resources/wpt-played/N.js` : le lanceur ne le
// sert qu'après avoir joué les gestes, et son exécution résout la promesse
// rendue au test.
//
// LIMITES : seuls la souris, la molette et le clavier sont pris en charge ;
// un geste tactile ou au stylet fait échouer la demande.
(function () {
  "use strict";
  const internal = window.test_driver_internal;
  internal.in_automation = true;

  const emit = (...parts) => console.log("WPT-A\t" + parts.join("\t"));
  let sequence = 0;
  const waiting = new Map();
  window.__wptPlayed = (rank) => {
    const resolve = waiting.get(rank);
    waiting.delete(rank);
    document.currentScript?.remove();
    if (resolve) resolve();
  };
  const played = () => new Promise((resolve) => {
    sequence += 1;
    waiting.set(sequence, resolve);
    const script = document.createElement("script");
    script.src = "/resources/wpt-played/" + sequence + ".js";
    (document.head ?? document.documentElement).appendChild(script);
  });

  // Touches spéciales de WebDriver (plan privé U+E000).
  const named = {
    "": "Backspace", "": "Tab", "": "Enter", "": "Enter",
    "": "Shift", "": "Control", "": "Alt", "": "Escape",
    "": " ", "": "End", "": "Home", "": "ArrowLeft",
    "": "ArrowUp", "": "ArrowRight", "": "ArrowDown", "": "Delete",
    "": "Meta",
  };
  const keyName = (value) => named[value] ?? value;

  // Centre de la partie visible d'un élément, dans le repère de la fenêtre.
  function center(element) {
    const rect = element.getBoundingClientRect();
    const left = Math.max(0, rect.left);
    const right = Math.min(window.innerWidth, rect.right);
    const top = Math.max(0, rect.top);
    const bottom = Math.min(window.innerHeight, rect.bottom);
    return { x: 0.5 * (left + right), y: 0.5 * (top + bottom) };
  }

  // Point visé par une action, selon son origine : la fenêtre, la position
  // courante du pointeur, ou le centre d'un élément.
  function point(action, pointer) {
    const origin = action.origin ?? "viewport";
    let base = { x: 0, y: 0 };
    if (origin === "pointer") base = pointer;
    else if (origin !== "viewport") {
      if (!origin || origin.nodeType !== 1) throw new Error("origine d'action inconnue");
      base = center(origin);
    }
    return { x: base.x + (action.x ?? 0), y: base.y + (action.y ?? 0) };
  }

  internal.action_sequence = function (sources) {
    const lines = [];
    try {
      const pointer = { x: 0, y: 0 };
      const ticks = Math.max(0, ...sources.map((source) => source.actions.length));
      for (let tick = 0; tick < ticks; tick++) {
        for (const source of sources) {
          const action = source.actions[tick];
          if (!action || action.type === "pause") continue;
          if (source.type === "pointer") {
            const kind = source.parameters?.pointerType ?? "mouse";
            if (kind !== "mouse") throw new Error(`pointeur « ${kind} » non pris en charge`);
            if (action.type === "pointerMove") {
              const at = point(action, pointer);
              pointer.x = at.x;
              pointer.y = at.y;
              lines.push(["move", at.x, at.y]);
            } else if (action.type === "pointerDown") {
              lines.push(["down", action.button ?? 0]);
            } else if (action.type === "pointerUp") {
              lines.push(["up", action.button ?? 0]);
            } else {
              throw new Error(`action « ${action.type} » non prise en charge`);
            }
          } else if (source.type === "wheel") {
            if (action.type !== "scroll") throw new Error(`action « ${action.type} » non prise en charge`);
            const at = point(action, pointer);
            lines.push(["wheel", at.x, at.y, action.deltaX ?? 0, action.deltaY ?? 0]);
          } else if (source.type === "key") {
            if (action.type !== "keyDown" && action.type !== "keyUp") {
              throw new Error(`action « ${action.type} » non prise en charge`);
            }
            lines.push(["key", action.type === "keyDown" ? "down" : "up", keyName(action.value)]);
          } else if (source.type !== "none") {
            throw new Error(`source « ${source.type} » non prise en charge`);
          }
        }
      }
    } catch (error) {
      // Rien n'est joué à moitié.
      return Promise.reject(error);
    }
    for (const line of lines) emit(...line);
    return played();
  };

  internal.click = function (element, coords) {
    emit("move", coords.x, coords.y);
    emit("down", 0);
    emit("up", 0);
    return played();
  };

  internal.send_keys = function (element, keys) {
    element.focus();
    for (const value of Array.from(String(keys))) {
      const name = keyName(value);
      emit("key", "down", name);
      emit("key", "up", name);
    }
    return played();
  };
})();
