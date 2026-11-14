// Micro-bancs du moteur JavaScript : chaque essai rend un temps en ms.
const essais = {
  "boucle arithmétique (5 M)": () => { let s = 0; for (let i = 0; i < 5000000; i++) s = (s + i * 2) % 1000003; return s; },
  "appels de fonction (2 M)": () => { const f = (a, b) => a + b; let s = 0; for (let i = 0; i < 2000000; i++) s = f(s, i) & 0xffff; return s; },
  "lecture de propriété (3 M)": () => { const o = { a: 1, b: 2, c: 3 }; let s = 0; for (let i = 0; i < 3000000; i++) s += o.b; return s; },
  "appel de méthode (2 M)": () => { class P { constructor() { this.v = 1; } m(x) { return this.v + x; } } const p = new P(); let s = 0; for (let i = 0; i < 2000000; i++) s = p.m(s) & 0xffff; return s; },
  "allocation d'objets (500 k)": () => { let s = 0; for (let i = 0; i < 500000; i++) { const o = { v: i, w: [i, i] }; s += o.w.length; } return s; },
  "fermetures (1 M)": () => { let s = 0; for (let i = 0; i < 1000000; i++) { const g = () => i + 1; s += g(); } return s; },
  "tableau push et somme (1 M)": () => { const t = []; for (let i = 0; i < 1000000; i++) t.push(i); let s = 0; for (let i = 0; i < t.length; i++) s += t[i]; return s; },
  "map/filter/reduce (300 k)": () => { const t = Array.from({ length: 300000 }, (_, i) => i); return t.map((x) => x * 2).filter((x) => x % 3 === 0).reduce((a, b) => a + b, 0); },
  "concaténation de chaînes (200 k)": () => { let s = ""; for (let i = 0; i < 200000; i++) s += "ab"; return s.length; },
  "Map set/get (300 k)": () => { const m = new Map(); for (let i = 0; i < 300000; i++) m.set("k" + i, i); let s = 0; for (let i = 0; i < 300000; i++) s += m.get("k" + i); return s; },
  "variables globales (2 M)": () => { globalThis.compteur = 0; for (let i = 0; i < 2000000; i++) compteur = compteur + 1; return compteur; },
  "try/catch sans erreur (2 M)": () => { let s = 0; for (let i = 0; i < 2000000; i++) { try { s += i; } catch (e) { s = 0; } } return s; },
  "expression rationnelle (100 k)": () => { let s = 0; const re = /(\d+)-(\w+)/; for (let i = 0; i < 100000; i++) { if (re.test("abc 123-def")) s++; } return s; },
  "JSON aller-retour (20 k)": () => { let s = 0; const o = { a: [1, 2, 3], b: "texte", c: { d: true } }; for (let i = 0; i < 20000; i++) s += JSON.parse(JSON.stringify(o)).a.length; return s; },
};
function lancer(nom) { const t0 = Date.now(); essais[nom](); console.log(nom + " : " + (Date.now() - t0) + " ms"); }
