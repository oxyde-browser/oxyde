// Lance les mêmes essais sous Node : `node bench/moteur-js/node.js`
// (ajouter `--jitless` pour l'interpréteur de V8 seul).
const fs = require("node:fs");
const path = require("node:path");
const source = fs.readFileSync(path.join(__dirname, "essais.js"), "utf8");
new Function(source + "\nfor (const nom of Object.keys(essais)) lancer(nom);")();
