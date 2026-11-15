// Canevas 2D (HTML Standard §4.12.5), écrit en JavaScript. Ce script tient
// l'état de chaque contexte (transformation, styles, tracé courant, pile des
// états) ; le moteur dessine (voir `canvas.rs` pour le format des tracés, des
// peintures et des états qu'il attend).
//
// Fonction appelée par le moteur avec :
//   canvasId(élément)                 -> identifiant de la surface de l'élément ;
//   canvasResize(id, l, h)            -> crée ou remet à zéro la surface ;
//   canvasFill(id, tracé, peinture, état, pairImpair) ;
//   canvasStroke(id, tracé, peinture, état, trait) ;
//   canvasClip(id, tracé, pairImpair, découpe) -> identifiant de découpe ;
//   canvasImage(id, nature, source, géométrie, état, lissage) ;
//   canvasGetPixels(id, x, y, l, h)   -> pixels RGBA (chaîne binaire) ;
//   canvasPutPixels(id, pixels, l, x, y, sx, sy, sl, sh) ;
//   canvasPng(id)                     -> image PNG (chaîne binaire) ;
//   canvasSourceSize(nature, id)      -> « largeur hauteur » d'une source, ou null ;
//   parseCssColor(texte)              -> « r g b a », ou null ;
//   canvasDecode(id, octets)          -> décode une image dans une surface neuve et rend
//                                        « largeur hauteur », ou null ;
//   canvasText(id, texte, police, placement, peinture, état, trait) ;
//   canvasMeasure(texte, police)      -> « largeur montée descente », ou null.
//
// LIMITES CONNUES : le texte se dessine sur une ligne, de gauche à droite
// (`direction` est retenu sans effet), sans `letterSpacing`, `wordSpacing`
// ni `fontKerning` ; les lignes de base `hanging` et `ideographic` valent
// `top` et `bottom` ; pas d'ombres ni de filtres ;
// `isPointInStroke` rend toujours faux ; pas de dégradé conique ; un motif se
// répète toujours dans les deux sens ; `drawImage` ne prend ni vidéo ni
// image SVG ; seul le contexte `2d` existe (ni WebGL, ni `bitmaprenderer`) ;
// un `OffscreenCanvas` ne se transfère pas à un worker ; `toDataURL`,
// `toBlob` et `convertToBlob` ne rendent que du PNG ; `createImageBitmap`
// ignore `premultiplyAlpha` et `colorSpaceConversion` ;
// changer la taille d'un canevas par `setAttribute` avec la même valeur ne
// l'efface pas.
(function (canvasId, canvasResize, canvasFill, canvasStroke, canvasClip, canvasImage, canvasGetPixels,
  canvasPutPixels, canvasPng, canvasSourceSize, parseCssColor, canvasDecode, canvasText, canvasMeasure) {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const DOMException = global.DOMException;
  const TAU = 2 * Math.PI;
  const finite = (...values) => values.every((value) => Number.isFinite(value));
  const tag = (ctor, name) => {
    define(ctor.prototype, Symbol.toStringTag, { value: name, configurable: true });
  };
  const expose = (name, value) => {
    define(global, name, { value, writable: true, enumerable: false, configurable: true });
  };
  const toBinary = (bytes) => {
    let out = "";
    for (let i = 0; i < bytes.length; i += 8192) {
      out += String.fromCharCode.apply(null, bytes.subarray(i, i + 8192));
    }
    return out;
  };

  // ---- Matrices : [a, b, c, d, e, f], x' = a·x + c·y + e, y' = b·x + d·y + f -----------
  const IDENTITY = [1, 0, 0, 1, 0, 0];
  const multiply = (m, n) => [
    m[0] * n[0] + m[2] * n[1], m[1] * n[0] + m[3] * n[1],
    m[0] * n[2] + m[2] * n[3], m[1] * n[2] + m[3] * n[3],
    m[0] * n[4] + m[2] * n[5] + m[4], m[1] * n[4] + m[3] * n[5] + m[5],
  ];
  const invert = (m) => {
    const det = m[0] * m[3] - m[1] * m[2];
    if (!Number.isFinite(det) || det === 0) return null;
    return [m[3] / det, -m[1] / det, -m[2] / det, m[0] / det,
      (m[2] * m[5] - m[3] * m[4]) / det, (m[1] * m[4] - m[0] * m[5]) / det];
  };
  const apply = (m, x, y) => [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]];

  // ---- Tracés ---------------------------------------------------------------------------
  // Verbes : 0 aller à, 1 ligne, 2 courbe quadratique, 3 courbe cubique, 4 fermer.
  class Outline {
    data = [];
    open = false;
    // Point courant, et début du sous-tracé, dans le repère des données.
    x = 0;
    y = 0;
    startX = 0;
    startY = 0;
    moveTo(m, x, y) {
      if (!finite(x, y)) return;
      const [px, py] = apply(m, x, y);
      this.data.push(0, px, py);
      this.open = true;
      this.x = this.startX = px;
      this.y = this.startY = py;
    }
    lineTo(m, x, y) {
      if (!finite(x, y)) return;
      if (!this.open) {
        this.moveTo(m, x, y);
        return;
      }
      const [px, py] = apply(m, x, y);
      this.data.push(1, px, py);
      this.x = px;
      this.y = py;
    }
    quadraticCurveTo(m, cx, cy, x, y) {
      if (!finite(cx, cy, x, y)) return;
      if (!this.open) this.moveTo(m, cx, cy);
      const [qx, qy] = apply(m, cx, cy);
      const [px, py] = apply(m, x, y);
      this.data.push(2, qx, qy, px, py);
      this.x = px;
      this.y = py;
    }
    bezierCurveTo(m, c1x, c1y, c2x, c2y, x, y) {
      if (!finite(c1x, c1y, c2x, c2y, x, y)) return;
      if (!this.open) this.moveTo(m, c1x, c1y);
      const [ax, ay] = apply(m, c1x, c1y);
      const [bx, by] = apply(m, c2x, c2y);
      const [px, py] = apply(m, x, y);
      this.data.push(3, ax, ay, bx, by, px, py);
      this.x = px;
      this.y = py;
    }
    closePath() {
      if (!this.open) return;
      this.data.push(4);
      // Le sous-tracé suivant repart du même point.
      this.data.push(0, this.startX, this.startY);
      this.x = this.startX;
      this.y = this.startY;
    }
    rect(m, x, y, w, h) {
      if (!finite(x, y, w, h)) return;
      this.moveTo(m, x, y);
      this.lineTo(m, x + w, y);
      this.lineTo(m, x + w, y + h);
      this.lineTo(m, x, y + h);
      this.closePath();
    }
    ellipse(m, x, y, rx, ry, rotation, start, end, counterclockwise) {
      if (!finite(x, y, rx, ry, rotation, start, end)) return;
      if (rx < 0 || ry < 0) throw new DOMException("rayon négatif", "IndexSizeError");
      // Angle parcouru, ramené à un tour au plus.
      let sweep = end - start;
      if (!counterclockwise) {
        if (sweep >= TAU) sweep = TAU;
        else sweep = ((sweep % TAU) + TAU) % TAU;
      } else if (sweep <= -TAU) {
        sweep = -TAU;
      } else {
        sweep = -((((-sweep) % TAU) + TAU) % TAU);
      }
      const cos = Math.cos(rotation);
      const sin = Math.sin(rotation);
      const point = (angle) => {
        const ex = rx * Math.cos(angle);
        const ey = ry * Math.sin(angle);
        return [x + cos * ex - sin * ey, y + sin * ex + cos * ey];
      };
      const tangent = (angle) => {
        const ex = -rx * Math.sin(angle);
        const ey = ry * Math.cos(angle);
        return [cos * ex - sin * ey, sin * ex + cos * ey];
      };
      const [fromX, fromY] = point(start);
      this.lineTo(m, fromX, fromY);
      // Un quart de tour au plus par courbe cubique.
      const pieces = Math.max(1, Math.ceil(Math.abs(sweep) / (Math.PI / 2) - 1e-9));
      const step = sweep / pieces;
      if (step === 0) return;
      const k = (4 / 3) * Math.tan(step / 4);
      for (let i = 0; i < pieces; i++) {
        const a0 = start + i * step;
        const a1 = a0 + step;
        const [p0x, p0y] = point(a0);
        const [p3x, p3y] = point(a1);
        const [t0x, t0y] = tangent(a0);
        const [t1x, t1y] = tangent(a1);
        this.bezierCurveTo(m, p0x + k * t0x, p0y + k * t0y, p3x - k * t1x, p3y - k * t1y, p3x, p3y);
      }
    }
    arc(m, x, y, radius, start, end, counterclockwise) {
      if (!finite(x, y, radius, start, end)) return;
      if (radius < 0) throw new DOMException("rayon négatif", "IndexSizeError");
      this.ellipse(m, x, y, radius, radius, 0, start, end, counterclockwise);
    }
    arcTo(m, x1, y1, x2, y2, radius) {
      if (!finite(x1, y1, x2, y2, radius)) return;
      if (radius < 0) throw new DOMException("rayon négatif", "IndexSizeError");
      if (!this.open) this.moveTo(m, x1, y1);
      // Point courant dans le repère de l'appelant.
      const back = invert(m);
      if (back === null) return;
      const [x0, y0] = apply(back, this.x, this.y);
      const v1x = x0 - x1;
      const v1y = y0 - y1;
      const v2x = x2 - x1;
      const v2y = y2 - y1;
      const l1 = Math.hypot(v1x, v1y);
      const l2 = Math.hypot(v2x, v2y);
      const cross = v1x * v2y - v1y * v2x;
      if (l1 === 0 || l2 === 0 || radius === 0 || Math.abs(cross) < 1e-12 * l1 * l2) {
        this.lineTo(m, x1, y1);
        return;
      }
      const u1x = v1x / l1;
      const u1y = v1y / l1;
      const u2x = v2x / l2;
      const u2y = v2y / l2;
      const half = Math.acos(Math.max(-1, Math.min(1, u1x * u2x + u1y * u2y))) / 2;
      const distance = radius / Math.tan(half);
      const t1x = x1 + u1x * distance;
      const t1y = y1 + u1y * distance;
      const t2x = x1 + u2x * distance;
      const t2y = y1 + u2y * distance;
      const bx = u1x + u2x;
      const by = u1y + u2y;
      const bl = Math.hypot(bx, by);
      const reach = radius / Math.sin(half);
      const cx = x1 + (bx / bl) * reach;
      const cy = y1 + (by / bl) * reach;
      this.lineTo(m, t1x, t1y);
      this.ellipse(m, cx, cy, radius, radius, 0, Math.atan2(t1y - cy, t1x - cx),
        Math.atan2(t2y - cy, t2x - cx), cross > 0);
    }
    roundRect(m, x, y, w, h, radii = 0) {
      if (!finite(x, y, w, h)) return;
      const list = (Array.isArray(radii) ? radii : [radii]).map((radius) => {
        const pair = typeof radius === "object" && radius !== null
          ? [Number(radius.x), Number(radius.y)] : [Number(radius), Number(radius)];
        if (!finite(pair[0], pair[1])) return null;
        if (pair[0] < 0 || pair[1] < 0) throw new RangeError("rayon négatif");
        return pair;
      });
      if (list.length < 1 || list.length > 4) throw new RangeError("un à quatre rayons sont attendus");
      if (list.includes(null)) return;
      // Coins : haut gauche, haut droit, bas droit, bas gauche.
      let [tl, tr, br, bl] = list.length === 1 ? [list[0], list[0], list[0], list[0]]
        : list.length === 2 ? [list[0], list[1], list[0], list[1]]
          : list.length === 3 ? [list[0], list[1], list[2], list[1]] : list;
      // Largeur ou hauteur négative : le rectangle est retourné.
      if (w < 0) {
        x += w;
        w = -w;
        [tl, tr, br, bl] = [tr, tl, bl, br];
      }
      if (h < 0) {
        y += h;
        h = -h;
        [tl, tr, br, bl] = [bl, br, tr, tl];
      }
      // Des rayons trop grands sont réduits ensemble.
      const scale = Math.min(1, w / (tl[0] + tr[0] || 1), w / (bl[0] + br[0] || 1),
        h / (tl[1] + bl[1] || 1), h / (tr[1] + br[1] || 1));
      [tl, tr, br, bl] = [tl, tr, br, bl].map(([rx, ry]) => [rx * scale, ry * scale]);
      this.moveTo(m, x + tl[0], y);
      this.lineTo(m, x + w - tr[0], y);
      this.ellipse(m, x + w - tr[0], y + tr[1], tr[0], tr[1], 0, -Math.PI / 2, 0, false);
      this.lineTo(m, x + w, y + h - br[1]);
      this.ellipse(m, x + w - br[0], y + h - br[1], br[0], br[1], 0, 0, Math.PI / 2, false);
      this.lineTo(m, x + bl[0], y + h);
      this.ellipse(m, x + bl[0], y + h - bl[1], bl[0], bl[1], 0, Math.PI / 2, Math.PI, false);
      this.lineTo(m, x, y + tl[1]);
      this.ellipse(m, x + tl[0], y + tl[1], tl[0], tl[1], 0, Math.PI, 1.5 * Math.PI, false);
      this.closePath();
      this.moveTo(m, x, y);
    }
    // Ajoute les données de `other`, transformées par `m`.
    append(other, m) {
      const data = other.data;
      for (let i = 0; i < data.length;) {
        const verb = data[i++];
        const count = verb === 4 ? 0 : verb === 3 ? 3 : verb === 2 ? 2 : 1;
        this.data.push(verb);
        for (let k = 0; k < count; k++, i += 2) {
          const [px, py] = apply(m, data[i], data[i + 1]);
          this.data.push(px, py);
          this.x = px;
          this.y = py;
          if (verb === 0) {
            this.startX = px;
            this.startY = py;
          }
        }
        if (verb !== 4) this.open = true;
      }
    }
    text() {
      return this.data.join(" ");
    }
    // Vrai si le point est dans le tracé (les courbes sont aplaties).
    contains(x, y, evenOdd) {
      const data = this.data;
      let winding = 0;
      let crossings = 0;
      let fromX = 0;
      let fromY = 0;
      let startX = 0;
      let startY = 0;
      const edge = (toX, toY) => {
        if ((fromY <= y) !== (toY <= y)) {
          const at = fromX + ((y - fromY) / (toY - fromY)) * (toX - fromX);
          if (at > x) {
            crossings++;
            winding += toY > fromY ? 1 : -1;
          }
        }
        fromX = toX;
        fromY = toY;
      };
      for (let i = 0; i < data.length;) {
        const verb = data[i++];
        if (verb === 0) {
          edge(startX, startY);
          fromX = startX = data[i];
          fromY = startY = data[i + 1];
          i += 2;
        } else if (verb === 1) {
          edge(data[i], data[i + 1]);
          i += 2;
        } else if (verb === 2 || verb === 3) {
          const p0x = fromX;
          const p0y = fromY;
          const points = data.slice(i, i + (verb === 2 ? 4 : 6));
          i += points.length;
          for (let step = 1; step <= 16; step++) {
            const t = step / 16;
            const u = 1 - t;
            if (verb === 2) {
              edge(u * u * p0x + 2 * u * t * points[0] + t * t * points[2],
                u * u * p0y + 2 * u * t * points[1] + t * t * points[3]);
            } else {
              edge(u * u * u * p0x + 3 * u * u * t * points[0] + 3 * u * t * t * points[2] + t * t * t * points[4],
                u * u * u * p0y + 3 * u * u * t * points[1] + 3 * u * t * t * points[3] + t * t * t * points[5]);
            }
          }
        } else {
          edge(startX, startY);
        }
      }
      edge(startX, startY);
      return evenOdd ? crossings % 2 === 1 : winding !== 0;
    }
  }

  // Tracé d'un attribut `d` de SVG, ajouté à `outline`.
  function svgPath(outline, text) {
    const tokens = String(text).match(/[a-zA-Z]|[-+]?(?:\d*\.\d+|\d+\.?)(?:[eE][-+]?\d+)?/g) ?? [];
    let at = 0;
    let x = 0;
    let y = 0;
    let startX = 0;
    let startY = 0;
    let lastControl = null;
    let lastCommand = "";
    const number = () => {
      const value = Number(tokens[at]);
      if (at >= tokens.length || Number.isNaN(value)) throw new Error("nombre attendu");
      at++;
      return value;
    };
    const m = IDENTITY;
    try {
      while (at < tokens.length) {
        let command = tokens[at];
        if (/^[a-zA-Z]$/.test(command)) at++;
        else if (lastCommand === "") break;
        else command = lastCommand === "M" ? "L" : lastCommand === "m" ? "l" : lastCommand;
        const relative = command === command.toLowerCase();
        const dx = relative ? x : 0;
        const dy = relative ? y : 0;
        let control = null;
        switch (command.toUpperCase()) {
          case "M":
            x = number() + dx;
            y = number() + dy;
            startX = x;
            startY = y;
            outline.moveTo(m, x, y);
            break;
          case "L":
            x = number() + dx;
            y = number() + dy;
            outline.lineTo(m, x, y);
            break;
          case "H":
            x = number() + dx;
            outline.lineTo(m, x, y);
            break;
          case "V":
            y = number() + dy;
            outline.lineTo(m, x, y);
            break;
          case "C": {
            const c1x = number() + dx;
            const c1y = number() + dy;
            const c2x = number() + dx;
            const c2y = number() + dy;
            x = number() + dx;
            y = number() + dy;
            outline.bezierCurveTo(m, c1x, c1y, c2x, c2y, x, y);
            control = [c2x, c2y, "C"];
            break;
          }
          case "S": {
            const c1x = lastControl?.[2] === "C" ? 2 * x - lastControl[0] : x;
            const c1y = lastControl?.[2] === "C" ? 2 * y - lastControl[1] : y;
            const c2x = number() + dx;
            const c2y = number() + dy;
            x = number() + dx;
            y = number() + dy;
            outline.bezierCurveTo(m, c1x, c1y, c2x, c2y, x, y);
            control = [c2x, c2y, "C"];
            break;
          }
          case "Q": {
            const cx = number() + dx;
            const cy = number() + dy;
            x = number() + dx;
            y = number() + dy;
            outline.quadraticCurveTo(m, cx, cy, x, y);
            control = [cx, cy, "Q"];
            break;
          }
          case "T": {
            const cx = lastControl?.[2] === "Q" ? 2 * x - lastControl[0] : x;
            const cy = lastControl?.[2] === "Q" ? 2 * y - lastControl[1] : y;
            x = number() + dx;
            y = number() + dy;
            outline.quadraticCurveTo(m, cx, cy, x, y);
            control = [cx, cy, "Q"];
            break;
          }
          case "A": {
            let rx = Math.abs(number());
            let ry = Math.abs(number());
            const phi = (number() * Math.PI) / 180;
            const large = number() !== 0;
            const sweep = number() !== 0;
            const fromX = x;
            const fromY = y;
            x = number() + dx;
            y = number() + dy;
            if (rx === 0 || ry === 0) {
              outline.lineTo(m, x, y);
              break;
            }
            // Passage de la forme « extrémités » à la forme « centre » (SVG, annexe B.2.4).
            const cos = Math.cos(phi);
            const sin = Math.sin(phi);
            const hx = (fromX - x) / 2;
            const hy = (fromY - y) / 2;
            const px = cos * hx + sin * hy;
            const py = -sin * hx + cos * hy;
            const lambda = (px * px) / (rx * rx) + (py * py) / (ry * ry);
            if (lambda > 1) {
              rx *= Math.sqrt(lambda);
              ry *= Math.sqrt(lambda);
            }
            const top = rx * rx * ry * ry - rx * rx * py * py - ry * ry * px * px;
            const bottom = rx * rx * py * py + ry * ry * px * px;
            const factor = (large === sweep ? -1 : 1) * Math.sqrt(Math.max(0, top / bottom));
            const cxp = (factor * rx * py) / ry;
            const cyp = (-factor * ry * px) / rx;
            const cx = cos * cxp - sin * cyp + (fromX + x) / 2;
            const cy = sin * cxp + cos * cyp + (fromY + y) / 2;
            const start = Math.atan2((py - cyp) / ry, (px - cxp) / rx);
            const end = Math.atan2((-py - cyp) / ry, (-px - cxp) / rx);
            outline.ellipse(m, cx, cy, rx, ry, phi, start, end, !sweep);
            break;
          }
          case "Z":
            outline.closePath();
            x = startX;
            y = startY;
            break;
          default:
            throw new Error("commande inconnue");
        }
        lastControl = control;
        lastCommand = command;
      }
    } catch {
      // Tracé mal formé : ce qui précède la faute est gardé.
    }
  }

  const PATH_METHODS = ["closePath", "moveTo", "lineTo", "bezierCurveTo", "quadraticCurveTo", "arc", "arcTo",
    "ellipse", "rect", "roundRect"];
  const outlines = new WeakMap();
  class Path2D {
    constructor(path = undefined) {
      const outline = new Outline();
      outlines.set(this, outline);
      if (path instanceof Path2D) outline.append(outlines.get(path), IDENTITY);
      else if (path !== undefined) svgPath(outline, path);
    }
    addPath(path, transform = undefined) {
      if (!(path instanceof Path2D)) throw new TypeError("Path2D.addPath : un tracé est attendu");
      const m = transform === undefined || transform === null ? IDENTITY : matrixOf(transform);
      if (!finite(...m)) return;
      outlines.get(this).append(outlines.get(path), m);
    }
  }
  for (const name of PATH_METHODS) {
    define(Path2D.prototype, name, {
      value: { [name](...args) { outlines.get(this)[name](IDENTITY, ...args.map(numberOrRadii)); } }[name],
      writable: true, enumerable: true, configurable: true,
    });
  }
  tag(Path2D, "Path2D");
  expose("Path2D", Path2D);
  // Argument d'une méthode de tracé : un nombre, sauf les rayons de `roundRect`.
  function numberOrRadii(value) {
    return typeof value === "object" && value !== null ? value : Number(value);
  }
  // Matrice d'un objet qui en a la forme (`DOMMatrix`, `{ a, b, c, d, e, f }`).
  function matrixOf(init) {
    const pick = (short, long, fallback) => Number(init[short] ?? init[long] ?? fallback);
    return [pick("a", "m11", 1), pick("b", "m12", 0), pick("c", "m21", 0), pick("d", "m22", 1),
      pick("e", "m41", 0), pick("f", "m42", 0)];
  }

  // ---- Peintures ------------------------------------------------------------------------
  // Écriture d'une couleur, comme la rend `fillStyle`.
  const serializeColor = ([r, g, b, a]) => {
    if (a >= 1) return "#" + [r, g, b].map((part) => part.toString(16).padStart(2, "0")).join("");
    const byte = Math.round(a * 255);
    let shown = Math.round((byte / 255) * 100) / 100;
    if (Math.round(shown * 255) !== byte) shown = Math.round((byte / 255) * 1000) / 1000;
    return "rgba(" + r + ", " + g + ", " + b + ", " + shown + ")";
  };
  const colorOf = (text) => {
    const parsed = parseCssColor(String(text));
    return parsed === null ? null : parsed.split(" ").map(Number);
  };
  const gradients = new WeakMap();
  class CanvasGradient {
    constructor() {
      throw new TypeError("constructeur illégal");
    }
    addColorStop(offset, color) {
      const at = Number(offset);
      if (!(at >= 0 && at <= 1)) throw new DOMException("position hors de [0, 1]", "IndexSizeError");
      const parsed = colorOf(color);
      if (parsed === null) throw new DOMException("couleur invalide", "SyntaxError");
      gradients.get(this).stops.push([at, parsed]);
    }
  }
  tag(CanvasGradient, "CanvasGradient");
  expose("CanvasGradient", CanvasGradient);
  const makeGradient = (kind, coords) => {
    const gradient = Object.create(CanvasGradient.prototype);
    gradients.set(gradient, { kind, coords, stops: [] });
    return gradient;
  };
  const patterns = new WeakMap();
  class CanvasPattern {
    constructor() {
      throw new TypeError("constructeur illégal");
    }
    setTransform() {}
  }
  tag(CanvasPattern, "CanvasPattern");
  expose("CanvasPattern", CanvasPattern);
  // Texte de peinture attendu par le moteur.
  const paintText = (style) => {
    if (typeof style === "string") return "c " + colorOf(style).join(" ");
    const gradient = gradients.get(style);
    if (gradient !== undefined) {
      // Tri stable : à position égale, l'ordre d'ajout.
      const stops = gradient.stops.map((stop, rank) => [stop, rank])
        .sort((a, b) => a[0][0] - b[0][0] || a[1] - b[1])
        .map(([[at, color]]) => at + " " + color.join(" "));
      return gradient.kind + " " + gradient.coords.join(" ") + " " + stops.join(" ");
    }
    const pattern = patterns.get(style);
    return "p " + pattern.kind + " " + pattern.id;
  };
  // Identifiant de surface des objets qui ne sont pas des éléments
  // (`OffscreenCanvas`, `ImageBitmap`) ; la surface est rendue quand l'objet
  // n'est plus atteignable.
  let nextSurface = 3000000000;
  const detached = new WeakMap();
  const released = new global.FinalizationRegistry((id) => canvasResize(id, 0, 0));
  const allocate = (owner) => {
    const id = nextSurface++;
    detached.set(owner, id);
    released.register(owner, id);
    return id;
  };
  const idOf = (owner) => detached.get(owner) ?? canvasId(owner);
  const bitmaps = new WeakMap();
  // Source de pixels d'un `drawImage` ou d'un motif : nature, identifiant, taille.
  const sourceOf = (image) => {
    if (image instanceof OffscreenCanvas) {
      surfaceOf(image);
      return { kind: "c", id: idOf(image), width: image.width, height: image.height };
    }
    if (image instanceof ImageBitmap) {
      const bitmap = bitmaps.get(image);
      if (bitmap.closed) throw new DOMException("image fermée", "InvalidStateError");
      return { kind: "c", id: bitmap.id, width: bitmap.width, height: bitmap.height };
    }
    if (image instanceof global.HTMLCanvasElement) {
      surfaceOf(image);
      return { kind: "c", id: canvasId(image), width: image.width, height: image.height };
    }
    if (image instanceof global.HTMLImageElement) {
      const id = canvasId(image);
      const size = canvasSourceSize("i", id);
      if (size === null) return null;
      const [width, height] = size.split(" ").map(Number);
      return { kind: "i", id, width, height };
    }
    throw new TypeError("image attendue : un canevas, une image ou une ImageBitmap");
  };

  // ---- TextMetrics ----------------------------------------------------------------------
  const metrics = new WeakMap();
  class TextMetrics {
    constructor() {
      throw new TypeError("constructeur illégal");
    }
  }
  for (const name of ["width", "actualBoundingBoxLeft", "actualBoundingBoxRight", "actualBoundingBoxAscent",
    "actualBoundingBoxDescent", "fontBoundingBoxAscent", "fontBoundingBoxDescent", "emHeightAscent",
    "emHeightDescent", "hangingBaseline", "alphabeticBaseline", "ideographicBaseline"]) {
    define(TextMetrics.prototype, name, {
      get() { return metrics.get(this)[name]; },
      enumerable: true, configurable: true,
    });
  }
  tag(TextMetrics, "TextMetrics");
  expose("TextMetrics", TextMetrics);
  const makeMetrics = (values) => {
    const result = Object.create(TextMetrics.prototype);
    metrics.set(result, values);
    return result;
  };

  // ---- ImageData ------------------------------------------------------------------------
  class ImageData {
    #data;
    #width;
    #height;
    constructor(first, second, third = undefined) {
      if (arguments.length < 2) throw new TypeError("ImageData : deux arguments au moins sont requis");
      if (first instanceof Uint8ClampedArray) {
        const width = Number(second) >>> 0;
        if (width === 0) throw new DOMException("largeur nulle", "IndexSizeError");
        if (first.length === 0 || first.length % 4 !== 0) {
          throw new DOMException("la longueur des données doit être un multiple de 4", "InvalidStateError");
        }
        if ((first.length / 4) % width !== 0) {
          throw new DOMException("les données ne remplissent pas des lignes entières", "IndexSizeError");
        }
        const height = first.length / 4 / width;
        if (third !== undefined && typeof third !== "object" && (Number(third) >>> 0) !== height) {
          throw new DOMException("hauteur incohérente avec les données", "IndexSizeError");
        }
        this.#data = first;
        this.#width = width;
        this.#height = height;
      } else {
        const width = Number(first) >>> 0;
        const height = Number(second) >>> 0;
        if (width === 0 || height === 0) throw new DOMException("taille nulle", "IndexSizeError");
        this.#data = new Uint8ClampedArray(width * height * 4);
        this.#width = width;
        this.#height = height;
      }
    }
    get data() { return this.#data; }
    get width() { return this.#width; }
    get height() { return this.#height; }
    get colorSpace() { return "srgb"; }
  }
  tag(ImageData, "ImageData");
  expose("ImageData", ImageData);

  // Entier de 32 bits signé, comme l'exigent les méthodes de pixels : une
  // valeur non finie ou hors de cet intervalle est refusée.
  const long = (value) => {
    const number = Math.trunc(Number(value));
    if (!Number.isFinite(number) || number < -2147483648 || number > 2147483647) {
      throw new TypeError("entier de 32 bits attendu");
    }
    return number;
  };

  // ---- Contexte --------------------------------------------------------------------------
  const OPERATIONS = ["source-over", "source-in", "source-out", "source-atop", "destination-over",
    "destination-in", "destination-out", "destination-atop", "lighter", "copy", "xor", "multiply", "screen",
    "overlay", "darken", "lighten", "color-dodge", "color-burn", "hard-light", "soft-light", "difference",
    "exclusion", "hue", "saturation", "color", "luminosity"];
  const CLEAR = 26;
  const CAPS = ["butt", "round", "square"];
  const JOINS = ["miter", "round", "bevel"];
  const freshState = () => ({
    m: IDENTITY, fill: "#000000", stroke: "#000000", alpha: 1, operation: 0, lineWidth: 1, cap: 0, join: 0,
    miter: 10, dash: [], dashOffset: 0, clip: 0, font: "10px sans-serif", textAlign: "start",
    textBaseline: "alphabetic", direction: "inherit", smoothing: true, smoothingQuality: "low",
    shadowColor: "rgba(0, 0, 0, 0)", shadowBlur: 0, shadowOffsetX: 0, shadowOffsetY: 0, filter: "none",
  });
  // Surface de chaque canevas : sa taille, créée à la demande.
  const surfaces = new WeakMap();
  const contexts = new WeakMap();
  function surfaceOf(canvas) {
    const width = canvas.width;
    const height = canvas.height;
    let surface = surfaces.get(canvas);
    if (surface === undefined || surface.width !== width || surface.height !== height) {
      const id = idOf(canvas);
      surface = { id, width, height, live: canvasResize(id, width, height) };
      surfaces.set(canvas, surface);
      contexts.get(canvas)?.reset();
    }
    return surface;
  }

  class Context2D {
    #canvas;
    #state = freshState();
    #stack = [];
    #path = new Outline();
    constructor(canvas, key) {
      if (key !== contexts) throw new TypeError("constructeur illégal");
      this.#canvas = canvas;
    }
    get canvas() { return this.#canvas; }
    getContextAttributes() {
      return { alpha: true, colorSpace: "srgb", desynchronized: false, willReadFrequently: false };
    }
    // Identifiant de la surface, ou null si elle n'a pas pu être créée
    // (taille nulle ou refusée) : rien ne se dessine alors.
    #surface() {
      const surface = surfaceOf(this.#canvas);
      return surface.live ? surface.id : null;
    }
    #stateText(operation = this.#state.operation, alpha = this.#state.alpha) {
      const s = this.#state;
      return alpha + " " + operation + " " + s.clip + " " + s.m.join(" ");
    }

    // -- État
    save() {
      this.#stack.push({ ...this.#state, dash: this.#state.dash.slice() });
    }
    restore() {
      const saved = this.#stack.pop();
      if (saved !== undefined) this.#state = saved;
    }
    reset() {
      this.#state = freshState();
      this.#stack = [];
      this.#path = new Outline();
      const surface = surfaces.get(this.#canvas);
      if (surface?.live) canvasResize(surface.id, surface.width, surface.height);
    }
    isContextLost() { return false; }

    // -- Transformation
    #concat(n) {
      if (!finite(...n)) return;
      this.#state.m = multiply(this.#state.m, n);
    }
    scale(x, y) { this.#concat([Number(x), 0, 0, Number(y), 0, 0]); }
    rotate(angle) {
      const cos = Math.cos(angle);
      const sin = Math.sin(angle);
      this.#concat([cos, sin, -sin, cos, 0, 0]);
    }
    translate(x, y) { this.#concat([1, 0, 0, 1, Number(x), Number(y)]); }
    transform(a, b, c, d, e, f) { this.#concat([a, b, c, d, e, f].map(Number)); }
    setTransform(a = undefined, b, c, d, e, f) {
      const n = typeof a === "object" && a !== null ? matrixOf(a)
        : a === undefined ? IDENTITY : [a, b, c, d, e, f].map(Number);
      if (finite(...n)) this.#state.m = n;
    }
    resetTransform() { this.#state.m = IDENTITY; }
    getTransform() {
      const [a, b, c, d, e, f] = this.#state.m;
      return typeof global.DOMMatrix === "function" ? new global.DOMMatrix([a, b, c, d, e, f])
        : { a, b, c, d, e, f, is2D: true, isIdentity: a === 1 && b === 0 && c === 0 && d === 1 && e === 0 && f === 0 };
    }

    // -- Styles
    get fillStyle() { return this.#state.fill; }
    set fillStyle(value) { this.#setStyle("fill", value); }
    get strokeStyle() { return this.#state.stroke; }
    set strokeStyle(value) { this.#setStyle("stroke", value); }
    #setStyle(which, value) {
      if (gradients.has(value) || patterns.has(value)) {
        this.#state[which] = value;
        return;
      }
      if (typeof value !== "string") return;
      const parsed = colorOf(value);
      if (parsed !== null) this.#state[which] = serializeColor(parsed);
    }
    get globalAlpha() { return this.#state.alpha; }
    set globalAlpha(value) {
      const alpha = Number(value);
      if (alpha >= 0 && alpha <= 1) this.#state.alpha = alpha;
    }
    get globalCompositeOperation() { return OPERATIONS[this.#state.operation]; }
    set globalCompositeOperation(value) {
      const rank = OPERATIONS.indexOf(String(value));
      if (rank >= 0) this.#state.operation = rank;
    }
    get lineWidth() { return this.#state.lineWidth; }
    set lineWidth(value) {
      const width = Number(value);
      if (width > 0 && Number.isFinite(width)) this.#state.lineWidth = width;
    }
    get lineCap() { return CAPS[this.#state.cap]; }
    set lineCap(value) {
      const rank = CAPS.indexOf(String(value));
      if (rank >= 0) this.#state.cap = rank;
    }
    get lineJoin() { return JOINS[this.#state.join]; }
    set lineJoin(value) {
      const rank = JOINS.indexOf(String(value));
      if (rank >= 0) this.#state.join = rank;
    }
    get miterLimit() { return this.#state.miter; }
    set miterLimit(value) {
      const limit = Number(value);
      if (limit > 0 && Number.isFinite(limit)) this.#state.miter = limit;
    }
    get lineDashOffset() { return this.#state.dashOffset; }
    set lineDashOffset(value) {
      const offset = Number(value);
      if (Number.isFinite(offset)) this.#state.dashOffset = offset;
    }
    getLineDash() { return this.#state.dash.slice(); }
    setLineDash(segments) {
      const list = Array.from(segments, Number);
      if (list.some((segment) => !(segment >= 0) || !Number.isFinite(segment))) return;
      // Un nombre impair de segments est répété une fois.
      this.#state.dash = list.length % 2 === 1 ? list.concat(list) : list;
    }
    get imageSmoothingEnabled() { return this.#state.smoothing; }
    set imageSmoothingEnabled(value) { this.#state.smoothing = Boolean(value); }
    get imageSmoothingQuality() { return this.#state.smoothingQuality; }
    set imageSmoothingQuality(value) {
      if (["low", "medium", "high"].includes(value)) this.#state.smoothingQuality = value;
    }
    get font() { return this.#state.font; }
    set font(value) {
      if (/\d(px|pt|em|rem|%)/.test(String(value))) this.#state.font = String(value);
    }
    get textAlign() { return this.#state.textAlign; }
    set textAlign(value) {
      if (["start", "end", "left", "right", "center"].includes(value)) this.#state.textAlign = value;
    }
    get textBaseline() { return this.#state.textBaseline; }
    set textBaseline(value) {
      if (["top", "hanging", "middle", "alphabetic", "ideographic", "bottom"].includes(value)) {
        this.#state.textBaseline = value;
      }
    }
    get direction() { return this.#state.direction; }
    set direction(value) {
      if (["ltr", "rtl", "inherit"].includes(value)) this.#state.direction = value;
    }
    get shadowColor() { return this.#state.shadowColor; }
    set shadowColor(value) {
      const parsed = colorOf(value);
      if (parsed !== null) this.#state.shadowColor = serializeColor(parsed);
    }
    get shadowBlur() { return this.#state.shadowBlur; }
    set shadowBlur(value) {
      const blur = Number(value);
      if (blur >= 0 && Number.isFinite(blur)) this.#state.shadowBlur = blur;
    }
    get shadowOffsetX() { return this.#state.shadowOffsetX; }
    set shadowOffsetX(value) {
      if (Number.isFinite(Number(value))) this.#state.shadowOffsetX = Number(value);
    }
    get shadowOffsetY() { return this.#state.shadowOffsetY; }
    set shadowOffsetY(value) {
      if (Number.isFinite(Number(value))) this.#state.shadowOffsetY = Number(value);
    }
    get filter() { return this.#state.filter; }
    set filter(value) { this.#state.filter = String(value); }
    createLinearGradient(x0, y0, x1, y1) {
      const coords = [x0, y0, x1, y1].map(Number);
      if (arguments.length < 4 || !finite(...coords)) throw new TypeError("coordonnées finies attendues");
      return makeGradient("l", coords);
    }
    createRadialGradient(x0, y0, r0, x1, y1, r1) {
      const coords = [x0, y0, r0, x1, y1, r1].map(Number);
      if (arguments.length < 6 || !finite(...coords)) throw new TypeError("coordonnées finies attendues");
      if (coords[2] < 0 || coords[5] < 0) throw new DOMException("rayon négatif", "IndexSizeError");
      return makeGradient("r", coords);
    }
    createPattern(image, repetition) {
      if (arguments.length < 2) throw new TypeError("createPattern : deux arguments sont requis");
      const kind = repetition === null || repetition === "" ? "repeat" : String(repetition);
      if (!["repeat", "repeat-x", "repeat-y", "no-repeat"].includes(kind)) {
        throw new DOMException("répétition inconnue", "SyntaxError");
      }
      const source = sourceOf(image);
      if (source === null) return null;
      if (source.width === 0 || source.height === 0) {
        throw new DOMException("image sans surface", "InvalidStateError");
      }
      const pattern = Object.create(CanvasPattern.prototype);
      patterns.set(pattern, source);
      return pattern;
    }

    // -- Tracé courant
    beginPath() { this.#path = new Outline(); }
    // Tracé à dessiner : celui qu'on passe, ramené dans le repère de la
    // surface, sinon le tracé courant.
    #outline(path) {
      if (!(path instanceof Path2D)) return this.#path;
      const outline = new Outline();
      outline.append(outlines.get(path), this.#state.m);
      return outline;
    }
    fill(first = undefined, second = undefined) {
      const rule = first instanceof Path2D ? second : first;
      if (rule !== undefined && rule !== "nonzero" && rule !== "evenodd") {
        throw new TypeError("règle de remplissage inconnue");
      }
      const id = this.#surface();
      if (id === null) return;
      canvasFill(id, this.#outline(first).text(), paintText(this.#state.fill), this.#stateText(),
        rule === "evenodd");
    }
    #strokeText() {
      const s = this.#state;
      return [s.lineWidth, s.cap, s.join, s.miter, s.dashOffset, ...s.dash].join(" ");
    }
    stroke(path = undefined) {
      const id = this.#surface();
      if (id === null) return;
      canvasStroke(id, this.#outline(path).text(), paintText(this.#state.stroke), this.#stateText(),
        this.#strokeText());
    }
    clip(first = undefined, second = undefined) {
      const rule = first instanceof Path2D ? second : first;
      if (rule !== undefined && rule !== "nonzero" && rule !== "evenodd") {
        throw new TypeError("règle de remplissage inconnue");
      }
      const id = this.#surface();
      if (id === null) return;
      this.#state.clip = canvasClip(id, this.#outline(first).text(), rule === "evenodd", this.#state.clip);
    }
    isPointInPath(first, second, third = undefined, fourth = undefined) {
      // Deux formes : (x, y, règle) et (tracé, x, y, règle).
      const given = arguments.length >= 4 || (arguments.length === 3 && typeof third !== "string")
        || (typeof first === "object" && first !== null);
      if (given && !(first instanceof Path2D)) throw new TypeError("isPointInPath : un tracé est attendu");
      const [x, y, rule] = given ? [second, third, fourth] : [first, second, third];
      if (rule !== undefined && rule !== "nonzero" && rule !== "evenodd") {
        throw new TypeError("règle de remplissage inconnue");
      }
      if (!finite(Number(x), Number(y))) return false;
      return this.#outline(given ? first : undefined).contains(Number(x), Number(y), rule === "evenodd");
    }
    isPointInStroke() { return false; }

    // -- Rectangles
    #rect(x, y, w, h) {
      const outline = new Outline();
      outline.rect(this.#state.m, x, y, w, h);
      return outline.text();
    }
    fillRect(x, y, w, h) {
      [x, y, w, h] = [x, y, w, h].map(Number);
      const id = this.#surface();
      if (id === null || !finite(x, y, w, h) || w === 0 || h === 0) return;
      canvasFill(id, this.#rect(x, y, w, h), paintText(this.#state.fill), this.#stateText(), false);
    }
    strokeRect(x, y, w, h) {
      [x, y, w, h] = [x, y, w, h].map(Number);
      const id = this.#surface();
      if (id === null || !finite(x, y, w, h) || (w === 0 && h === 0)) return;
      canvasStroke(id, this.#rect(x, y, w, h), paintText(this.#state.stroke), this.#stateText(),
        this.#strokeText());
    }
    clearRect(x, y, w, h) {
      [x, y, w, h] = [x, y, w, h].map(Number);
      const id = this.#surface();
      if (id === null || !finite(x, y, w, h) || w === 0 || h === 0) return;
      canvasFill(id, this.#rect(x, y, w, h), "c 0 0 0 1", this.#stateText(CLEAR, 1), false);
    }

    // -- Images
    drawImage(image, ...numbers) {
      if (![2, 4, 8].includes(numbers.length)) throw new TypeError("drawImage : 3, 5 ou 9 arguments");
      const source = sourceOf(image);
      if (source === null) return;
      if (source.width === 0 || source.height === 0) {
        throw new DOMException("image sans surface", "InvalidStateError");
      }
      let [sx, sy, sw, sh, dx, dy, dw, dh] = numbers.length === 8 ? numbers.map(Number)
        : [0, 0, source.width, source.height, Number(numbers[0]), Number(numbers[1]),
          Number(numbers[2] ?? source.width), Number(numbers[3] ?? source.height)];
      if (!finite(sx, sy, sw, sh, dx, dy, dw, dh)) return;
      // Largeurs et hauteurs négatives : le rectangle est pris dans l'autre sens.
      if (sw < 0) { sx += sw; sw = -sw; }
      if (sh < 0) { sy += sh; sh = -sh; }
      if (dw < 0) { dx += dw; dw = -dw; }
      if (dh < 0) { dy += dh; dh = -dh; }
      if (sw === 0 || sh === 0) return;
      // Ce qui dépasse de la source n'est pas dessiné.
      const left = Math.max(0, sx);
      const top = Math.max(0, sy);
      const right = Math.min(source.width, sx + sw);
      const bottom = Math.min(source.height, sy + sh);
      if (right <= left || bottom <= top) return;
      const scaleX = dw / sw;
      const scaleY = dh / sh;
      const id = this.#surface();
      if (id === null) return;
      canvasImage(id, source.kind, source.id,
        [left, top, right - left, bottom - top, dx + (left - sx) * scaleX, dy + (top - sy) * scaleY,
          (right - left) * scaleX, (bottom - top) * scaleY].join(" "),
        this.#stateText(), this.#state.smoothing);
    }

    // -- Pixels
    createImageData(first, second = undefined) {
      if (first instanceof ImageData) return new ImageData(first.width, first.height);
      if (arguments.length < 2) throw new TypeError("createImageData : une ImageData ou deux tailles");
      const width = Math.abs(long(first));
      const height = Math.abs(long(second));
      if (width === 0 || height === 0) throw new DOMException("taille nulle", "IndexSizeError");
      return new ImageData(width, height);
    }
    getImageData(sx, sy, sw, sh) {
      if (arguments.length < 4) throw new TypeError("getImageData : quatre nombres sont requis");
      [sx, sy, sw, sh] = [sx, sy, sw, sh].map(long);
      if (sw === 0 || sh === 0) throw new DOMException("taille nulle", "IndexSizeError");
      if (sw < 0) { sx += sw; sw = -sw; }
      if (sh < 0) { sy += sh; sh = -sh; }
      const image = new ImageData(sw, sh);
      const id = this.#surface();
      const pixels = id === null ? null : canvasGetPixels(id, sx, sy, sw, sh);
      if (pixels !== null) {
        const data = image.data;
        for (let i = 0; i < pixels.length; i++) data[i] = pixels.charCodeAt(i);
      }
      return image;
    }
    putImageData(image, dx, dy, dirtyX = 0, dirtyY = 0, dirtyWidth = undefined, dirtyHeight = undefined) {
      if (!(image instanceof ImageData)) throw new TypeError("putImageData : une ImageData est attendue");
      let x = Math.trunc(Number(dirtyX));
      let y = Math.trunc(Number(dirtyY));
      let w = dirtyWidth === undefined ? image.width : Math.trunc(Number(dirtyWidth));
      let h = dirtyHeight === undefined ? image.height : Math.trunc(Number(dirtyHeight));
      dx = Math.trunc(Number(dx));
      dy = Math.trunc(Number(dy));
      if (!finite(dx, dy, x, y, w, h)) return;
      if (w < 0) { x += w; w = -w; }
      if (h < 0) { y += h; h = -h; }
      if (x < 0) { w += x; x = 0; }
      if (y < 0) { h += y; y = 0; }
      w = Math.min(w, image.width - x);
      h = Math.min(h, image.height - y);
      const id = this.#surface();
      if (id === null || w <= 0 || h <= 0) return;
      canvasPutPixels(id, toBinary(image.data), image.width, dx, dy, x, y, w, h);
    }

    // -- Texte
    #text(text, x, y, maxWidth, stroked) {
      [x, y] = [Number(x), Number(y)];
      const limit = maxWidth === undefined ? -1 : Number(maxWidth);
      const id = this.#surface();
      if (id === null || !finite(x, y) || Number.isNaN(limit) || (maxWidth !== undefined && limit <= 0)) return;
      const s = this.#state;
      // Tabulations et fins de ligne valent des espaces.
      const line = String(text).replace(/[\t\n\f\r]/g, " ");
      const align = { center: 1, right: 2, end: 2 }[s.textAlign] ?? 0;
      const baseline = { top: 1, hanging: 1, middle: 2, bottom: 3, ideographic: 3 }[s.textBaseline] ?? 0;
      canvasText(id, line, s.font, [x, y, Number.isFinite(limit) ? limit : -1, align, baseline].join(" "),
        paintText(stroked ? s.stroke : s.fill), this.#stateText(), stroked ? this.#strokeText() : "");
    }
    fillText(text, x, y, maxWidth = undefined) {
      if (arguments.length < 3) throw new TypeError("fillText : le texte et sa position sont requis");
      this.#text(text, x, y, maxWidth, false);
    }
    strokeText(text, x, y, maxWidth = undefined) {
      if (arguments.length < 3) throw new TypeError("strokeText : le texte et sa position sont requis");
      this.#text(text, x, y, maxWidth, true);
    }
    measureText(text) {
      if (arguments.length < 1) throw new TypeError("measureText : un texte est requis");
      const s = this.#state;
      const measured = canvasMeasure(String(text).replace(/[\t\n\f\r]/g, " "), s.font);
      const [width, ascent, descent] = measured === null ? [0, 0, 0] : measured.split(" ").map(Number);
      // Décalage de l'ancre selon l'alignement et la ligne de base.
      const left = { center: width / 2, right: width, end: width }[s.textAlign] ?? 0;
      const raised = { top: ascent, hanging: ascent, middle: (ascent - descent) / 2, bottom: -descent,
        ideographic: -descent }[s.textBaseline] ?? 0;
      return makeMetrics({
        width,
        actualBoundingBoxLeft: left,
        actualBoundingBoxRight: width - left,
        actualBoundingBoxAscent: ascent - raised,
        actualBoundingBoxDescent: descent + raised,
        fontBoundingBoxAscent: ascent - raised,
        fontBoundingBoxDescent: descent + raised,
        emHeightAscent: ascent - raised,
        emHeightDescent: descent + raised,
        hangingBaseline: ascent - raised,
        alphabeticBaseline: -raised,
        ideographicBaseline: -descent - raised,
      });
    }
    // Les méthodes de tracé agissent sur le tracé courant, dans le repère courant.
    static {
      for (const name of PATH_METHODS) {
        define(Context2D.prototype, name, {
          value: {
            [name](...args) {
              this.#path[name](this.#state.m, ...args.map(numberOrRadii));
            },
          }[name],
          writable: true, enumerable: true, configurable: true,
        });
      }
    }
  }
  // Les deux interfaces de contexte ont chacune leurs membres en propre, mais
  // le même comportement : celui de `Context2D`, dont chaque contexte est
  // une instance rattachée ensuite au prototype de son interface.
  const contextInterface = (name) => {
    const Interface = { [name]: function () { throw new TypeError("constructeur illégal"); } }[name];
    for (const key of Reflect.ownKeys(Context2D.prototype)) {
      if (key === "constructor") continue;
      define(Interface.prototype, key, Object.getOwnPropertyDescriptor(Context2D.prototype, key));
    }
    define(Interface, "prototype", { writable: false });
    return Interface;
  };
  const CanvasRenderingContext2D = contextInterface("CanvasRenderingContext2D");
  const OffscreenCanvasRenderingContext2D = contextInterface("OffscreenCanvasRenderingContext2D");
  const makeContext = (canvas, Interface) => {
    const context = new Context2D(canvas, contexts);
    Object.setPrototypeOf(context, Interface.prototype);
    return context;
  };
  tag(OffscreenCanvasRenderingContext2D, "OffscreenCanvasRenderingContext2D");
  expose("OffscreenCanvasRenderingContext2D", OffscreenCanvasRenderingContext2D);
  tag(CanvasRenderingContext2D, "CanvasRenderingContext2D");
  expose("CanvasRenderingContext2D", CanvasRenderingContext2D);

  // ---- Élément <canvas> ------------------------------------------------------------------
  const Canvas = global.HTMLCanvasElement;
  define(Canvas.prototype, "getContext", {
    value: function getContext(kind) {
      if (String(kind) !== "2d") return null;
      let context = contexts.get(this);
      if (context === undefined) {
        context = makeContext(this, CanvasRenderingContext2D);
        contexts.set(this, context);
        surfaceOf(this);
      }
      return context;
    },
    writable: true, enumerable: true, configurable: true,
  });
  // Fixer la largeur ou la hauteur efface le canevas, même sans changement.
  for (const side of ["width", "height"]) {
    const inherited = Object.getOwnPropertyDescriptor(Canvas.prototype, side);
    define(Canvas.prototype, side, {
      get: inherited.get,
      set(value) {
        inherited.set.call(this, value);
        surfaces.delete(this);
        if (contexts.has(this)) surfaceOf(this);
      },
      enumerable: true, configurable: true,
    });
  }
  const pngOf = (canvas) => {
    if (canvas.width === 0 || canvas.height === 0) return null;
    const surface = surfaceOf(canvas);
    return surface.live ? canvasPng(surface.id) : null;
  };
  define(Canvas.prototype, "toDataURL", {
    value: function toDataURL() {
      const png = pngOf(this);
      return png === null ? "data:," : "data:image/png;base64," + global.btoa(png);
    },
    writable: true, enumerable: true, configurable: true,
  });
  define(Canvas.prototype, "toBlob", {
    value: function toBlob(callback) {
      if (typeof callback !== "function") throw new TypeError("toBlob : une fonction de rappel est attendue");
      const png = pngOf(this);
      global.setTimeout(() => {
        if (png === null) {
          callback(null);
          return;
        }
        const bytes = new Uint8Array(png.length);
        for (let i = 0; i < png.length; i++) bytes[i] = png.charCodeAt(i);
        callback(new global.Blob([bytes], { type: "image/png" }));
      }, 0);
    },
    writable: true, enumerable: true, configurable: true,
  });

  // ---- OffscreenCanvas -------------------------------------------------------------------
  // Taille d'un canevas : un entier non signé de 32 bits.
  const side = (value) => {
    const number = Number(value);
    if (!Number.isFinite(number) && value !== undefined) throw new TypeError("taille de canevas invalide");
    return number >>> 0;
  };
  class OffscreenCanvas extends global.EventTarget {
    #width;
    #height;
    constructor(width, height) {
      super();
      if (arguments.length < 2) throw new TypeError("OffscreenCanvas : largeur et hauteur requises");
      this.#width = side(width);
      this.#height = side(height);
      allocate(this);
      this.oncontextlost = null;
      this.oncontextrestored = null;
    }
    get width() { return this.#width; }
    set width(value) {
      this.#width = side(value);
      surfaces.delete(this);
      if (contexts.has(this)) surfaceOf(this);
    }
    get height() { return this.#height; }
    set height(value) {
      this.#height = side(value);
      surfaces.delete(this);
      if (contexts.has(this)) surfaceOf(this);
    }
    getContext(kind) {
      if (String(kind) !== "2d") return null;
      let context = contexts.get(this);
      if (context === undefined) {
        context = makeContext(this, OffscreenCanvasRenderingContext2D);
        contexts.set(this, context);
        surfaceOf(this);
      }
      return context;
    }
    convertToBlob() {
      return new Promise((resolve, reject) => {
        const png = pngOf(this);
        if (png === null) {
          reject(new DOMException("canevas sans surface", "IndexSizeError"));
          return;
        }
        const bytes = new Uint8Array(png.length);
        for (let i = 0; i < png.length; i++) bytes[i] = png.charCodeAt(i);
        resolve(new global.Blob([bytes], { type: "image/png" }));
      });
    }
    transferToImageBitmap() {
      if (!contexts.has(this)) throw new DOMException("canevas sans contexte", "InvalidStateError");
      const bitmap = copyBitmap({ kind: "c", id: surfaceOf(this).id, width: this.#width, height: this.#height });
      // Le canevas repart d'une surface vide.
      surfaces.delete(this);
      surfaceOf(this);
      return bitmap;
    }
  }
  tag(OffscreenCanvas, "OffscreenCanvas");
  expose("OffscreenCanvas", OffscreenCanvas);

  // ---- ImageBitmap -----------------------------------------------------------------------
  class ImageBitmap {
    constructor() {
      throw new TypeError("constructeur illégal");
    }
    get width() {
      const bitmap = bitmaps.get(this);
      return bitmap.closed ? 0 : bitmap.width;
    }
    get height() {
      const bitmap = bitmaps.get(this);
      return bitmap.closed ? 0 : bitmap.height;
    }
    close() {
      const bitmap = bitmaps.get(this);
      if (bitmap.closed) return;
      bitmap.closed = true;
      canvasResize(bitmap.id, 0, 0);
    }
  }
  tag(ImageBitmap, "ImageBitmap");
  expose("ImageBitmap", ImageBitmap);
  const makeBitmap = (width, height) => {
    const image = Object.create(ImageBitmap.prototype);
    const id = allocate(image);
    bitmaps.set(image, { id, width, height, closed: false });
    return image;
  };
  // Copie dans une ImageBitmap neuve du rectangle (`sx`, `sy`, `sw`, `sh`) de
  // la source, ramené à la taille (`width`, `height`), retourné si `flip`.
  function copyBitmap(source, sx = 0, sy = 0, sw = source.width, sh = source.height, width = sw, height = sh,
    flip = false) {
    const image = makeBitmap(width, height);
    const { id } = bitmaps.get(image);
    if (!canvasResize(id, width, height)) {
      throw new DOMException("image trop grande ou sans surface", "InvalidStateError");
    }
    const m = flip ? [1, 0, 0, -1, 0, height] : IDENTITY;
    canvasImage(id, source.kind, source.id, [sx, sy, sw, sh, 0, 0, width, height].join(" "),
      "1 9 0 " + m.join(" "), true);
    return image;
  }
  expose("createImageBitmap", function createImageBitmap(image, ...rest) {
    return new Promise((resolve) => {
      if (arguments.length < 1) throw new TypeError("createImageBitmap : une source est requise");
      const cropped = rest.length >= 4;
      if (rest.length > 1 && !cropped) throw new TypeError("createImageBitmap : 1, 2, 5 ou 6 arguments");
      const options = (cropped ? rest[4] : rest[0]) ?? {};
      let [sx, sy, sw, sh] = cropped ? rest.slice(0, 4).map((value) => Math.trunc(Number(value))) : [];
      if (cropped) {
        if (!finite(sx, sy, sw, sh)) throw new TypeError("rectangle fini attendu");
        if (sw === 0 || sh === 0) throw new RangeError("rectangle de taille nulle");
        if (sw < 0) { sx += sw; sw = -sw; }
        if (sh < 0) { sy += sh; sh = -sh; }
      }
      for (const key of ["resizeWidth", "resizeHeight"]) {
        if (options[key] !== undefined && !((Number(options[key]) >>> 0) > 0)) {
          throw new DOMException("taille demandée nulle", "InvalidStateError");
        }
      }
      const finish = (source) => {
        if (source === null || source.width === 0 || source.height === 0) {
          throw new DOMException("source sans surface", "InvalidStateError");
        }
        const fromX = cropped ? sx : 0;
        const fromY = cropped ? sy : 0;
        const fromWidth = cropped ? sw : source.width;
        const fromHeight = cropped ? sh : source.height;
        let width = options.resizeWidth === undefined ? undefined : Number(options.resizeWidth) >>> 0;
        let height = options.resizeHeight === undefined ? undefined : Number(options.resizeHeight) >>> 0;
        // Une seule dimension donnée : l'autre garde les proportions.
        if (width === undefined && height === undefined) {
          width = fromWidth;
          height = fromHeight;
        } else if (width === undefined) {
          width = Math.max(1, Math.ceil((fromWidth * height) / fromHeight));
        } else if (height === undefined) {
          height = Math.max(1, Math.ceil((fromHeight * width) / fromWidth));
        }
        return copyBitmap(source, fromX, fromY, fromWidth, fromHeight, width, height,
          options.imageOrientation === "flipY");
      };
      if (image instanceof ImageData) {
        // Les pixels passent par une surface provisoire.
        const holder = makeBitmap(image.width, image.height);
        const { id } = bitmaps.get(holder);
        if (!canvasResize(id, image.width, image.height)) {
          throw new DOMException("image trop grande", "InvalidStateError");
        }
        canvasPutPixels(id, toBinary(image.data), image.width, 0, 0, 0, 0, image.width, image.height);
        resolve(finish({ kind: "c", id, width: image.width, height: image.height }));
      } else if (image instanceof global.Blob) {
        resolve(image.arrayBuffer().then((buffer) => {
          const holder = makeBitmap(0, 0);
          const state = bitmaps.get(holder);
          const size = canvasDecode(state.id, toBinary(new Uint8Array(buffer)));
          if (size === null) throw new DOMException("image illisible", "InvalidStateError");
          [state.width, state.height] = size.split(" ").map(Number);
          return finish({ kind: "c", id: state.id, width: state.width, height: state.height });
        }));
      } else {
        resolve(finish(sourceOf(image)));
      }
    });
  });
})
