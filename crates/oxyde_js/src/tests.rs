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
        "Rex fait du bruit : ouaf|REX|4|chien|1|true|x|function|legs+name+sound+kind"
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
fn eval_reflect_and_function_names() {
    // `eval` évalue dans la portée globale.
    assert_eq!(
        eval("eval('1 + 2') + eval(4) + eval('var créé = 10; créé')"),
        "17"
    );
    assert_eq!(eval("eval('var x = 5'); x"), "5");
    assert!(eval("eval('var = 1')").contains("SyntaxError"));
    assert_eq!(
        eval("try { eval('('); } catch (e) { e instanceof SyntaxError }"),
        "true"
    );
    assert_eq!(
        eval("const f = new Function('return 40 + 2'); f() + Function('x', 'return x * 2')(4)"),
        "50"
    );
    assert_eq!(eval("function f() { return eval('1 + 1'); } f()"), "2");

    // Noms inférés des fonctions et classes anonymes.
    assert_eq!(
        eval(
            "const fleche = () => {}; let fn = function () {}; var cls = class {}; let autre; autre = function* () {};
             const o = { methode() {}, prop: () => {}, [Symbol.iterator]: function () {} };
             const { defaut = () => {} } = {}; const nommee = function interne() {};
             [fleche.name, fn.name, cls.name, autre.name, o.methode.name, o.prop.name, defaut.name, nommee.name, (() => {}).name].join()"
        ),
        "fleche,fn,cls,autre,methode,prop,defaut,interne,"
    );
    assert_eq!(
        eval("const C = class extends (function () {}) {}; C.name + Object.getPrototypeOf(C).name"),
        "C"
    );
    assert_eq!(
        eval(
            "const d = Object.getOwnPropertyDescriptor(function f(a, b) {}, 'length'); [d.value, d.writable, d.enumerable, d.configurable].join()"
        ),
        "2,false,false,true"
    );

    // Reflect.
    assert_eq!(
        eval(
            "const o = { a: 1 }; const s = Symbol('s');
             [Reflect.has(o, 'a'), Reflect.get(o, 'a'), Reflect.set(o, 'b', 2), o.b, Reflect.defineProperty(o, s, { value: 3 }),
              Reflect.ownKeys(o).length, Reflect.deleteProperty(o, 'a'), 'a' in o, Reflect.getPrototypeOf(o) === Object.prototype,
              Reflect.isExtensible(o), Reflect.preventExtensions(o), Reflect.isExtensible(o), Reflect.setPrototypeOf(o, null)].join()"
        ),
        "true,1,true,2,true,3,true,false,true,true,true,false,false"
    );
    assert_eq!(
        eval(
            "Reflect.apply(Math.max, null, [1, 5, 3]) + Reflect.apply(function () { return this.v; }, { v: 10 }, [])"
        ),
        "15"
    );
    assert_eq!(
        eval(
            "class A { constructor(x) { this.x = x; } } class B extends A {} const b = Reflect.construct(A, [7], B); b.x + ':' + (b instanceof B)"
        ),
        "7:true"
    );
    assert_eq!(
        eval("Reflect.get({ get v() { return this.w; } }, 'v', { w: 9 })"),
        "9"
    );
    assert!(eval("Reflect.construct(() => {}, [])").contains("TypeError"));
    assert!(eval("Reflect.get(1, 'a')").contains("TypeError"));
    assert_eq!(eval("String(Reflect)"), "[object Reflect]");
}

/// Chargeur de modules de test : les textes sont dans `vm.host`, sous leur nom.
fn test_loader(vm: &mut Vm, specifier: &str, _referrer: &str) -> Result<(String, String), String> {
    let name = specifier.trim_start_matches("./");
    let sources = vm
        .host
        .as_ref()
        .and_then(|h| h.downcast_ref::<Vec<(&'static str, &'static str)>>())
        .ok_or("pas de modules")?;
    sources
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(n, source)| ((*n).to_owned(), (*source).to_owned()))
        .ok_or_else(|| format!("{name} absent"))
}

/// Évalue le module `main` parmi `modules`, puis l'expression globale `probe`.
fn eval_modules(modules: &[(&'static str, &'static str)], probe: &str) -> String {
    let mut vm = Vm::new();
    vm.host = Some(Box::new(modules.to_vec()));
    vm.set_module_loader(test_loader);
    let main = modules
        .iter()
        .find(|(n, _)| *n == "main")
        .map_or("", |(_, s)| s);
    if let Err(error) = vm.eval_module(main, "main") {
        return format!("ERREUR {error}");
    }
    match vm.eval(probe) {
        Ok(value) => vm.display(&value),
        Err(error) => format!("ERREUR {error}"),
    }
}

#[test]
fn modules() {
    // Imports nommés, par défaut, renommés, espace de noms.
    assert_eq!(
        eval_modules(
            &[
                ("main", "import defaut, { un, deux as two, somme } from './maths';
                          import * as maths from './maths';
                          globalThis.out = [defaut, un, two, somme(un, two), Object.keys(maths).join('|'), Object.prototype.toString.call(maths), typeof maths.somme].join();"),
                ("maths", "export const un = 1; export let deux = 2; export function somme(a, b) { return a + b; }
                           export default 'défaut';"),
            ],
            "out"
        ),
        "défaut,1,2,3,default|deux|somme|un,[object Module],function"
    );
    // Les liaisons sont vivantes : l'importateur voit les modifications.
    assert_eq!(
        eval_modules(
            &[
                ("main", "import { compteur, incrementer } from './etat'; const avant = compteur; incrementer(); incrementer();
                          globalThis.out = avant + ':' + compteur;"),
                ("etat", "export let compteur = 0; export function incrementer() { compteur++; }"),
            ],
            "out"
        ),
        "0:2"
    );
    // Un module n'est évalué qu'une fois, dépendances d'abord ; sa portée est
    // privée et `this` y vaut `undefined`.
    assert_eq!(
        eval_modules(
            &[
                ("main", "import './a'; import './b'; import { trace } from './journal'; const prive = 1;
                          globalThis.out = trace.join() + ':' + typeof this + ':' + typeof globalThis.prive + ':' + import.meta.url;"),
                ("a", "import { trace } from './journal'; import './b'; trace.push('a');"),
                ("b", "import { trace } from './journal'; trace.push('b');"),
                ("journal", "export const trace = []; trace.push('journal');"),
            ],
            "out + ':' + typeof trace"
        ),
        "journal,b,a:undefined:undefined:main:undefined"
    );
    // Réexportations : nommées, en bloc, sous un espace de noms, par défaut.
    assert_eq!(
        eval_modules(
            &[
                ("main", "import { x, y as z, tout, d, local } from './facade'; globalThis.out = [x, z, tout.x, d, local].join();"),
                ("facade", "export * from './base'; export { y } from './base'; export * as tout from './base';
                            export { default as d } from './base'; const local = 'ici'; export { local };"),
                ("base", "export const x = 1, y = 2; export default 3;"),
            ],
            "out"
        ),
        "1,2,1,3,ici"
    );
    // Formes de `export default` et inférence du nom.
    assert_eq!(
        eval_modules(
            &[
                ("main", "import f from './f'; import C from './c'; import g from './g'; import v from './v';
                          globalThis.out = [f(), f.name, new C().m(), C.name, g.name, v].join();"),
                ("f", "export default function nommee() { return 'f'; }"),
                ("c", "export default class Classe { m() { return 'm'; } }"),
                ("g", "export default () => {};"),
                ("v", "const a = 20; export default a * 2 + 2;"),
            ],
            "out"
        ),
        "f,nommee,m,Classe,default,42"
    );
    // Cycle d'importations : les fonctions hissées sont utilisables, une
    // liaison pas encore initialisée lève ReferenceError.
    assert_eq!(
        eval_modules(
            &[
                (
                    "main",
                    "import { pair } from './pair'; globalThis.out = pair(10) + ':' + pair(7);"
                ),
                (
                    "pair",
                    "import { impair } from './impair'; export function pair(n) { return n === 0 ? true : impair(n - 1); }"
                ),
                (
                    "impair",
                    "import { pair } from './pair'; export function impair(n) { return n === 0 ? false : pair(n - 1); }"
                ),
            ],
            "out"
        ),
        "true:false"
    );
    assert!(
        eval_modules(
            &[
                ("main", "import { b } from './b'; export const a = 1;"),
                ("b", "import { a } from './main'; export const b = a + 1;"),
            ],
            "1"
        )
        .contains("ReferenceError")
    );
    // Fermetures et async dans un module gardent sa portée.
    assert_eq!(
        eval_modules(
            &[
                (
                    "main",
                    "import { lire } from './m'; globalThis.out = 'attente'; lire().then(v => { globalThis.out = v; });"
                ),
                (
                    "m",
                    "const secret = 7; export async function lire() { await null; return secret * 6; }"
                ),
            ],
            "out"
        ),
        "42"
    );
    // Erreurs.
    for (modules, expected) in [
        (
            &[
                ("main", "import { absent } from './m';"),
                ("m", "export const x = 1;"),
            ][..],
            "n'exporte pas",
        ),
        (&[("main", "import './nulle-part';")][..], "introuvable"),
        (
            &[("main", "import './m';"), ("m", "export const = 1;")][..],
            "SyntaxError",
        ),
        (
            &[("main", "import './m';"), ("m", "throw new Error('boum');")][..],
            "Error: boum",
        ),
        (
            &[("main", "export const a = 1; export { a };")][..],
            "exporté deux fois",
        ),
        (
            &[
                ("main", "import { a } from './m'; import { a } from './m';"),
                ("m", "export const a = 1;"),
            ][..],
            "importé deux fois",
        ),
        (
            &[
                ("main", "export { inconnu } from './m';"),
                ("m", "export const a = 1;"),
            ][..],
            "réexporte",
        ),
        (&[("main", "await 1;")][..], "SyntaxError"),
        (&[("main", "new import('./m');")][..], "SyntaxError"),
        (&[("main", "import();")][..], "SyntaxError"),
    ] {
        let result = eval_modules(modules, "1");
        assert!(result.contains(expected), "{modules:?} → {result}");
    }
    // `import()` : une promesse de l'espace de noms, réglée après le code
    // synchrone ; le module n'est évalué qu'une fois.
    assert_eq!(
        eval_modules(
            &[
                ("main", "const trace = []; const p = import('./m'); trace.push(typeof p.then);
                          p.then(ns => import('./m').then(again => {
                              globalThis.out = [...trace, ns.a, ns === again, globalThis.evaluations].join();
                          }));
                          trace.push('fin');"),
                ("m", "export const a = 1; globalThis.evaluations = (globalThis.evaluations || 0) + 1;"),
            ],
            "out"
        ),
        "function,fin,1,true,1"
    );
    assert_eq!(
        eval_modules(
            &[
                ("main", "Promise.allSettled([import('./absent'), import('./casse'), import({ toString() { throw 7; } }), import('./m', {})])
                              .then(r => { globalThis.out = r.map(x => x.status === 'fulfilled' ? 'ok' : (x.reason.name || x.reason)).join(); });"),
                ("casse", "export const a = ;"),
                ("m", "export default 1;"),
            ],
            "out"
        ),
        "TypeError,SyntaxError,7,ok"
    );
    // Hors d'un module, `import` et `export` restent refusés.
    assert!(eval("import x from 'y'").contains("SyntaxError"));
    assert!(eval("export const a = 1").contains("SyntaxError"));
    assert!(eval("import.meta").contains("SyntaxError"));
    assert_eq!(
        crate::module_requests(
            "import a from 'x'; export * from 'y'; import 'x'; export { b } from 'z';"
        )
        .expect("module valide"),
        ["x", "y", "z"]
    );
}

#[test]
fn async_generators_and_for_await() {
    // `next()` rend une promesse ; les valeurs rendues sont attendues.
    assert_eq!(
        eval_then(
            "var out = []; async function* g() { yield 1; const v = await Promise.resolve(2); yield v; yield Promise.resolve(3); return 'fin'; }
             const it = g();
             (async () => { for (let i = 0; i < 5; i++) { const r = await it.next(); out.push(r.value + ':' + r.done); } })();",
            "out.join()"
        ),
        "1:false,2:false,3:false,fin:true,undefined:true"
    );
    assert_eq!(
        eval(
            "async function* g() {} const it = g(); [it.next() instanceof Promise, it[Symbol.asyncIterator]() === it, Object.prototype.toString.call(it), typeof it.return].join()"
        ),
        "true,true,[object AsyncGenerator],function"
    );
    // for await : itérables asynchrones, synchrones, promesses dans un tableau.
    assert_eq!(
        eval_then(
            "var out = []; async function* compte(n) { for (let i = 1; i <= n; i++) { await null; yield i; } }
             (async () => {
               for await (const v of compte(3)) out.push(v);
               for await (const v of [Promise.resolve('a'), 'b']) out.push(v);
               for await (const [k, v] of new Map([['x', 1]])) out.push(k + v);
               const source = { [Symbol.asyncIterator]() { let n = 0; return { next: async () => ({ value: n, done: n++ >= 2 }) }; } };
               for await (const v of source) out.push('s' + v);
               out.push('fin');
             })();",
            "out.join()"
        ),
        "1,2,3,a,b,x1,s0,s1,fin"
    );
    // Les requêtes sont servies dans l'ordre, même émises d'un coup.
    assert_eq!(
        eval_then(
            "var out = []; async function* g() { yield 'a'; yield 'b'; }
             const it = g(); Promise.all([it.next(), it.next(), it.next()]).then(rs => { out = rs.map(r => r.value + ':' + r.done); });",
            "out.join()"
        ),
        "a:false,b:false,undefined:true"
    );
    // Exceptions : levées dans le générateur, rattrapées, ou propagées en rejet.
    assert_eq!(
        eval_then(
            "var out = []; async function* g() { try { yield 1; } catch (e) { out.push('rattrapé ' + e); yield 2; } throw new Error('boum'); }
             (async () => {
               const it = g(); await it.next(); out.push((await it.throw('x')).value);
               try { await it.next(); } catch (e) { out.push(e.message); }
               out.push((await it.next()).done);
               try { for await (const v of (async function* () { yield 1; throw 'arrêt'; })()) out.push(v); } catch (e) { out.push(e); }
               try { for await (const v of [Promise.reject('rejet')]) out.push(v); } catch (e) { out.push(e); }
             })();",
            "out.join()"
        ),
        "rattrapé x,2,boum,true,1,arrêt,rejet"
    );
    // return() et throw() sur un générateur non démarré ou terminé.
    assert_eq!(
        eval_then(
            "var out = []; async function* g() { yield 1; }
             (async () => {
               const a = g(); const r = await a.return(Promise.resolve('tôt')); out.push(r.value + ':' + r.done, (await a.next()).done);
               const b = g(); try { await b.throw('jeté'); } catch (e) { out.push(e); }
               const c = g(); await c.next(); await c.next(); out.push((await c.return(9)).value);
             })();",
            "out.join()"
        ),
        "tôt:true,true,jeté,9"
    );
    // yield* délègue à un itérable asynchrone ou synchrone ; méthodes de classe.
    assert_eq!(
        eval_then(
            "var out = []; async function* interne() { yield 1; yield 2; }
             class C { async *tout() { yield 0; yield* interne(); yield* [3, 4]; } static async *s() { yield 's'; } }
             const o = { async *m() { yield 'm'; } };
             (async () => {
               for await (const v of new C().tout()) out.push(v);
               for await (const v of C.s()) out.push(v);
               for await (const v of o.m()) out.push(v);
               const expr = async function* () { yield 'e'; }; for await (const v of expr()) out.push(v);
             })();",
            "out.join('')"
        ),
        "01234sme"
    );
    // break quitte la boucle ; les étiquettes et continue fonctionnent.
    assert_eq!(
        eval_then(
            "var out = []; async function* infini() { let i = 0; while (true) yield i++; }
             (async () => { for await (const v of infini()) { if (v === 1) continue; if (v > 3) break; out.push(v); } out.push('fin'); })();",
            "out.join()"
        ),
        "0,2,3,fin"
    );
    assert!(eval("function f() { for await (const x of []) {} }").contains("SyntaxError"));
    assert!(eval("async function f() { for await (const x in {}) {} }").contains("SyntaxError"));
    assert!(eval("async function* g() {} new g()").contains("TypeError"));
    assert_eq!(
        eval_then(
            "var out; (async function* () {})().next.call({}).catch(e => { out = e.name; });",
            "out"
        ),
        "TypeError"
    );
}

#[test]
fn array_methods_are_generic() {
    // Les méthodes opèrent sur tout objet muni d'une longueur.
    assert_eq!(
        eval(
            "const o = { length: 3, 0: 'a', 1: 'b', 2: 'c' };
             const A = Array.prototype;
             [A.join.call(o, '-'), A.map.call(o, x => x + x).join(), A.indexOf.call(o, 'c'), A.slice.call(o, 1).join(),
              A.includes.call('xyz', 'y'), A.reduce.call(o, (a, b) => a + b), A.push.call(o, 'd'), o.length, o[3],
              A.pop.call(o), o.length, A.shift.call(o), o[0], o.length, A.filter.call('abc', c => c > 'a').join('')].join()"
        ),
        "a-b-c,aa,bb,cc,2,b,c,true,abc,4,4,d,d,3,a,b,2,bc"
    );
    // Trous : sautés par forEach/map, lus comme undefined par find, préservés par slice.
    assert_eq!(
        eval(
            "const a = [1, , 3]; let vus = 0; a.forEach(() => vus++);
             [vus, a.map(x => x * 2).length, 1 in a.map(x => x), a.findIndex(x => x === undefined), 1 in a.slice(), a.indexOf(undefined), a.includes(undefined)].join()"
        ),
        "2,3,false,1,false,-1,true"
    );
    // Les éléments hérités du prototype sont vus.
    assert_eq!(
        eval("Array.prototype[1] = 'p'; const r = [0, , 2].join(); delete Array.prototype[1]; r"),
        "0,p,2"
    );
    assert_eq!(
        eval(
            "[3, 1, 2].toSorted().join() + [1, 2, 3].toReversed().join() + [1, 2, 3].with(1, 9).join() + [1, 2, 3].toSpliced(0, 1).join()"
        ),
        "1,2,33,2,11,9,32,3"
    );
    assert_eq!(eval("[1, 2, 3, 4, 5].copyWithin(0, 3).join()"), "4,5,3,4,5");
    assert_eq!(
        eval("const a = [5, , 1, undefined, 3]; a.sort(); [a.join(), 4 in a, a.length].join('|')"),
        "1,3,5,,|false|5"
    );
    assert_eq!(
        eval(
            "[[1, [2]], [3]].flat().length + ':' + [1, [2, [3, [4]]]].flat(Infinity).join() + ':' + [1, 2].flatMap(x => [x, [x]]).length"
        ),
        "3:1,2,3,4:4"
    );
    assert_eq!(
        eval(
            "const it = Array.prototype.entries.call({ length: 2, 0: 'x', 1: 'y' }); [...it].join('|')"
        ),
        "0,x|1,y"
    );
    assert_eq!(
        eval("const a = [1]; const it = a.values(); a.push(2); [...it].join()"),
        "1,2"
    );
    assert_eq!(
        eval("Array.from({ length: 2, 0: 'a', 1: 'b' }, (v, i) => v + i).join()"),
        "a0,b1"
    );
    assert!(eval("[].reduce((a, b) => a)").contains("TypeError"));
    assert!(eval("Array.prototype.forEach.call(null, () => {})").contains("TypeError"));
    assert!(eval("[1].forEach(1)").contains("TypeError"));
    // Une longueur démesurée est arrêtée par le budget, pas par un blocage.
    let mut vm = Vm::new();
    vm.set_budget(Some(100_000));
    assert!(
        vm.eval("Array.prototype.forEach.call({ length: 2 ** 53 }, () => {})")
            .is_err()
    );
    assert!(vm.was_interrupted());
}

#[test]
fn typed_arrays_and_buffers() {
    assert_eq!(
        eval(
            "const b = new ArrayBuffer(8); const u8 = new Uint8Array(b); const u32 = new Uint32Array(b, 4, 1);
             u8[0] = 257; u8[1] = -1; u32[0] = 0x01020304; u8[9] = 5;
             [u8.length, u8[0], u8[1], u8[4], u8[7], u8[9], u32.byteOffset, u32.byteLength, b.byteLength, u8.buffer === b,
              Object.prototype.toString.call(u8), u8 instanceof Uint8Array, Uint8Array.BYTES_PER_ELEMENT, '9' in u8, 1 in u8].join()"
        ),
        "8,1,255,4,1,,4,4,8,true,[object Uint8Array],true,1,false,true"
    );
    // Conversions à l'écriture.
    assert_eq!(
        eval(
            "const c = new Uint8ClampedArray([300, -5, 1.5, 2.5]); const i8 = new Int8Array([200]); const f32 = new Float32Array([0.1]);
             const f16 = new Float16Array([1.5, 65520, 1e-8]); const i16 = Int16Array.of(70000, '12');
             [c.join(), i8[0], f32[0] === Math.fround(0.1), f16.join(), i16.join()].join('|')"
        ),
        "255,0,2,2|-56|true|1.5,Infinity,0|4464,12"
    );
    // Méthodes : celles d'Array, plus les variantes typées.
    assert_eq!(
        eval(
            "const a = new Float64Array([3, 1, 2]);
             [a.map(x => x * 2).join(), a.filter(x => x > 1).join(), a.slice(1).join(), a.indexOf(2), a.includes(1),
              a.reduce((s, x) => s + x), [...a].join(), [...a.entries()].join('|'), a.toSorted().join(), a.join(),
              a.sort().join(), a.reverse().join(), a.fill(7, 1).join(), a.at(-1), a.subarray(1).length,
              new Float64Array([1, NaN, -0, 0, -1]).sort().join(), a.map(x => x) instanceof Float64Array, a.with(0, 9).join()].join(';')"
        ),
        "6,2,4;3,2;1,2;2;true;6;3,1,2;0,3|1,1|2,2;1,2,3;3,1,2;1,2,3;3,2,1;3,7,7;7;2;-1,0,0,1,NaN;true;9,7,7"
    );
    // Une vue partage les octets de son tampon ; subarray aussi, slice non.
    assert_eq!(
        eval(
            "const a = new Uint8Array([1, 2, 3, 4]); const vue = a.subarray(1, 3); const copie = a.slice(1, 3);
             vue[0] = 9; copie[1] = 8; const b = new Uint16Array(a.buffer); a.set([5, 6], 2);
             [a.join(), vue.join(), copie.join(), b.length, vue.byteOffset].join('|')"
        ),
        "1,9,5,6|9,5|2,8|2|1"
    );
    // Constructions.
    assert_eq!(
        eval(
            "[new Uint8Array(3).join(), new Uint8Array([1, 2]).join(), new Uint8Array(new Set([7, 8])).join(),
              new Int16Array(new Uint8Array([255])).join(), Uint8Array.from('123', Number).join(), new Uint8Array({ length: 2, 0: 4 }).join(),
              new Float32Array().length].join('|')"
        ),
        "0,0,0|1,2|7,8|255|1,2,3|4,0|0"
    );
    // DataView et boutisme.
    assert_eq!(
        eval(
            "const v = new DataView(new ArrayBuffer(8)); v.setUint16(0, 0x1234); v.setUint16(2, 0x1234, true); v.setFloat32(4, 1.5);
             [v.getUint8(0), v.getUint8(1), v.getUint8(2), v.getUint16(0, true), v.getInt8(3), v.getFloat32(4), v.byteLength, v.getFloat16(4)].join()"
        ),
        "18,52,52,13330,18,1.5,8,1.9375"
    );
    // Tampons redimensionnables et détachés.
    assert_eq!(
        eval(
            "const b = new ArrayBuffer(4, { maxByteLength: 8 }); const suit = new Uint8Array(b); const fixe = new Uint8Array(b, 0, 4);
             b.resize(8); const grand = [suit.length, fixe.length]; b.resize(2);
             const t = b.transfer(); [grand.join(), suit.length, fixe.length, b.detached, b.byteLength, t.byteLength, t.resizable, ArrayBuffer.isView(suit), b.slice === t.slice].join()"
        ),
        "8,4,0,0,true,0,2,true,true,true"
    );
    for source in [
        "new Uint8Array(-1)",
        "new Uint16Array(new ArrayBuffer(3))",
        "new Uint32Array(new ArrayBuffer(8), 2)",
        "new DataView(new ArrayBuffer(1)).getUint16(0)",
        "new ArrayBuffer(2 ** 40)",
        "new Uint8Array(new ArrayBuffer(2), 4)",
        "new ArrayBuffer(8, { maxByteLength: 4 })",
        "new Uint8Array(1).with(3, 0)",
    ] {
        assert!(
            eval(source).contains("RangeError"),
            "{source} → {}",
            eval(source)
        );
    }
    for source in [
        "Uint8Array(2)",
        "new (Object.getPrototypeOf(Uint8Array))()",
        "Uint8Array.prototype.length",
        "ArrayBuffer(1)",
        "const b = new ArrayBuffer(1); const a = new Uint8Array(b); b.transfer(); a.fill(0)",
        "new DataView({})",
        "Uint8Array.prototype.map.call([], x => x)",
    ] {
        assert!(
            eval(source).contains("TypeError"),
            "{source} → {}",
            eval(source)
        );
    }
    assert_eq!(
        eval("JSON.stringify(new Uint8Array([1, 2])) + Object.keys(new Uint8Array(2)).join()"),
        "{\"0\":1,\"1\":2}0,1"
    );
}

#[test]
fn bigints() {
    assert_eq!(
        eval("typeof 1n + ' ' + (2n ** 100n) + ' ' + (0xffn + 0b11n + 0o7n) + ' ' + 1_000n"),
        "bigint 1267650600228229401496703205376 265 1000"
    );
    assert_eq!(
        eval(
            "[7n / 2n, -7n / 2n, 7n % -2n, -7n % 2n, 5n - 8n, 3n * -4n, -(5n), ~5n, 1n << 70n, -5n >> 1n, 6n & 3n, 6n | 3n, 6n ^ 3n].join()"
        ),
        "3,-3,1,-1,-3,-12,-5,-6,1180591620717411303424,-3,2,7,5"
    );
    assert_eq!(
        eval("let x = 9007199254740993n; x++; ++x; x--; String(x) + ':' + (x += 2n)"),
        "9007199254740994:9007199254740996"
    );
    // Comparaisons et égalités, y compris avec des nombres et des chaînes.
    assert_eq!(
        eval(
            "[1n < 2, 2 < 1n, 1n == 1, 1n === 1, 1n == '1', 2n > 1.5, 1n < NaN, 10n > '9', 0n == false, 1n === 1n, 2n >= 2n, 1n < 'x'].join()"
        ),
        "true,false,true,false,true,true,false,true,true,true,true,false"
    );
    // Conversions.
    assert_eq!(
        eval(
            "[BigInt(42), BigInt('0x10'), BigInt(true), BigInt(' 12 '), Number(2n ** 64n), String(-3n), 5n + '', `${7n}`, (255n).toString(16), parseInt(1n), Boolean(0n), !!1n, 0n ? 'a' : 'b'].join()"
        ),
        "42,16,1,12,18446744073709552000,-3,5,7,ff,1,false,true,b"
    );
    assert_eq!(
        eval(
            "[BigInt.asUintN(8, -1n), BigInt.asIntN(8, 255n), BigInt.asIntN(64, 2n ** 63n), BigInt.asUintN(3, 9n)].join()"
        ),
        "255,-1,-9223372036854775808,1"
    );
    assert_eq!(
        eval(
            "const o = { 1n: 'a' }; o[1] + o['1'] + [10, 20][1n] + Object(1n).valueOf() + typeof Object(1n)"
        ),
        "aa201object"
    );
    assert_eq!(
        eval(
            "const m = new Map([[1n, 'un']]); m.get(1n) + m.has(1) + new Set([1n, 1n, 2n]).size + [1n, 2n].includes(1n) + [3n, 1n, 2n].sort().join()"
        ),
        "unfalse2true1,2,3"
    );
    assert_eq!(
        eval("Object.prototype.toString.call(1n) + (1n).constructor.name"),
        "[object BigInt]BigInt"
    );
    // Mélanges et conversions implicites interdits.
    for source in [
        "1n + 1",
        "1 * 2n",
        "+1n",
        "Math.abs(1n)",
        "1n >>> 0n",
        "BigInt(undefined)",
        "new BigInt(1)",
        "JSON.stringify({ a: 1n })",
        "BigInt.asIntN(8, 1)",
        "isNaN(1n)",
    ] {
        assert!(
            eval(source).contains("TypeError"),
            "{source} → {}",
            eval(source)
        );
    }
    for source in [
        "1n / 0n",
        "1n % 0n",
        "2n ** -1n",
        "BigInt(1.5)",
        "BigInt(NaN)",
        "2n ** (2n ** 30n)",
        "(1n).toString(1)",
        "1n << (2n ** 40n)",
    ] {
        assert!(
            eval(source).contains("RangeError"),
            "{source} → {}",
            eval(source)
        );
    }
    assert!(eval("BigInt('1.5')").contains("SyntaxError"));
    for source in ["1.5n", "1e3n", "01n", "1n2", ".5n"] {
        assert!(
            eval(source).contains("SyntaxError"),
            "{source} → {}",
            eval(source)
        );
    }
}

#[test]
fn bigint_typed_arrays() {
    assert_eq!(
        eval(
            "const a = new BigInt64Array([1n, -1n, 2n ** 63n]); const u = new BigUint64Array(a.buffer);
             const v = new DataView(new ArrayBuffer(8)); v.setBigInt64(0, -2n); a[0] = 5n;
             [a.join(), u[1], typeof a[0], a.map(x => x * 2n).join(), a.toSorted().join(), a.includes(5n), a.fill(7n, 2).join(),
              v.getBigUint64(0), v.getBigInt64(0), v.getUint8(7), BigInt64Array.BYTES_PER_ELEMENT, BigInt64Array.of(3n).with(0, 4n).join()].join('|')"
        ),
        "5,-1,-9223372036854775808|18446744073709551615|bigint|10,-2,0|-9223372036854775808,-1,5|true|5,-1,7|18446744073709551614|-2|254|8|4"
    );
    for source in [
        "new BigInt64Array([1])",
        "new BigInt64Array(1)[0] = 1",
        "new Uint8Array([1n])",
        "new BigInt64Array(new Uint8Array(1))",
        "new BigInt64Array(1).fill(1)",
    ] {
        assert!(
            eval(source).contains("TypeError"),
            "{source} → {}",
            eval(source)
        );
    }
}

#[test]
fn dates() {
    assert_eq!(
        eval(
            "const d = new Date(2022, 1, 1, 10, 20, 30, 456);
             [d.getTime(), d.getFullYear(), d.getMonth(), d.getDate(), d.getDay(), d.getHours(), d.getMinutes(), d.getSeconds(),
              d.getMilliseconds(), d.toISOString(), d.toUTCString(), d.toDateString(), d.getTimezoneOffset(), JSON.stringify(d)].join('|')"
        ),
        "1643710830456|2022|1|1|2|10|20|30|456|2022-02-01T10:20:30.456Z|Tue, 01 Feb 2022 10:20:30 GMT|Tue Feb 01 2022|0|\"2022-02-01T10:20:30.456Z\""
    );
    assert_eq!(
        eval(
            "const d = new Date(0); d.setFullYear(2000, 1, 29); d.setHours(25); const a = d.toISOString();
             d.setMonth(12); d.setDate(0); d.setMilliseconds(-1);
             [a, d.toISOString(), d.setTime(NaN), d.getFullYear(), String(d), d.setFullYear(1999), d.getMonth()].join('|')"
        ),
        "2000-03-01T01:00:00.000Z|2000-12-31T00:59:59.999Z|NaN|NaN|Invalid Date|915148800000|0"
    );
    assert_eq!(
        eval(
            "[Date.UTC(2020, 0), Date.parse('2020-01-01'), Date.parse('Wed, 01 Jan 2020 00:00:00 GMT'), Date.parse('x'), new Date('2020-01-01T00:00:00+01:00').getTime(),
              new Date(99, 0).getFullYear(), new Date(new Date(5)).getTime(), new Date(8.64e15 + 1).getTime(), typeof Date(), typeof Date.now(), new Date(2020, 0) < new Date(2021, 0)].join('|')"
        ),
        "1577836800000|1577836800000|1577836800000|NaN|1577833200000|1999|5|NaN|string|number|true"
    );
    // Une date préfère la chaîne dans `+`, le nombre dans les opérations numériques.
    assert_eq!(
        eval(
            "const d = new Date(86400000); [typeof (d + 1), d - 1, +d, d == d.toString(), `${d}` === d.toString(), d[Symbol.toPrimitive]('number')].join('|')"
        ),
        "string|86399999|86400000|true|true|86400000"
    );
    assert_eq!(
        eval("const o = { [Symbol.toPrimitive](h) { return h; } }; [`${o}`, o + '', o * 1].join()"),
        "string,default,NaN"
    );
    assert!(eval("new Date(NaN).toISOString()").contains("RangeError"));
    assert!(eval("Date.prototype.getTime.call({})").contains("TypeError"));
    assert!(eval("({ [Symbol.toPrimitive]() { return {}; } }) + 1").contains("TypeError"));
    assert_eq!(
        eval("new Date(NaN).toJSON() + Object.prototype.toString.call(new Date())"),
        "null[object Date]"
    );
}

#[test]
fn private_members_and_escaped_identifiers() {
    assert_eq!(
        eval(
            "class C {
               #x = 1; static #compte = 0; #m() { return this.#x * 2; } static #s() { return 'statique'; }
               get #g() { return 'lu'; } set #g(v) { this.#x = v; }
               static creer() { C.#compte++; return new C(); }
               tout() { this.#x += 10; this.#g = this.#x + 1; return [this.#x, this.#m(), this.#g, C.#s(), C.#compte, #x in this, #x in {}].join(); }
               static a(o) { return #x in o; }
             }
             const c = C.creer();
             [c.tout(), Object.keys(c).length, Object.getOwnPropertyNames(C).includes('#s'), Reflect.ownKeys(c).length,
              JSON.stringify(c), Object.getOwnPropertySymbols(c).length, C.a(c), C.a(new (class {})())].join('|')"
        ),
        "12,24,lu,statique,1,true,false|0|false|0|{}|0|true|false"
    );
    // Chaque classe a ses propres clés : un nom identique ne donne pas accès.
    assert!(
        eval("class A { #x = 1; static lire(o) { return o.#x; } } class B { #x = 2; } A.lire(new B())").contains("TypeError")
    );
    assert!(
        eval("class A { #x; static ecrire(o) { o.#x = 1; } } A.ecrire({})").contains("TypeError")
    );
    assert!(eval("class A { m() { return #x; } }").contains("SyntaxError"));
    // Une classe interne voit les noms privés de la classe englobante.
    assert_eq!(
        eval(
            "class Ext { #v = 7; interne() { const self = this; return new (class { lire() { return self.#v; } })().lire(); } } new Ext().interne()"
        ),
        "7"
    );
    // Les champs sont définis, sans passer par un mutateur hérité.
    assert_eq!(
        eval(
            "let appels = 0; class P { set x(v) { appels++; } } class E extends P { x = 1; f = () => 2; } const e = new E(); [appels, e.x, e.f.name, Object.hasOwn(e, 'x')].join()"
        ),
        "0,1,f,true"
    );
    // Identifiants écrits avec des échappements Unicode.
    assert_eq!(
        eval(
            "var \\u0061b = 2; var \\u{63}d = 3; ab + cd + (() => { let cl\\u0065 = 1; return cle; })()"
        ),
        "6"
    );
    assert_eq!(
        eval(
            "class U { #\\u{6F} = 4; static l(u) { return u.#o; } } U.l(new U()) + ({ \\u0078: 1 }).x"
        ),
        "5"
    );
    assert_eq!(eval("var ℘ = 1, a\u{200c}b = 2; ℘ + a\u{200c}b"), "3");
    assert!(eval("var \\u0031a = 1").contains("SyntaxError"));
    assert!(eval("var a\\x41 = 1").contains("SyntaxError"));
    assert_eq!(eval("#!/usr/bin/env oxyde\n1 + 1"), "2");
}

#[test]
fn early_errors() {
    // Chaque source est une erreur de syntaxe, signalée avant toute exécution.
    let rejected = [
        // Redéclarations.
        "let a; let a;",
        "let a; var a;",
        "{ let b; { var b; } }",
        "a => { let a; }",
        "switch (0) { case 1: let a; case 2: let a; }",
        "try {} catch (e) { let e; }",
        "{ function f() {} var f; }",
        // Mode strict.
        "'use strict'; 010",
        "'use strict'; '\\1'",
        "'use strict'; var eval;",
        "'use strict'; arguments = 1;",
        "'use strict'; eval++;",
        "'use strict'; delete x;",
        "'use strict'; var static;",
        "'use strict'; if (1) function f() {}",
        "function f() { 'use strict'; var yield; }",
        "function f(a, a) { 'use strict'; }",
        "function eval() { 'use strict'; }",
        "function f(a = 1) { 'use strict'; }",
        "'\\01'; 'use strict';",
        "class C { m() { var let; } }",
        // Mots réservés selon le contexte.
        "function* g() { var yield; }",
        "async function f() { var await; }",
        "function* g(x = yield) {}",
        "(function* yield() {})",
        "let let = 1",
        "v\\u0061r x = 1",
        "var x = tru\\u0065",
        "cl\\u0061ss C {}",
        // Noms privés.
        "class C { #a; #a; }",
        "class C { get #a() {} get #a() {} }",
        "class C { m() { this.#b } }",
        "this.#x",
        "class C { #constructor }",
        "class C { #a; m() { delete this.#a } }",
        // `super`, `new.target`, classes.
        "super.x",
        "function f() { super.x }",
        "class C { constructor() { super(); } }",
        "new.target",
        "class C { get constructor() {} }",
        "class C { static prototype() {} }",
        "class C { constructor() {} constructor() {} }",
        "class C { x = arguments; }",
        "class C { static { return; } }",
        "class C { static { await; } }",
        "class C { static { var arguments; } }",
        // Instructions et étiquettes.
        "x: x: 1",
        "if (1) class C {}",
        "while (1) function f() {}",
        "if (1) let x;",
        "for (;;) const a = 1;",
        // Littéraux.
        "({ a = 1 })",
        "f({ a = 1 })",
        "({ __proto__: 1, __proto__: 2 })",
        "({ get a(x) {} })",
        "({ set a() {} })",
        "1_000_",
        "1__0",
        "0x_1",
        "0_1",
        "`\\07`",
        "/(?<a>.)(?<a>.)/",
        "/a**/",
        "(a, a) => 1",
        "-a ** 2",
    ];
    for source in rejected {
        assert!(
            eval(source).contains("SyntaxError"),
            "devrait être refusé : {source}"
        );
    }
    // Ce qui y ressemble mais reste valide.
    assert_eq!(
        eval("var a; var a; function f(a, a) { return a } f(1, 2)"),
        "2"
    );
    assert_eq!(eval("010 + 1 + '\\101' + 08"), "9A8");
    assert_eq!(eval("if (1) function f() { return 1 } 1"), "1");
    assert_eq!(
        eval("({ a = 1 } = {}); [{ b = 2 }] = [{}]; for ({ c = 3 } of [{}]) ; [a, b, c].join()"),
        "1,2,3"
    );
    assert_eq!(
        eval("var p, q; ({ __proto__: p, __proto__: q } = { __proto__: 5 }); q"),
        "5"
    );
    assert_eq!(
        eval("var yield = 1, await = 2, let = 3, static = 4; yield + await + let + static"),
        "10"
    );
    assert_eq!(
        eval("function f() { return new.target } String(f())"),
        "undefined"
    );
    assert_eq!(eval("l: for (;;) { break l; } m: 1; m: 2"), "2");
    assert_eq!(eval("try {} catch (e) { var e; } 1"), "1");
    assert_eq!(eval("1_000.5_5e1_0"), "10005500000000");
    assert_eq!(
        eval("var o = { eval: 1, static: 2, v\\u0061r: 3 }; o.eval + o.static + o.var"),
        "6"
    );
    assert_eq!(
        eval(
            "class A { #a = 1; get #b() { return this.#a } set #b(v) {} static #b2; static t(o) { return o.#b } } A.t(new A)"
        ),
        "1"
    );
    assert_eq!(
        eval(
            "class B extends Object { constructor() { super(); this.x = () => super.toString; } } typeof new B().x"
        ),
        "function"
    );
    assert_eq!(
        eval("function g() { 'use strict' } function h(a, a) {} 0o17"),
        "15"
    );
    assert_eq!(eval("(-2) ** 2 + 2 ** -1"), "4.5");
    // Membres statiques : générateurs, blocs, ordre d'évaluation et `this`.
    assert_eq!(
        eval(
            "class C { static *g() { yield 1 } static async *h() {} static x = this.name + 1;
               static { C.y = this.x + '!'; } static z = C.y; static f = function () {}; static a = () => this }
             [C.g().next().value, C.x, C.y, C.z, C.f.name, C.a() === C].join()"
        ),
        "1,C1,C1!,C1!,f,true"
    );
    assert_eq!(
        eval(
            "var log = []; class D { static a = log.push('a'); m() {} static { log.push('b') }
               static [(log.push('k'), 'c')] = log.push('c'); static #p = 5; static r() { return D.#p } }
             log.join() + D.c + D.r()"
        ),
        "k,a,b,c45"
    );
    assert_eq!(eval("let\nx = 1; if (0) let\nx; x"), "1");
}

#[test]
fn property_descriptors() {
    // Un descripteur partiel ne touche que les attributs qu'il nomme.
    assert_eq!(
        eval(
            "var o = { x: 1 }; Object.defineProperty(o, 'x', { enumerable: false });
             var d = Object.getOwnPropertyDescriptor(o, 'x');
             Object.defineProperty(o, 'y', { value: 2 }); o.y = 3;
             var e = Object.getOwnPropertyDescriptor(o, 'y');
             [d.value, d.writable, d.enumerable, d.configurable, o.y, e.writable, e.enumerable, e.configurable, Object.keys(o).length].join()"
        ),
        "1,true,false,true,2,false,false,false,0"
    );
    // Une propriété non configurable refuse d'être redéfinie autrement.
    assert!(
        eval("var o = {}; Object.defineProperty(o, 'x', { value: 1 }); Object.defineProperty(o, 'x', { value: 2 })")
            .contains("TypeError")
    );
    assert_eq!(
        eval(
            "var o = {}; Object.defineProperty(o, 'x', { value: 1 }); [Reflect.defineProperty(o, 'x', { value: 1 }), Reflect.defineProperty(o, 'x', { get() {} }), Reflect.set(o, 'x', 5), delete o.x].join()"
        ),
        "true,false,false,false"
    );
    assert!(eval("Object.defineProperty({}, 'x', { get() {}, value: 1 })").contains("TypeError"));
    // Tableaux : accesseur sur un index, longueur, gel.
    assert_eq!(
        eval(
            "var a = [1, 2, 3]; Object.defineProperty(a, 1, { get() { return 9 } }); a[1] = 5; a.length = 2;
             var b = Object.freeze([1, 2]); b[0] = 5; b[5] = 1;
             var c = [1, 2, 3]; Object.defineProperty(c, 1, { configurable: false }); c.length = 0;
             [a.join(), a.length, Object.keys(a).join('|'), b.join(), b.length, Object.isFrozen(b), c.length, Object.getOwnPropertyDescriptor(b, 'length').writable].join()"
        ),
        "1,9,2,0|1,1,2,2,true,2,false"
    );
    assert!(eval("[].length = -1").contains("RangeError"));
    // Intégrité, extensibilité, Object.is.
    assert_eq!(
        eval(
            "var s = Object.seal({ a: 1 }); s.a = 2; s.b = 3; delete s.a;
             var p = Object.preventExtensions({}); p.x = 1;
             [s.a, s.b, Object.isSealed(s), Object.isFrozen(s), Object.isExtensible(p), p.x, Object.is(NaN, NaN), Object.is(0, -0),
              Object.keys(Object.getOwnPropertyDescriptors({ a: 1, get b() { return 1 } })).join('|')].join()"
        ),
        "2,,true,false,false,,true,false,a|b"
    );
    // `name` et `length` d'une fonction : figés mais supprimables.
    assert_eq!(
        eval(
            "function f(a, b) {} f.name = 'x'; var avant = [f.name, f.length, Reflect.ownKeys(f).join('|')].join();
             delete f.name; delete f.length; [avant, f.name, f.length, f.hasOwnProperty('name'), Reflect.ownKeys(f).join('|')].join()"
        ),
        "f,2,length|name|prototype,,0,false,prototype"
    );
    // Une propriété propre non énumérable masque celle du prototype dans for…in.
    assert_eq!(
        eval(
            "var proto = { a: 1, b: 2 }; var o = Object.create(proto, { a: { value: 3, enumerable: false } }); o.c = 4;
             var seen = []; for (var k in o) seen.push(k); seen.join()"
        ),
        "c,b"
    );
    assert_eq!(
        eval(
            "class A { static [Symbol.hasInstance](v) { return v === 1 } } [1 instanceof A, 2 instanceof A].join()"
        ),
        "true,false"
    );
    assert_eq!(
        eval(
            "var o = {}; o.__defineGetter__('x', function () { return 7 }); [o.x, typeof o.__lookupGetter__('x'), o.__lookupSetter__('x')].join()"
        ),
        "7,function,"
    );
}

#[test]
fn proxies() {
    assert_eq!(
        eval(
            "var journal = [];
             var p = new Proxy({ a: 1 }, {
                 get(t, k, r) { journal.push('get ' + String(k)); return k === 'b' ? 42 : Reflect.get(t, k, r); },
                 set(t, k, v, r) { journal.push('set ' + k); return Reflect.set(t, k, v * 2, r); },
                 has(t, k) { return k === 'z' || k in t; },
                 deleteProperty(t, k) { journal.push('delete ' + k); return delete t[k]; },
                 ownKeys(t) { return [...Reflect.ownKeys(t), 'q']; },
                 getOwnPropertyDescriptor(t, k) { return k === 'q' ? { value: 1, enumerable: true, configurable: true } : Reflect.getOwnPropertyDescriptor(t, k); },
                 defineProperty(t, k, d) { journal.push('define ' + k); return Reflect.defineProperty(t, k, d); },
             });
             p.c = 5; delete p.a; Object.defineProperty(p, 'd', { value: 1, configurable: true });
             [p.b, p.c, 'z' in p, 'a' in p, Object.keys(p).join('|'), typeof p, journal.join('|')].join()"
        ),
        "42,10,true,false,c|q,object,set c|define c|delete a|define d|get b|get c"
    );
    // Sans trappe, tout va à la cible ; un proxy de tableau reste un tableau.
    assert_eq!(
        eval(
            "var t = [1, 2]; var p = new Proxy(t, {}); p.push(3);
             [t.length, Array.isArray(p), Object.prototype.toString.call(p), JSON.stringify(Object.getOwnPropertyNames(p)), p instanceof Array].join()"
        ),
        "3,true,[object Array],[\"0\",\"1\",\"2\",\"length\"],true"
    );
    // Appel et construction.
    assert_eq!(
        eval(
            "var f = new Proxy(function (a) { return a * 2; }, {
                 apply(t, self, args) { return t(...args) + 1; },
                 construct(t, args, nt) { return { v: args[0], nt: nt === f }; },
             });
             var g = new Proxy(class { constructor(x) { this.x = x; } }, {});
             [f(2), new f(7).v, new f(7).nt, typeof f, new g(3).x, typeof new Proxy({}, {})].join()"
        ),
        "5,7,true,function,3,object"
    );
    // Prototype, extensibilité, for…in et affectation à travers un prototype.
    assert_eq!(
        eval(
            "var proto = { h: 1 };
             var p = new Proxy({}, { getPrototypeOf() { return proto; }, isExtensible(t) { return Reflect.isExtensible(t); } });
             var enfant = Object.create(new Proxy({}, { set(t, k, v, r) { return Reflect.defineProperty(r, 'vu', { value: k, enumerable: true, configurable: true }); }, ownKeys() { return ['m']; },
                 getOwnPropertyDescriptor() { return { value: 1, enumerable: true, configurable: true }; } }));
             enfant.x = 1; var cles = []; for (var k in enfant) cles.push(k);
             [Object.getPrototypeOf(p) === proto, p instanceof Object, Object.isExtensible(p), enfant.vu, 'x' in enfant, cles.join('|')].join()"
        ),
        "true,true,true,x,false,vu|m"
    );
    // Invariants : une trappe ne peut pas mentir sur une propriété figée.
    for source in [
        "var t = {}; Object.defineProperty(t, 'x', { value: 1 }); new Proxy(t, { get() { return 2; } }).x",
        "var t = {}; Object.defineProperty(t, 'x', { value: 1 }); 'x' in new Proxy(t, { has() { return false; } })",
        "var t = Object.preventExtensions({ a: 1 }); Object.keys(new Proxy(t, { ownKeys() { return []; } }))",
        "var t = {}; Object.defineProperty(t, 'x', { value: 1 }); delete new Proxy(t, { deleteProperty() { return true; } }).x",
        "Object.getPrototypeOf(new Proxy({}, { getPrototypeOf() { return 1; } }))",
        "new Proxy({}, { get: 1 }).x",
        "Proxy({}, {})",
        "new Proxy(1, {})",
        "new (new Proxy(() => {}, {}))()",
    ] {
        assert!(eval(source).contains("TypeError"), "{source}");
    }
    // Révocation.
    assert_eq!(
        eval(
            "var r = Proxy.revocable({ a: 1 }, {}); var avant = r.proxy.a; r.revoke(); r.revoke();
             var apres; try { r.proxy.a; } catch (e) { apres = e.name; } [avant, apres, typeof r.proxy].join()"
        ),
        "1,TypeError,object"
    );
}

#[test]
fn strict_mode_at_runtime() {
    // Hors mode strict, une écriture refusée est silencieuse.
    assert_eq!(
        eval(
            "var o = Object.freeze({ a: 1 }); o.a = 2; o.b = 3; var g = { get x() { return 1; } }; g.x = 2;
             non_declaree = 5; [o.a, o.b, g.x, delete o.a, non_declaree, (function () { return this === globalThis; })(),
              (function () { return typeof this; }).call(1)].join()"
        ),
        "1,,1,false,5,true,object"
    );
    // En mode strict, chacune lève une erreur et `this` n'est pas converti.
    for (source, expected) in [
        ("'use strict'; Object.freeze({ a: 1 }).a = 2", "TypeError"),
        (
            "'use strict'; Object.preventExtensions({}).b = 1",
            "TypeError",
        ),
        (
            "'use strict'; ({ get x() { return 1; } }).x = 2",
            "TypeError",
        ),
        (
            "'use strict'; delete Object.freeze({ a: 1 }).a",
            "TypeError",
        ),
        (
            "'use strict'; var a = Object.freeze([1]); a[0] = 2",
            "TypeError",
        ),
        ("'use strict'; 'abc'.length = 1", "TypeError"),
        ("'use strict'; non_declaree = 5", "ReferenceError"),
        (
            "function f() { 'use strict'; undefined_ici = 1; } f()",
            "ReferenceError",
        ),
        (
            "class C { m() { Object.freeze(this).x = 1; } } new C().m()",
            "TypeError",
        ),
    ] {
        assert!(
            eval(source).contains(expected),
            "{source} → {}",
            eval(source)
        );
    }
    assert_eq!(
        eval(
            "'use strict'; [(function () { return this; })() === undefined, (function () { return typeof this; }).call(1),
              (() => this === globalThis)(), typeof (function () { return this; }).call('a')].join()"
        ),
        "true,number,true,string"
    );
    // Le mode est celui de la fonction, pas de l'appelant.
    assert_eq!(
        eval(
            "function laxiste() { return this === globalThis; } (function () { 'use strict'; return laxiste(); })()"
        ),
        "true"
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

    assert!(matches!(vm.eval("with (a) {}"), Err(EvalError::Syntax(_))));
    assert!(eval("/(/.test('aa')").contains("SyntaxError"));
    assert_eq!(eval("new Function('a', 'b', 'return a + b')(2, 3)"), "5");
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
