//! Tests de bout en bout du moteur : du source à la valeur.

use crate::{EvalError, Value, Vm, with_large_stack};

/// Évalue `source` et renvoie la représentation de son résultat.
fn eval(source: &str) -> String {
    let mut vm = Vm::new();
    match vm.eval(source) {
        Ok(value) => vm.display(&value),
        Err(error) => format!("ERREUR {error}"),
    }
}

#[test]
fn arithmetic_and_strings() {
    assert_eq!(eval("1 + 2 * 3"), "7");
    assert_eq!(eval("'a' + 1 + 2"), "a12");
    assert_eq!(eval("1 + 2 + 'a'"), "3a");
    assert_eq!(eval("7 % 3 + 2 ** 10 - 10 / 4"), "1022.5");
    assert_eq!(eval("0.1 + 0.2"), "0.30000000000000004");
    assert_eq!(eval("(5 >> 1) | (1 << 4) ^ 3"), "19");
    assert_eq!(eval("-1 >>> 28"), "15");
    assert_eq!(eval("`a${1 + 1}b${'c'}`"), "a2bc");
    assert_eq!(
        eval("typeof 1 + typeof 'a' + typeof null + typeof undefined + typeof (() => 1)"),
        "numberstringobjectundefinedfunction"
    );
    assert_eq!(eval("typeof inconnu"), "undefined");
    assert_eq!(
        eval("1 < 2 && 'a' < 'b' && !(NaN <= NaN) && null == undefined && null !== undefined"),
        "true"
    );
    assert_eq!(eval("'5' * '2' + +'3' - -'1'"), "14");
    assert_eq!(eval("[] + [] + 'x' + [1, 2] + {}"), "x1,2[object Object]");
}

#[test]
fn variables_scopes_and_closures() {
    assert_eq!(eval("let a = 1; { let a = 2; } a"), "1");
    assert_eq!(
        eval("var x = 1; function f() { return x; var x = 2; } f()"),
        "undefined"
    );
    assert_eq!(
        eval("function make() { let n = 0; return () => ++n; } const c = make(); c(); c(); c()"),
        "3"
    );
    // Chaque tour de boucle a sa propre liaison.
    assert_eq!(
        eval(
            "const fs = []; for (let i = 0; i < 3; i++) fs.push(() => i); fs.map(f => f()).join()"
        ),
        "0,1,2"
    );
    assert_eq!(
        eval("var gs = []; for (var j = 0; j < 3; j++) gs.push(() => j); gs.map(f => f()).join()"),
        "3,3,3"
    );
    assert_eq!(
        eval("const hs = []; for (const k of [1, 2]) hs.push(() => k); hs[0]() + hs[1]()"),
        "3"
    );
    assert_eq!(
        eval(
            "function outer() { let v = 1; function inc() { v++; } inc(); inc(); return v; } outer()"
        ),
        "3"
    );
    assert_eq!(
        eval("function f() { return g(); function g() { return 42; } } f()"),
        "42"
    );
    assert_eq!(
        eval("(function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); })(10)"),
        "3628800"
    );
    assert!(eval("{ x; let x = 1; }").contains("ReferenceError"));
    assert!(eval("function f() { const c = 1; c = 2; } f()").contains("TypeError"));
    assert!(eval("inconnu + 1").contains("ReferenceError"));
}

#[test]
fn control_flow() {
    assert_eq!(
        eval(
            "let s = 0; for (let i = 0; i < 10; i++) { if (i % 2) continue; if (i > 6) break; s += i; } s"
        ),
        "12"
    );
    assert_eq!(
        eval("let n = 0; while (true) { if (++n >= 5) break; } n"),
        "5"
    );
    assert_eq!(eval("let m = 0; do { m++; } while (m < 3); m"), "3");
    assert_eq!(
        eval(
            "let r = ''; outer: for (const a of [1, 2, 3]) { for (const b of [1, 2, 3]) { \
             if (b === 2) continue outer; if (a === 3) break outer; r += a + '' + b + ' '; } } r"
        ),
        "11 21 "
    );
    assert_eq!(
        eval(
            "function f(x) { switch (x) { case 1: return 'un'; case 2: case 3: return 'deux-trois'; \
             default: return 'autre'; } } f(1) + f(3) + f(9)"
        ),
        "undeux-troisautre"
    );
    assert_eq!(
        eval(
            "let t = ''; switch (2) { case 1: t += 'a'; case 2: t += 'b'; case 3: t += 'c'; break; case 4: t += 'd'; } t"
        ),
        "bc"
    );
    assert_eq!(
        eval("const o = {a: 1, b: 2}; let ks = ''; for (const k in o) ks += k; ks"),
        "ab"
    );
    assert_eq!(
        eval("let q = ''; for (const c of 'héé') q += c + '.'; q"),
        "h.é.é."
    );
    assert_eq!(eval("1 ? 2 ? 'a' : 'b' : 'c'"), "a");
    assert_eq!(eval("null ?? 'x'"), "x");
    assert_eq!(eval("0 || '' || 'y'"), "y");
    assert_eq!(eval("let z = 0; z ||= 5; z &&= z + 1; z ??= 9; z"), "6");
}

#[test]
fn exceptions() {
    assert_eq!(
        eval("try { throw new Error('boum'); } catch (e) { e.message }"),
        "boum"
    );
    assert_eq!(
        eval("let log = ''; try { log += 'a'; } finally { log += 'b'; } log"),
        "ab"
    );
    assert_eq!(
        eval("function f() { try { return 'try'; } finally { g = 'fin'; } } var g; f() + g"),
        "tryfin"
    );
    assert_eq!(
        eval(
            "function f() { for (const i of [1, 2]) { try { if (i === 1) continue; return i; } \
             finally { n++; } } } var n = 0; f() + ':' + n"
        ),
        "2:2"
    );
    assert_eq!(
        eval("let s = ''; try { try { throw 1; } finally { s += 'f'; } } catch (e) { s += e; } s"),
        "f1"
    );
    assert_eq!(
        eval(
            "let s = ''; try { try { throw 1; } catch (e) { throw e + 1; } finally { s += 'f'; } } catch (e) { s += e; } s"
        ),
        "f2"
    );
    assert_eq!(
        eval("try { null.x } catch (e) { e instanceof TypeError }"),
        "true"
    );
    assert_eq!(
        eval("try { undefined() } catch (e) { e.name }"),
        "TypeError"
    );
    assert_eq!(
        eval(
            "class E extends Error { constructor(m) { super(m); this.name = 'E'; } } \
             try { throw new E('x') } catch (e) { e instanceof E && e instanceof Error && String(e) }"
        ),
        "E: x"
    );
    assert_eq!(
        eval("throw new RangeError('trop')"),
        "ERREUR RangeError: trop"
    );
    assert_eq!(
        eval("function f() { throw 'chaîne'; } try { f() } catch { 'vu' }"),
        "vu"
    );
}

#[test]
fn objects_and_prototypes() {
    assert_eq!(
        eval("const o = {a: 1, 'b c': 2, [1 + 1]: 3}; o.a + o['b c'] + o[2]"),
        "6"
    );
    assert_eq!(
        eval(
            "const o = {get x() { return this.y * 2 }, set x(v) { this.y = v }, y: 1}; o.x = 5; o.x"
        ),
        "10"
    );
    assert_eq!(
        eval("const a = 1, b = 2; JSON.stringify({a, b, ...{c: 3}, m() { return 1 }})"),
        r#"{"a":1,"b":2,"c":3}"#
    );
    assert_eq!(
        eval(
            "function P(n) { this.n = n; } P.prototype.get = function () { return this.n; }; new P(7).get()"
        ),
        "7"
    );
    assert_eq!(
        eval(
            "const p = {hi() { return 'p' }}; const c = Object.create(p); c.hi() + Object.getPrototypeOf(c).hi()"
        ),
        "pp"
    );
    assert_eq!(eval("const o = {a: 1}; delete o.a; 'a' in o"), "false");
    assert_eq!(
        eval("const o = {a: 1}; Object.freeze(o); o.a = 2; o.b = 3; JSON.stringify(o)"),
        r#"{"a":1}"#
    );
    assert_eq!(
        eval(
            "const o = {}; Object.defineProperty(o, 'x', {value: 1}); o.x = 2; Object.keys(o).length + ':' + o.x"
        ),
        "0:1"
    );
    assert_eq!(
        eval("Object.entries({a: 1, b: 2}).map(([k, v]) => k + v).join()"),
        "a1,b2"
    );
    assert_eq!(
        eval("({}).toString() + [].toString() + Object.prototype.toString.call([])"),
        "[object Object][object Array]"
    );
    assert_eq!(
        eval("const o = {a: {b: null}}; o.a?.b?.c + '' + o.x?.y.z + o.a.f?.()"),
        "undefinedundefinedundefined"
    );
    assert_eq!(
        eval(
            "const f = function () { return this.v; }; f.call({v: 1}) + f.apply({v: 2}, []) + f.bind({v: 3})()"
        ),
        "6"
    );
    assert_eq!(
        eval("function f(a, b) { return arguments.length + a + (b ?? 0); } f(1) + f(1, 2, 3)"),
        "8"
    );
}

#[test]
fn classes() {
    let source = "
        class Animal {
            static count = 0;
            #secret = 'x';
            legs = 4;
            constructor(name) { this.name = name; Animal.count++; }
            speak() { return this.name + ' fait du bruit'; }
            get upper() { return this.name.toUpperCase(); }
            static create(n) { return new this(n); }
            reveal() { return this.#secret; }
        }
        class Dog extends Animal {
            sound = 'ouaf';
            constructor(name) { super(name); this.kind = 'chien'; }
            speak() { return super.speak() + ' : ' + this.sound; }
        }
        const d = Dog.create('Rex');
        [d.speak(), d.upper, d.legs, d.kind, Animal.count, d instanceof Animal, d.reveal(), typeof Dog,
         Object.keys(d).join('+')].join('|')";
    assert_eq!(
        eval(source),
        "Rex fait du bruit : ouaf|REX|4|chien|1|true|x|function|#secret+legs+name+sound+kind"
    );
    assert_eq!(
        eval("class A {} class B extends A {} new B() instanceof A"),
        "true"
    );
    assert!(
        eval("class A { constructor() { this.x = 1; } } class B extends A { constructor() { this.y = 2; } } new B()")
            .contains("ReferenceError")
    );
    assert_eq!(
        eval("const C = class { m() { return 1; } }; new C().m()"),
        "1"
    );
    assert_eq!(
        eval("class A { static make() { return 'a'; } } class B extends A {} B.make()"),
        "a"
    );
}

#[test]
fn destructuring_spread_and_defaults() {
    assert_eq!(
        eval("const [a, , b = 5, ...rest] = [1, 2, undefined, 4, 5]; a + b + rest.length"),
        "8"
    );
    assert_eq!(
        eval(
            "const {x, y: {z = 3} = {}, ...others} = {x: 1, p: 2, q: 3}; x + z + Object.keys(others).join('')"
        ),
        "4pq"
    );
    assert_eq!(eval("let a = 1, b = 2; [a, b] = [b, a]; '' + a + b"), "21");
    assert_eq!(
        eval(
            "function f({a, b = 2} = {}, [c] = [3], ...r) { return a + b + c + r.length; } f({a: 1}, [10], 0, 0)"
        ),
        "15"
    );
    assert_eq!(
        eval("const f = (x = 1, y = x + 1) => x + y; f() + f(5)"),
        "14"
    );
    assert_eq!(
        eval("Math.max(...[1, 5, 3], 2) + [...'ab', ...[1]].length"),
        "8"
    );
    assert_eq!(
        eval("const o = {}; ({a: o.p, b: o['q']} = {a: 1, b: 2}); o.p + o.q"),
        "3"
    );
    assert_eq!(
        eval("const m = new Map([[1, 'a']]); const [[k, v]] = m; k + v"),
        "1a"
    );
    assert!(eval("const {a} = null").contains("TypeError"));
}

#[test]
fn arrays() {
    assert_eq!(
        eval("[3, 1, 2].sort().concat([9]).reverse().join('')"),
        "9321"
    );
    assert_eq!(
        eval("[10, 9, 1].sort() + '|' + [10, 9, 1].sort((a, b) => a - b)"),
        "1,10,9|1,9,10"
    );
    assert_eq!(
        eval("[1, 2, 3, 4].filter(x => x % 2).map(x => x * 10).reduce((a, b) => a + b, 1)"),
        "41"
    );
    assert_eq!(
        eval(
            "const a = [1, 2, 3, 4, 5]; a.splice(1, 2, 'x') + '|' + a + '|' + a.slice(-2) + '|' + a.indexOf(4)"
        ),
        "2,3|1,x,4,5|4,5|2"
    );
    assert_eq!(
        eval(
            "const a = []; a[3] = 1; a.length + ':' + a.push(2) + ':' + a.pop() + ':' + a.shift()"
        ),
        "4:5:2:undefined"
    );
    assert_eq!(
        eval("[1, [2, [3, [4]]]].flat(2).length + [1, 2].flatMap(x => [x, x]).join('')"),
        "41122"
    );
    assert_eq!(
        eval("[1, 2, 3].find(x => x > 1) + [1, 2, 3].findIndex(x => x > 5) + [1, 2, 3].at(-1)"),
        "4"
    );
    assert_eq!(
        eval(
            "[1, 2, 3].includes(2) && [NaN].includes(NaN) && [NaN].indexOf(NaN) === -1 && [1].some(x => x) && [].every(x => x)"
        ),
        "true"
    );
    assert_eq!(
        eval(
            "Array.from({length: 3}, (_, i) => i * 2) + '|' + Array.isArray([]) + '|' + Array.of(1, 2) + '|' + new Array(3).length"
        ),
        "0,2,4|true|1,2|3"
    );
    assert_eq!(
        eval("const a = [1, 2, 3]; a.length = 1; a.fill(7); a + ''"),
        "7"
    );
    // Tri stable, et comparateur incohérent sans panique.
    assert_eq!(
        eval(
            "[{k: 1, v: 'a'}, {k: 0, v: 'b'}, {k: 1, v: 'c'}].sort((x, y) => x.k - y.k).map(o => o.v).join('')"
        ),
        "bac"
    );
    assert_eq!(eval("[5, 3, 8, 1, 9, 2].sort(() => 1).length"), "6");
    assert!(eval("[].reduce((a, b) => a + b)").contains("TypeError"));
    assert!(eval("new Array(-1)").contains("RangeError"));
    assert!(eval("const a = []; a[1e9] = 1").contains("RangeError"));
}

#[test]
fn strings_numbers_math_json() {
    assert_eq!(
        eval(
            "'Oxyde'.toUpperCase() + 'abc'.slice(1) + 'abc'.substring(2, 0) + 'a-b-c'.split('-').length + ' x '.trim()"
        ),
        "OXYDEbcab3x"
    );
    assert_eq!(
        eval(
            "'héllo'.length + ':' + '😀'.length + ':' + 'héllo'.indexOf('l') + ':' + 'héllo'[1] + ':' + 'abc'.at(-1)"
        ),
        "5:2:2:é:c"
    );
    assert_eq!(
        eval(
            "'a.b.c'.replace('.', '-') + 'a.b.c'.replaceAll('.', '-') + 'x'.padStart(3, 'ab') + 'ab'.repeat(2) + 'abc'.includes('bc')"
        ),
        "a-b.ca-b-cabxababtrue"
    );
    assert_eq!(
        eval("'abc'.charCodeAt(1) + String.fromCharCode(72, 105) + 'b'.localeCompare('a')"),
        "98Hi1"
    );
    assert_eq!(
        eval(
            "(255).toString(16) + (0.5).toString(2) + (1.005).toFixed(2) + (1234.5678).toFixed(1) + parseInt('42px') + parseFloat('3.14abc') + parseInt('ff', 16)"
        ),
        "ff0.11.001234.6423.14255"
    );
    assert_eq!(
        eval(
            "Number('12') + Number('') + Number(true) + Number.isInteger(5) + Number.MAX_SAFE_INTEGER"
        ),
        "9007199254741004"
    );
    assert_eq!(
        eval("isNaN(Number('x')) && Number.isNaN(NaN) && !Number.isNaN('x') && isFinite('1')"),
        "true"
    );
    assert_eq!(
        eval(
            "Math.round(2.5) + ':' + Math.round(-2.5) + ':' + Math.round(-0.4) + ':' + Math.hypot(3, 4) + ':' + Math.PI.toFixed(4) + ':' + Math.max(1, 3, 2) + ':' + Math.min()"
        ),
        "3:-2:0:5:3.1416:3:Infinity"
    );
    assert_eq!(
        eval("const r = Math.random(); r >= 0 && r < 1 && Math.random() !== r"),
        "true"
    );
    assert_eq!(
        eval(
            r#"JSON.stringify({a: [1, "x", null, undefined, () => 1], b: {c: true}, d: undefined, e: NaN})"#
        ),
        r#"{"a":[1,"x",null,null,null],"b":{"c":true},"e":null}"#
    );
    assert_eq!(
        eval(
            r#"const o = JSON.parse('{"a": [1, 2.5e1, "\\u00e9\\n"], "b": {"c": null}, "__proto__": 1}');
               o.a[1] + o.a[2].length + (o.b.c === null) + Object.keys(o).length"#
        ),
        "31"
    );
    assert_eq!(
        eval("JSON.stringify([1, {a: 2}], null, 2)"),
        "[\n  1,\n  {\n    \"a\": 2\n  }\n]"
    );
    assert!(eval("const o = {}; o.self = o; JSON.stringify(o)").contains("TypeError"));
    for bad in [
        "{", "[1,]", "01", "'a'", "{\"a\":}", "nul", "1 2", "\"\\x\"", "",
    ] {
        assert!(
            eval(&format!("JSON.parse({bad:?})")).contains("SyntaxError"),
            "{bad:?}"
        );
    }
    assert_eq!(
        eval("encodeURIComponent('a b&é') + decodeURIComponent('%C3%A9%20')"),
        "a%20b%26%C3%A9é "
    );
}

#[test]
fn maps_and_sets() {
    assert_eq!(
        eval(
            "const m = new Map(); m.set('a', 1).set(NaN, 2).set(0, 3); m.get(NaN) + m.get(-0) + m.size + m.has('a') + m.delete('a') + m.size"
        ),
        "12"
    );
    assert_eq!(
        eval(
            "const s = new Set([1, 2, 2, 3]); s.add(1); let t = 0; for (const v of s) t += v; s.size + ':' + t + ':' + [...s].join('')"
        ),
        "3:6:123"
    );
    assert_eq!(
        eval(
            "const m = new Map([['x', 1], ['y', 2]]); let out = ''; m.forEach((v, k) => out += k + v); for (const [k, v] of m) out += k + v; out + [...m.keys()].join('')"
        ),
        "x1y2x1y2xy"
    );
    assert_eq!(
        eval("const k = {}; const w = new WeakMap(); w.set(k, 1); w.get(k) + (w.has({}) ? 1 : 0)"),
        "1"
    );
}

#[test]
fn symbols() {
    assert_eq!(eval("typeof Symbol()"), "symbol");
    assert_eq!(eval("Symbol('a') === Symbol('a')"), "false");
    assert_eq!(eval("const s = Symbol('a'); s === s"), "true");
    assert_eq!(eval("Symbol.for('k') === Symbol.for('k')"), "true");
    assert_eq!(eval("Symbol.keyFor(Symbol.for('k'))"), "k");
    assert_eq!(eval("Symbol('desc').toString()"), "Symbol(desc)");
    assert_eq!(eval("Symbol('desc').description"), "desc");
    assert_eq!(eval("Symbol().description"), "undefined");
    // Les propriétés à clé symbole sont distinctes des clés chaînes et
    // invisibles des énumérations ordinaires.
    assert_eq!(
        eval(
            "const s = Symbol('k'); const o = { [s]: 1, k: 2 }; o[s] = o[s] + 10;
             [o[s], o.k, Object.keys(o).length, JSON.stringify(o), s in o,
              Object.getOwnPropertySymbols(o).length, Object.hasOwn(o, s), delete o[s], s in o].join()"
        ),
        "11,2,1,{\"k\":2},true,1,true,true,false"
    );
    assert_eq!(
        eval(
            "const s = Symbol(); const o = {}; for (const k in { [s]: 1, a: 2 }) o[k] = 1; Object.keys(o).join()"
        ),
        "a"
    );
    assert_eq!(
        eval("const s = Symbol('x'); const a = { [s]: 1 }; const b = { ...a }; b[s]"),
        "1"
    );
    assert_eq!(
        eval("class A { get [Symbol.toStringTag]() { return 'Truc'; } } String(new A())"),
        "[object Truc]"
    );
    // Un symbole ne se convertit ni en chaîne ni en nombre implicitement.
    assert!(eval("'' + Symbol()").starts_with("ERREUR TypeError"));
    assert!(eval("+Symbol()").starts_with("ERREUR TypeError"));
    assert!(eval("new Symbol()").starts_with("ERREUR TypeError"));
    assert_eq!(
        eval("const m = new Map(); const s = Symbol(); m.set(s, 1); m.get(s)"),
        "1"
    );
}

#[test]
fn iteration_protocol() {
    // Itérable défini par un script.
    assert_eq!(
        eval(
            "const compte = { de: 1, a: 4, [Symbol.iterator]() {
                 let n = this.de; const fin = this.a;
                 return { next: () => n <= fin ? { value: n++, done: false } : { value: undefined, done: true } };
             } };
             let somme = 0; for (const n of compte) somme += n;
             const [premier, ...reste] = compte;
             somme + ':' + [...compte].join('') + ':' + premier + ':' + reste.length + ':' + Array.from(compte).length"
        ),
        "10:1234:1:3:4"
    );
    // Itérateurs intégrés : objets munis de `next`, eux-mêmes itérables.
    assert_eq!(
        eval(
            "const it = [10, 20][Symbol.iterator](); const a = it.next(); const b = it.next(); const c = it.next();
             [a.value, a.done, b.value, c.value, c.done, it[Symbol.iterator]() === it].join()"
        ),
        "10,false,20,,true,true"
    );
    assert_eq!(eval("[...'aé😀'].length"), "3");
    assert_eq!(eval("[...[1, 2].entries()].join('|')"), "0,1|1,2");
    assert_eq!(eval("[...[5, 6].keys()].join()"), "0,1");
    assert_eq!(
        eval(
            "const m = new Map([['a', 1]]); [...m.values()].join() + [...m.entries()][0].join() + [...new Set([7]).values()].join()"
        ),
        "1a,17"
    );
    assert_eq!(
        eval("typeof [][Symbol.iterator] + typeof ''[Symbol.iterator]"),
        "functionfunction"
    );
    assert!(eval("for (const x of { next() {} }) {}").starts_with("ERREUR TypeError"));
    assert!(eval("const [a] = 5;").starts_with("ERREUR TypeError"));
    assert!(eval("[...{ [Symbol.iterator]() { return 1; } }]").starts_with("ERREUR TypeError"));
}

#[test]
fn generators() {
    assert_eq!(
        eval(
            "function* compte(n) { for (let i = 0; i < n; i++) yield i; return 'fin'; }
             const g = compte(2); const a = g.next(), b = g.next(), c = g.next(), d = g.next();
             [a.value, a.done, b.value, c.value, c.done, d.value, d.done].join()"
        ),
        "0,false,1,fin,true,,true"
    );
    assert_eq!(
        eval("function* g() { yield 1; yield 2; } [...g()].join()"),
        "1,2"
    );
    assert_eq!(
        eval(
            "function* g() { yield 1; yield 2; yield 3; } let s = 0; for (const v of g()) s += v; s"
        ),
        "6"
    );
    // Les valeurs envoyées par `next(v)` deviennent le résultat de `yield`.
    assert_eq!(
        eval(
            "function* echo() { let recu = yield 'pret'; while (true) recu = yield recu * 2; }
             const g = echo(); g.next(); [g.next(5).value, g.next(7).value].join()"
        ),
        "10,14"
    );
    // `throw()` lève au point de suspension ; le générateur peut rattraper.
    assert_eq!(
        eval(
            "function* g() { try { yield 1; } catch (e) { yield 'rattrape ' + e; } }
             const it = g(); it.next(); it.throw('boum').value"
        ),
        "rattrape boum"
    );
    assert!(
        eval("function* g() { yield 1; } const it = g(); it.next(); it.throw(new Error('x'))")
            .contains("Error: x")
    );
    assert_eq!(
        eval(
            "function* g() { yield 1; yield 2; } const it = g(); it.next(); const r = it.return(9); [r.value, r.done, it.next().done].join()"
        ),
        "9,true,true"
    );
    // `yield*` délègue à un autre itérable.
    assert_eq!(
        eval(
            "function* a() { yield 1; yield 2; } function* b() { yield 0; yield* a(); yield* [3, 4]; } [...b()].join('')"
        ),
        "01234"
    );
    // Variables capturées par une fermeture : elles survivent aux suspensions.
    assert_eq!(
        eval(
            "function* g() { let n = 0; const inc = () => ++n; yield inc; yield n; inc(); yield n; }
             const it = g(); const inc = it.next().value; inc(); inc(); const a = it.next().value; const b = it.next().value; a + ':' + b"
        ),
        "2:3"
    );
    // Méthodes génératrices, `Symbol.iterator` écrit comme un générateur.
    assert_eq!(
        eval(
            "class Liste { constructor(...v) { this.v = v; } *[Symbol.iterator]() { yield* this.v; } }
             const o = { *pairs() { yield 2; yield 4; } };
             [...new Liste(1, 2, 3)].join('') + [...o.pairs()].join('')"
        ),
        "12324"
    );
    assert_eq!(
        eval(
            "function* g() { const [a, b] = yield; return a + b; } const it = g(); it.next(); it.next([1, 2]).value"
        ),
        "3"
    );
    // Un générateur infini ne coûte rien tant qu'on ne le consomme pas.
    assert_eq!(
        eval(
            "function* n() { let i = 0; while (true) yield i++; } const [a, b, c] = n(); a + b + c"
        ),
        "3"
    );
    assert_eq!(
        eval("function* g() {} typeof g().next + String(g())"),
        "function[object Generator]"
    );
    assert!(
        eval("function* g() { it.next(); yield 1; } const it = g(); it.next()")
            .contains("TypeError")
    );
    assert!(eval("function* g() {} new g()").contains("TypeError"));
    // Hors d'un générateur, `yield` reste un identifiant.
    assert_eq!(eval("var yield = 3; yield + 1"), "4");
    assert!(eval("function f() { yield 1; }").contains("SyntaxError"));
}

/// Évalue `source`, laisse s'exécuter les microtâches, puis évalue `probe`.
fn eval_then(source: &str, probe: &str) -> String {
    let mut vm = Vm::new();
    if let Err(error) = vm.eval(source) {
        return format!("ERREUR {error}");
    }
    match vm.eval(probe) {
        Ok(value) => vm.display(&value),
        Err(error) => format!("ERREUR {error}"),
    }
}

#[test]
fn promises() {
    // Les réactions s'exécutent après le script, dans l'ordre d'inscription.
    assert_eq!(
        eval_then(
            "var out = []; Promise.resolve(1).then(v => out.push('a' + v)); out.push('sync');
             new Promise(r => { out.push('exec'); r(2); }).then(v => out.push('b' + v));",
            "out.join()"
        ),
        "sync,exec,a1,b2"
    );
    // Chaînage, valeurs de retour, promesses rendues par un gestionnaire.
    assert_eq!(
        eval_then(
            "var out; Promise.resolve(1).then(v => v + 1).then(v => Promise.resolve(v * 10)).then(v => { out = v; });",
            "out"
        ),
        "20"
    );
    // Rejets : `catch`, exceptions levées dans un gestionnaire, passage au travers.
    assert_eq!(
        eval_then(
            "var out = []; Promise.reject('r').then(() => out.push('non')).catch(e => out.push('c:' + e));
             Promise.resolve().then(() => { throw new Error('x'); }).catch(e => out.push(e.message));
             new Promise(() => { throw 'exec'; }).then(null, e => out.push(e));",
            "out.join()"
        ),
        "exec,c:r,x"
    );
    assert_eq!(
        eval_then(
            "var out = []; Promise.resolve(1).finally(() => out.push('f')).then(v => out.push(v));
             Promise.reject(2).finally(() => out.push('g')).catch(e => out.push(e));",
            "out.join()"
        ),
        "f,g,1,2"
    );
    // Seul le premier appel à resolve / reject compte.
    assert_eq!(
        eval_then(
            "var out; new Promise((res, rej) => { res(1); rej(2); res(3); }).then(v => { out = v; }, () => { out = 'non'; });",
            "out"
        ),
        "1"
    );
    // Objets « thenable ».
    assert_eq!(
        eval_then(
            "var out; Promise.resolve({ then(ok) { ok(42); } }).then(v => { out = v; });",
            "out"
        ),
        "42"
    );
    assert_eq!(
        eval_then(
            "var out; const p = new Promise(r => { globalThis.fin = r; }); p.then(v => { out = v; }); fin(p);",
            "typeof out"
        ),
        "undefined"
    );
    // Combinateurs.
    assert_eq!(
        eval_then(
            "var out = {}; const p = (v, ok = true) => new Promise((res, rej) => ok ? res(v) : rej(v));
             Promise.all([p(1), 2, p(3)]).then(v => { out.all = v.join(''); });
             Promise.all([p(1), p('e', false)]).catch(e => { out.allKo = e; });
             Promise.all([]).then(v => { out.vide = v.length; });
             Promise.allSettled([p(1), p('e', false)]).then(v => { out.settled = v.map(r => r.status + ':' + (r.value ?? r.reason)).join(); });
             Promise.race([new Promise(() => {}), p('vite')]).then(v => { out.race = v; });
             Promise.any([p('a', false), p('b')]).then(v => { out.any = v; });
             Promise.any([p('a', false)]).catch(e => { out.anyKo = e.name + e.errors.join(); });",
            "[out.all, out.allKo, out.vide, out.settled, out.race, out.any, out.anyKo].join('|')"
        ),
        "123|e|0|fulfilled:1,rejected:e|vite|b|AggregateErrora"
    );
    assert_eq!(
        eval("String(Promise.resolve()) + (Promise.resolve(1) instanceof Promise)"),
        "[object Promise]true"
    );
    assert!(eval("Promise()").contains("TypeError"));
    assert!(eval("new Promise(1)").contains("TypeError"));

    // Un rejet sans gestionnaire est signalé dans la console, un rejet géré non.
    let mut vm = Vm::new();
    vm.eval("Promise.reject(new Error('perdu')); Promise.reject('géré').catch(() => {});")
        .expect("script valide");
    let messages = vm.take_console();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].level, "error");
    assert!(
        messages[0].text.contains("Error: perdu"),
        "{}",
        messages[0].text
    );
}

#[test]
fn async_functions() {
    assert_eq!(
        eval_then(
            "var out = []; async function f(x) { out.push('debut'); const v = await x; out.push('apres ' + v); return v + 1; }
             f(1).then(v => out.push('fin ' + v)); out.push('sync');",
            "out.join()"
        ),
        "debut,sync,apres 1,fin 2"
    );
    assert_eq!(
        eval("async function f() {} (f() instanceof Promise) + typeof f"),
        "truefunction"
    );
    // Exceptions : `await` d'un rejet lève, `try`/`catch` le rattrape.
    assert_eq!(
        eval_then(
            "var out = []; async function f() { try { await Promise.reject('r'); } catch (e) { out.push('c:' + e); } throw new Error('fin'); }
             f().catch(e => out.push(e.message));",
            "out.join()"
        ),
        "c:r,fin"
    );
    // Boucles, appels imbriqués, fonctions fléchées et méthodes async.
    assert_eq!(
        eval_then(
            "var out; const double = async x => (await x) * 2;
             const o = { async somme(...v) { let s = 0; for (const x of v) s += await double(x); return s; } };
             class C { async m() { return await o.somme(1, 2, 3); } static async s() { return 'st'; } }
             (async () => { out = (await new C().m()) + (await C.s()); })();",
            "out"
        ),
        "12st"
    );
    // `await` d'une valeur ordinaire, d'un thenable ; variables capturées.
    assert_eq!(
        eval_then(
            "var out; async function f() { let n = 0; const inc = () => ++n; await null; inc(); await { then(ok) { ok(inc()); } }; return n; }
             f().then(v => { out = v; });",
            "out"
        ),
        "2"
    );
    // Entrelacement de deux fonctions async.
    assert_eq!(
        eval_then(
            "var out = []; async function f(nom) { for (let i = 0; i < 2; i++) { out.push(nom + i); await 0; } }
             f('a'); f('b');",
            "out.join('')"
        ),
        "a0b0a1b1"
    );
    assert_eq!(
        eval_then(
            "var out; Promise.all([1, 2, 3].map(async x => x * 2)).then(v => { out = v.join(); });",
            "out"
        ),
        "2,4,6"
    );
    // Hors d'une fonction async, `await` reste un identifiant.
    assert_eq!(eval("var await = 2; await * 2"), "4");
    assert!(eval("function f() { await 1; }").contains("SyntaxError"));
    assert_eq!(eval("const async = 5; async + 1"), "6");
    assert_eq!(eval("const async = x => x * 2; async(4)"), "8");

    // Le ramasse-miettes peut passer pendant une longue chaîne de microtâches.
    let mut vm = Vm::new();
    vm.eval("var total = 0; (async () => { for (let i = 0; i < 20000; i++) { total += (await { v: [i] }).v[0]; } })();")
        .expect("script valide");
    let total = vm.eval("total").expect("lecture");
    assert_eq!(vm.display(&total), "199990000");
    assert!(vm.collections() > 0, "le ramasse-miettes est passé");
    assert!(
        vm.live_objects() < 5_000,
        "{} objets vivants",
        vm.live_objects()
    );
}

#[test]
fn regular_expressions() {
    assert_eq!(
        eval("/a+/.test('caab') + ':' + /^a/.test('ba')"),
        "true:false"
    );
    assert_eq!(
        eval(
            r"const m = /(\d+)-(\d+)/.exec('tel 12-345 fin'); [m[0], m[1], m[2], m.index, m.input, m.length].join()"
        ),
        "12-345,12,345,4,tel 12-345 fin,3"
    );
    assert_eq!(eval("/x/.exec('abc')"), "null");
    assert_eq!(
        eval(
            r"const m = /(?<an>\d{4})-(?<mois>\d\d)/.exec('le 2026-11'); m.groups.an + '/' + m.groups.mois"
        ),
        "2026/11"
    );
    // Drapeau g : `lastIndex` avance, puis revient à zéro.
    assert_eq!(
        eval(
            "const re = /a./g; const s = 'a1 a2 a3'; const vus = []; let m;
             while ((m = re.exec(s))) vus.push(m[0] + '@' + m.index + '>' + re.lastIndex);
             vus.join() + ' ' + re.lastIndex"
        ),
        "a1@0>2,a2@3>5,a3@6>8 0"
    );
    assert_eq!(
        eval(
            "const re = /b/y; re.lastIndex = 1; re.test('abc') + ':' + re.lastIndex + ':' + re.test('abc')"
        ),
        "true:2:false"
    );
    // Propriétés et conversions.
    assert_eq!(
        eval(
            r"const re = /a\/b/gi; [re.source, re.flags, re.global, re.ignoreCase, re.multiline, String(re), typeof re].join(' ')"
        ),
        r"a\/b gi true true false /a\/b/gi object"
    );
    assert_eq!(
        eval("String(new RegExp('a/b')) + new RegExp('') + new RegExp(/x/g, 'i')"),
        r"/a\/b//(?:)//x/i"
    );
    assert_eq!(eval("new RegExp('^\\\\d+$').test('123')"), "true");
    assert_eq!(eval("/a/ instanceof RegExp"), "true");
    assert_eq!(
        eval("Object.prototype.toString.call(/a/)"),
        "[object RegExp]"
    );
    // Méthodes de String.
    assert_eq!(eval("'a1b22c333'.match(/\\d+/g).join()"), "1,22,333");
    assert_eq!(eval("'abc'.match(/x/g)"), "null");
    assert_eq!(
        eval("const m = 'abc'.match(/(b)(c)/); m.index + m[1] + m[2]"),
        "1bc"
    );
    assert_eq!(eval("'abc'.match('b').index"), "1");
    assert_eq!(
        eval("[...'a1b2'.matchAll(/[a-z](\\d)/g)].map(m => m[1] + '@' + m.index).join()"),
        "1@0,2@2"
    );
    assert!(eval("'a'.matchAll(/a/)").contains("TypeError"));
    assert_eq!(
        eval("'abc'.search(/c/) + ':' + 'abc'.search(/x/) + ':' + 'héé'.search(/é/)"),
        "2:-1:1"
    );
    assert_eq!(
        eval("'2026-11-12'.replace(/(\\d+)-(\\d+)-(\\d+)/, '$3/$2/$1')"),
        "12/11/2026"
    );
    assert_eq!(eval("'a b  c'.replace(/\\s+/g, '_')"), "a_b_c");
    assert_eq!(
        eval("'aaa'.replace(/a/, 'b') + 'aaa'.replaceAll(/a/g, 'b')"),
        "baabbb"
    );
    assert!(eval("'a'.replaceAll(/a/, 'b')").contains("TypeError"));
    assert_eq!(
        eval("'jean dupont'.replace(/(?<p>\\w+) (?<n>\\w+)/, '$<n>, $<p> [$&] [$`] [$\\'] $$ $9')"),
        "dupont, jean [jean dupont] [] [] $ $9"
    );
    assert_eq!(
        eval("'a-b-c'.replace(/-(\\w)/g, (tout, lettre, index) => lettre.toUpperCase() + index)"),
        "aB1C3"
    );
    assert_eq!(eval("'abc'.replace(/x*/g, '-')"), "-a-b-c-");
    assert_eq!(eval("'xay'.replace('a', \"[$`|$&|$']\")"), "x[x|a|y]y");
    assert_eq!(eval("'a, b ;c'.split(/\\s*[,;]\\s*/).join('|')"), "a|b|c");
    assert_eq!(
        eval("'a1b2c'.split(/(\\d)/).join('|') + ' ' + 'abc'.split(/(?:)/).join('|')"),
        "a|1|b|2|c a|b|c"
    );
    assert_eq!(
        eval("'a,b,c'.split(/,/, 2).join('|') + ''.split(/x/).length + ''.split(/(?:)/).length"),
        "a|b10"
    );
    // Unicode : index en unités UTF-16, classes de propriétés.
    assert_eq!(eval("const m = /b/.exec('😀b'); m.index"), "2");
    assert_eq!(eval("'Été à Paris'.match(/\\p{Lu}/gu).join('')"), "ÉP");
    assert_eq!(eval("'aXbxc'.split(/x/i).join('')"), "abc");
    // Erreurs de syntaxe et motifs explosifs.
    assert!(eval("new RegExp('(')").contains("SyntaxError"));
    assert!(eval("new RegExp('a', 'gg')").contains("SyntaxError"));
    assert!(
        eval("/(a+)+$/.test('aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab')")
            .contains("RangeError")
    );
    // Division et littéral ne se confondent pas.
    assert_eq!(
        eval("const a = 8, g = 2; a / 2 / g + (/2/.test('12') ? 1 : 0)"),
        "3"
    );
}

#[test]
fn garbage_collection_reclaims_memory() {
    let mut vm = Vm::new();
    let before = vm.live_objects();
    vm.eval("let keep = []; for (let i = 0; i < 20000; i++) { const tmp = {i, a: [i]}; if (i % 5000 === 0) keep.push(tmp); }")
        .expect("script valide");
    assert!(
        vm.collections() > 0,
        "le ramasse-miettes doit avoir tourné pendant la boucle"
    );
    vm.collect_garbage();
    let after = vm.live_objects();
    assert!(
        after < before + 200,
        "les objets temporaires sont libérés : {before} → {after}"
    );
    // Ce qui est encore atteignable a survécu et reste utilisable.
    let kept = vm
        .eval("keep.map(o => o.a[0]).join()")
        .expect("script valide");
    assert_eq!(vm.display(&kept), "0,5000,10000,15000");
    // Les fermetures gardent leurs variables capturées en vie.
    vm.eval("function counter() { let n = {v: 0}; return () => ++n.v; } var c = counter();")
        .expect("script valide");
    vm.collect_garbage();
    let value = vm.eval("c(); c()").expect("script valide");
    assert_eq!(value, Value::Number(2.0));
}

#[test]
fn runaway_scripts_are_interrupted() {
    let mut vm = Vm::new();
    vm.set_budget(Some(100_000));
    let result = vm.eval("try { while (true) {} } catch (e) { 'rattrapé' }");
    assert!(
        matches!(result, Err(EvalError::Thrown { .. })),
        "l'interruption ne se rattrape pas"
    );
    assert!(vm.was_interrupted());
    vm.set_budget(None);
    assert_eq!(
        vm.eval("1 + 1"),
        Ok(Value::Number(2.0)),
        "la machine reste utilisable"
    );
}

#[test]
fn deep_recursion_is_a_catchable_error() {
    let outcome = with_large_stack(|| {
        let mut vm = Vm::new();
        let a = vm.eval(
            "function f(n) { return f(n + 1); } try { f(0) } catch (e) { e instanceof RangeError }",
        );
        let b = vm.eval("const big = [1]; function g(n) { return big.map(() => g(n + 1)); } try { g(0) } catch (e) { e.name }");
        let c = vm.eval("1 + 1");
        (a, b.map(|v| vm.display(&v)), c)
    });
    assert_eq!(outcome.0, Ok(Value::Bool(true)));
    assert_eq!(outcome.1.as_deref(), Ok("RangeError"));
    assert_eq!(outcome.2, Ok(Value::Number(2.0)));
}

#[test]
fn console_and_unsupported_features() {
    let mut vm = Vm::new();
    vm.eval("console.log('a', 1, {b: [2]}); console.error(new Error('x'))")
        .expect("script valide");
    let messages = vm.take_console();
    assert_eq!(messages.len(), 2);
    assert_eq!(
        (messages[0].level, messages[0].text.as_str()),
        ("log", "a 1 { b: [2] }")
    );
    assert_eq!(
        (messages[1].level, messages[1].text.as_str()),
        ("error", "Error: x")
    );
    assert!(vm.take_console().is_empty());

    assert!(matches!(
        vm.eval("async function* f() {}"),
        Err(EvalError::Syntax(_))
    ));
    assert!(eval("/(/.test('aa')").contains("SyntaxError"));
    assert!(eval("new Function('return 1')").contains("EvalError"));
    assert!(eval("return 1").contains("SyntaxError"));
}

#[test]
fn never_panics_on_arbitrary_programs() {
    // Programmes valides ou non, tous sémantiquement douteux.
    let programs = [
        "null.x.y",
        "(void 0)()",
        "new 1",
        "new (() => 1)",
        "({}) instanceof 1",
        "1 in 2",
        "x = x",
        "[...1]",
        "for (const a of 1) {}",
        "class A extends 1 {}",
        "class A extends null { constructor() { super() } } new A",
        "({}).x.y.z = 1",
        "Object.defineProperty(1, 'a', {})",
        "Object.setPrototypeOf(Object.prototype, {})",
        "const o = {}; Object.setPrototypeOf(o, o)",
        "'a'.repeat(1e10)",
        "'a'.padEnd(1e12)",
        "new Array(1e12)",
        "[].concat(...Array(10).fill([1]))",
        "JSON.parse('['.repeat(10000))",
        "let a = []; a.length = -1",
        "Function.prototype.call.call(1)",
        "[1, 2].sort(1)",
        "({}).toString.call(null)",
        "Math.max({valueOf() { throw 1 }})",
        "String(Object.create(null))",
        "`${{toString() { return {} }}}`",
        "var o = {get a() { return o.a }}; o.a",
        "throw undefined",
        "delete null.x",
        "super.x",
        "break",
        "a: a: 1",
        "let let = 1",
        "0++",
    ];
    with_large_stack(|| {
        for program in programs {
            let mut vm = Vm::new();
            vm.set_budget(Some(2_000_000));
            let _ = vm.eval(program);
        }
    });
}
