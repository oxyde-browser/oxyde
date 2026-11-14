// `Intl` (ECMA-402), en version réduite, écrit en JavaScript.
//
// Une grande part des sites lit `Intl` sans vérifier qu'il existe : son
// absence arrête leurs scripts. Cette version met en forme nombres, dates,
// pluriels, listes et durées relatives pour le français et l'anglais.
//
// LIMITES CONNUES : deux langues seulement (`fr`, `en` ; toute autre langue
// demandée est servie en français) ; calendrier grégorien, chiffres latins ;
// les fuseaux horaires se limitent à `UTC` et à l'heure du système (un autre
// fuseau nommé est traité comme l'heure du système, et le nom du fuseau du
// système est déduit de son décalage) ; `Collator` compare sans table de
// collation (accents et casse en second) ; `Segmenter` découpe par règles
// simples ; pas d'ères, pas de `formatRangeToParts`, pas de `DurationFormat` ;
// les arrondis sont ceux de `toFixed`.
(function () {
  "use strict";
  const global = globalThis;
  const define = Object.defineProperty;
  const NBSP = " ";
  const NNBSP = " ";

  const DATA = {
    fr: {
      decimal: ",", group: NNBSP, percent: NNBSP + "%", minus: "-",
      months: ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août",
        "septembre", "octobre", "novembre", "décembre"],
      monthsShort: ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.",
        "oct.", "nov.", "déc."],
      days: ["dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi"],
      daysShort: ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."],
      dayPeriods: ["AM", "PM"],
      compact: [["", 1], [NBSP + "k", 1e3], [NBSP + "M", 1e6], [NBSP + "Md", 1e9], [NBSP + "Bn", 1e12]],
      and: " et ", or: " ou ", at: " à ",
      units: {
        second: ["seconde", "secondes"], minute: ["minute", "minutes"], hour: ["heure", "heures"],
        day: ["jour", "jours"], week: ["semaine", "semaines"], month: ["mois", "mois"],
        quarter: ["trimestre", "trimestres"], year: ["an", "ans"],
      },
      relative: (text, past) => (past ? "il y a " + text : "dans " + text),
      named: {
        second: { 0: "maintenant" }, minute: { 0: "cette minute-ci" }, hour: { 0: "cette heure-ci" },
        day: { "-2": "avant-hier", "-1": "hier", 0: "aujourd’hui", 1: "demain", 2: "après-demain" },
        week: { "-1": "la semaine dernière", 0: "cette semaine", 1: "la semaine prochaine" },
        month: { "-1": "le mois dernier", 0: "ce mois-ci", 1: "le mois prochain" },
        quarter: { "-1": "le trimestre dernier", 0: "ce trimestre", 1: "le trimestre prochain" },
        year: { "-1": "l’année dernière", 0: "cette année", 1: "l’année prochaine" },
      },
      languages: { fr: "français", en: "anglais", de: "allemand", es: "espagnol", it: "italien" },
      regions: { FR: "France", US: "États-Unis", GB: "Royaume-Uni", DE: "Allemagne", BE: "Belgique",
        CH: "Suisse", CA: "Canada", ES: "Espagne", IT: "Italie" },
      currencies: { EUR: "euro", USD: "dollar des États-Unis", GBP: "livre sterling" },
    },
    en: {
      decimal: ".", group: ",", percent: "%", minus: "-",
      months: ["January", "February", "March", "April", "May", "June", "July", "August",
        "September", "October", "November", "December"],
      monthsShort: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
      days: ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"],
      daysShort: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
      dayPeriods: ["AM", "PM"],
      compact: [["", 1], ["K", 1e3], ["M", 1e6], ["B", 1e9], ["T", 1e12]],
      and: " and ", or: " or ", at: " at ",
      units: {
        second: ["second", "seconds"], minute: ["minute", "minutes"], hour: ["hour", "hours"],
        day: ["day", "days"], week: ["week", "weeks"], month: ["month", "months"],
        quarter: ["quarter", "quarters"], year: ["year", "years"],
      },
      relative: (text, past) => (past ? text + " ago" : "in " + text),
      named: {
        second: { 0: "now" }, minute: { 0: "this minute" }, hour: { 0: "this hour" },
        day: { "-1": "yesterday", 0: "today", 1: "tomorrow" },
        week: { "-1": "last week", 0: "this week", 1: "next week" },
        month: { "-1": "last month", 0: "this month", 1: "next month" },
        quarter: { "-1": "last quarter", 0: "this quarter", 1: "next quarter" },
        year: { "-1": "last year", 0: "this year", 1: "next year" },
      },
      languages: { fr: "French", en: "English", de: "German", es: "Spanish", it: "Italian" },
      regions: { FR: "France", US: "United States", GB: "United Kingdom", DE: "Germany",
        BE: "Belgium", CH: "Switzerland", CA: "Canada", ES: "Spain", IT: "Italy" },
      currencies: { EUR: "Euro", USD: "US Dollar", GBP: "British Pound" },
    },
  };
  const SYMBOLS = { EUR: "€", USD: "$", GBP: "£", JPY: "¥", CHF: "CHF", CAD: "CA$", AUD: "A$", CNY: "CN¥" };
  const NO_CENTS = new Set(["JPY", "KRW", "VND", "CLP", "ISK", "HUF"]);

  // ---- Locales ------------------------------------------------------------------------------------------------
  function canonical(tag) {
    if (typeof tag !== "string") {
      if (tag instanceof Locale) return tag.toString();
      throw new TypeError("Intl : une étiquette de langue est attendue");
    }
    const parts = tag.trim().replace(/_/g, "-").split("-");
    if (!/^[a-zA-Z]{2,3}$|^[a-zA-Z]{5,8}$/.test(parts[0])) {
      throw new RangeError("Intl : étiquette de langue invalide : " + tag);
    }
    const out = [parts[0].toLowerCase()];
    let extension = false;
    for (const part of parts.slice(1)) {
      if (!/^[a-zA-Z0-9]{1,8}$/.test(part)) throw new RangeError("Intl : étiquette de langue invalide : " + tag);
      if (part.length === 1) extension = true;
      if (extension) out.push(part.toLowerCase());
      else if (part.length === 4 && /^[a-zA-Z]+$/.test(part)) out.push(part[0].toUpperCase() + part.slice(1).toLowerCase());
      else if (part.length === 2 || /^\d{3}$/.test(part)) out.push(part.toUpperCase());
      else out.push(part.toLowerCase());
    }
    return out.join("-");
  }
  function requested(locales) {
    if (locales === undefined) return [];
    const list = typeof locales === "string" || locales instanceof Locale ? [locales] : Array.from(locales);
    return Array.from(new Set(list.map(canonical)));
  }
  const defaultLocale = () => {
    try {
      return canonical(global.navigator?.language ?? "fr-FR");
    } catch {
      return "fr-FR";
    }
  };
  const languageOf = (tag) => tag.split("-")[0];
  const regionOf = (tag) => tag.split("-").slice(1).find((part) => /^[A-Z]{2}$|^\d{3}$/.test(part));
  function resolve(locales) {
    const found = requested(locales).find((tag) => languageOf(tag) in DATA);
    const locale = (found ?? defaultLocale()).replace(/-u(-[a-z0-9]{2,8})+/, "");
    const language = languageOf(locale) in DATA ? languageOf(locale) : "fr";
    return { locale: languageOf(locale) in DATA ? locale : "fr-FR", data: DATA[language], language, region: regionOf(locale) };
  }
  const supported = (locales) => requested(locales).filter((tag) => languageOf(tag) in DATA);
  function option(options, name, allowed, fallback) {
    const value = options?.[name];
    if (value === undefined) return fallback;
    const text = String(value);
    if (allowed !== undefined && !allowed.includes(text)) {
      throw new RangeError("Intl : valeur « " + text + " » refusée pour l'option " + name);
    }
    return text;
  }
  function numberOption(options, name, low, high, fallback) {
    const value = options?.[name];
    if (value === undefined) return fallback;
    const number = Number(value);
    if (!(number >= low && number <= high)) {
      throw new RangeError("Intl : valeur hors bornes pour l'option " + name);
    }
    return Math.floor(number);
  }
  // Un constructeur appelable avec ou sans `new`, comme le veut la norme
  // pour les plus anciens.
  function service(name, initialize, methods) {
    const Service = {
      [name]: function (locales, options) {
        const target = new.target === undefined ? Object.create(Service.prototype) : this;
        initialize(target, locales, options === undefined ? undefined : Object(options));
        return target;
      },
    }[name];
    for (const [key, value] of Object.entries(methods)) {
      define(Service.prototype, key, { value, writable: true, enumerable: false, configurable: true });
    }
    define(Service.prototype, Symbol.toStringTag, { value: "Intl." + name, configurable: true });
    define(Service, "supportedLocalesOf", { value: supported, writable: true, configurable: true });
    return Service;
  }
  const state = new WeakMap();
  const internal = (object, name) => {
    const found = state.get(object);
    if (found === undefined) throw new TypeError("Intl." + name + " : receveur incompatible");
    return found;
  };
  // `format` et `compare` sont des accesseurs qui rendent une fonction liée.
  const bound = (Service, name, key, call) => {
    define(Service.prototype, key, {
      get() {
        const self = internal(this, name);
        self.bound ??= (...args) => call(self, ...args);
        return self.bound;
      },
      enumerable: false,
      configurable: true,
    });
  };

  // ---- Pluriels ------------------------------------------------------------------------------------------------
  function pluralCategory(language, type, value) {
    const n = Math.abs(Number(value));
    const integer = Number.isInteger(n);
    if (type === "ordinal") {
      if (language === "fr") return n === 1 ? "one" : "other";
      const [ten, hundred] = [n % 10, n % 100];
      if (ten === 1 && hundred !== 11) return "one";
      if (ten === 2 && hundred !== 12) return "two";
      if (ten === 3 && hundred !== 13) return "few";
      return "other";
    }
    if (language === "fr") return n >= 0 && n < 2 ? "one" : "other";
    return n === 1 && integer ? "one" : "other";
  }
  const PluralRules = service("PluralRules", (target, locales, options) => {
    const { locale, language } = resolve(locales);
    state.set(target, { locale, language, type: option(options, "type", ["cardinal", "ordinal"], "cardinal") });
  }, {
    select(value) {
      const self = internal(this, "PluralRules");
      return pluralCategory(self.language, self.type, value);
    },
    selectRange(start, end) {
      const self = internal(this, "PluralRules");
      return pluralCategory(self.language, self.type, end);
    },
    resolvedOptions() {
      const self = internal(this, "PluralRules");
      const categories = self.type === "ordinal" && self.language === "en"
        ? ["few", "one", "two", "other"] : ["one", "other"];
      return { locale: self.locale, type: self.type, pluralCategories: categories,
        minimumIntegerDigits: 1, minimumFractionDigits: 0, maximumFractionDigits: 3 };
    },
  });

  // ---- Nombres ------------------------------------------------------------------------------------------------
  function numberParts(self, value) {
    const { data } = self;
    let number = typeof value === "bigint" ? Number(value) : Number(value);
    const parts = [];
    const push = (type, text) => { if (text !== "") parts.push({ type, value: text }); };
    if (Number.isNaN(number)) return [{ type: "nan", value: "NaN" }];
    const negative = number < 0 || Object.is(number, -0);
    number = Math.abs(number);
    if (self.style === "percent") number *= 100;
    let suffix = "";
    if (self.notation === "compact" && Number.isFinite(number)) {
      const scale = data.compact.filter(([, size]) => number >= size).pop() ?? data.compact[0];
      number /= scale[1];
      suffix = scale[0];
    }
    let digits;
    if (!Number.isFinite(number)) {
      digits = "∞";
    } else if (self.minimumSignificantDigits !== undefined) {
      digits = Number(number.toPrecision(self.maximumSignificantDigits)).toString();
      if (/e/.test(digits)) digits = number.toFixed(0);
      const count = digits.replace(/^0\.0*|\./g, "").length;
      if (count < self.minimumSignificantDigits) {
        digits += (digits.includes(".") ? "" : ".") + "0".repeat(self.minimumSignificantDigits - count);
      }
    } else {
      let maximum = self.maximumFractionDigits;
      if (self.notation === "compact" && self.compactDefault) maximum = number < 100 && suffix !== "" ? 1 : 0;
      digits = number.toFixed(Math.min(maximum, 100));
      if (digits.includes(".")) {
        let [whole, fraction] = digits.split(".");
        const minimum = self.notation === "compact" && self.compactDefault ? 0 : self.minimumFractionDigits;
        while (fraction.length > minimum && fraction.endsWith("0")) fraction = fraction.slice(0, -1);
        digits = fraction === "" ? whole : whole + "." + fraction;
      }
    }
    let [whole, fraction] = digits.split(".");
    whole = whole.padStart(self.minimumIntegerDigits, "0");
    const zero = Number(digits) === 0;
    const sign = self.signDisplay === "never" ? ""
      : negative && !(zero && (self.signDisplay === "exceptZero" || self.signDisplay === "auto" || self.signDisplay === "negative"))
        ? data.minus
        : (self.signDisplay === "always" || (self.signDisplay === "exceptZero" && !zero)) && !negative ? "+" : "";
    const symbol = self.style !== "currency" ? ""
      : self.currencyDisplay === "code" || self.currencyDisplay === "name" ? self.currency
        : SYMBOLS[self.currency] ?? self.currency;
    const before = self.style === "currency" && self.language === "en";
    push(sign === "+" ? "plusSign" : "minusSign", sign);
    if (before) {
      push("currency", symbol);
      if (symbol.length > 1 && /[A-Za-z]$/.test(symbol)) push("literal", NBSP);
    }
    const grouped = self.useGrouping && whole.length > 3 && digits !== "∞";
    if (grouped) {
      const groups = [];
      for (let end = whole.length; end > 0; end -= 3) groups.unshift(whole.slice(Math.max(0, end - 3), end));
      groups.forEach((group, index) => {
        if (index > 0) push("group", data.group);
        push("integer", group);
      });
    } else {
      push(digits === "∞" ? "infinity" : "integer", whole);
    }
    if (fraction !== undefined) {
      push("decimal", data.decimal);
      push("fraction", fraction);
    }
    push("compact", suffix);
    if (self.style === "percent") {
      const text = data.percent;
      if (text.length > 1) push("literal", text.slice(0, -1));
      push("percentSign", "%");
    }
    if (self.style === "currency" && !before) {
      push("literal", NBSP);
      push("currency", symbol);
    }
    if (self.style === "unit") {
      push("literal", NBSP);
      push("unit", self.unit);
    }
    return parts;
  }
  const NumberFormat = service("NumberFormat", (target, locales, options) => {
    const resolved = resolve(locales);
    const style = option(options, "style", ["decimal", "percent", "currency", "unit"], "decimal");
    const currency = option(options, "currency", undefined, undefined)?.toUpperCase();
    if (style === "currency" && !/^[A-Z]{3}$/.test(currency ?? "")) {
      throw new TypeError("Intl.NumberFormat : une devise est attendue");
    }
    const unit = option(options, "unit", undefined, undefined);
    if (style === "unit" && unit === undefined) throw new TypeError("Intl.NumberFormat : une unité est attendue");
    const cents = style === "currency" ? (NO_CENTS.has(currency) ? 0 : 2) : 0;
    const minimumFractionDigits = numberOption(options, "minimumFractionDigits", 0, 100, undefined);
    const fallbackMaximum = style === "currency" ? cents : style === "percent" ? 0 : 3;
    const maximumFractionDigits = numberOption(options, "maximumFractionDigits", 0, 100, undefined);
    const minimum = minimumFractionDigits ?? Math.min(cents, maximumFractionDigits ?? cents);
    const maximum = maximumFractionDigits ?? Math.max(fallbackMaximum, minimum);
    if (minimum > maximum) throw new RangeError("Intl.NumberFormat : décimales minimales au-delà des maximales");
    const minimumSignificantDigits = numberOption(options, "minimumSignificantDigits", 1, 21, undefined);
    const maximumSignificantDigits = numberOption(options, "maximumSignificantDigits", 1, 21, undefined);
    const significant = minimumSignificantDigits !== undefined || maximumSignificantDigits !== undefined;
    const grouping = options?.useGrouping;
    state.set(target, {
      ...resolved, style, currency, unit,
      currencyDisplay: option(options, "currencyDisplay", ["symbol", "narrowSymbol", "code", "name"], "symbol"),
      notation: option(options, "notation", ["standard", "scientific", "engineering", "compact"], "standard"),
      compactDefault: minimumFractionDigits === undefined && maximumFractionDigits === undefined && !significant,
      signDisplay: option(options, "signDisplay", ["auto", "never", "always", "exceptZero", "negative"], "auto"),
      minimumIntegerDigits: numberOption(options, "minimumIntegerDigits", 1, 21, 1),
      minimumFractionDigits: minimum, maximumFractionDigits: maximum,
      minimumSignificantDigits: significant ? minimumSignificantDigits ?? 1 : undefined,
      maximumSignificantDigits: significant ? maximumSignificantDigits ?? 21 : undefined,
      useGrouping: grouping === undefined ? true : grouping !== false && grouping !== "false",
    });
  }, {
    formatToParts(value) {
      return numberParts(internal(this, "NumberFormat"), value);
    },
    formatRange(start, end) {
      const self = internal(this, "NumberFormat");
      const join = (value) => numberParts(self, value).map((part) => part.value).join("");
      return join(start) + "–" + join(end);
    },
    resolvedOptions() {
      const self = internal(this, "NumberFormat");
      const out = { locale: self.locale, numberingSystem: "latn", style: self.style };
      if (self.style === "currency") Object.assign(out, { currency: self.currency, currencyDisplay: self.currencyDisplay, currencySign: "standard" });
      if (self.style === "unit") Object.assign(out, { unit: self.unit, unitDisplay: "short" });
      Object.assign(out, { minimumIntegerDigits: self.minimumIntegerDigits });
      if (self.minimumSignificantDigits !== undefined) {
        Object.assign(out, { minimumSignificantDigits: self.minimumSignificantDigits, maximumSignificantDigits: self.maximumSignificantDigits });
      } else {
        Object.assign(out, { minimumFractionDigits: self.minimumFractionDigits, maximumFractionDigits: self.maximumFractionDigits });
      }
      return Object.assign(out, { useGrouping: self.useGrouping ? "auto" : false, notation: self.notation,
        signDisplay: self.signDisplay, roundingMode: "halfExpand", roundingIncrement: 1,
        roundingPriority: "auto", trailingZeroDisplay: "auto" });
    },
  });
  bound(NumberFormat, "NumberFormat", "format",
    (self, value) => numberParts(self, value).map((part) => part.value).join(""));

  // ---- Dates ------------------------------------------------------------------------------------------------
  const STYLES = {
    full: { weekday: "long", day: "numeric", month: "long", year: "numeric" },
    long: { day: "numeric", month: "long", year: "numeric" },
    medium: { day: "numeric", month: "short", year: "numeric" },
    short: { day: "2-digit", month: "2-digit", year: "numeric" },
  };
  const TIMES = {
    full: { hour: "numeric", minute: "2-digit", second: "2-digit", timeZoneName: "short" },
    long: { hour: "numeric", minute: "2-digit", second: "2-digit", timeZoneName: "short" },
    medium: { hour: "numeric", minute: "2-digit", second: "2-digit" },
    short: { hour: "numeric", minute: "2-digit" },
  };
  // Nom du fuseau du système : déduit de son décalage, faute de mieux.
  function systemZone(region) {
    const offset = -new Date().getTimezoneOffset();
    if (offset === 0) return "UTC";
    const winter = -new Date(new Date().getFullYear(), 0, 1).getTimezoneOffset();
    const summer = -new Date(new Date().getFullYear(), 6, 1).getTimezoneOffset();
    if (winter === 60 && summer === 120) return "Europe/Paris";
    if (winter === 0 && summer === 60) return "Europe/London";
    const hours = offset / 60;
    return Number.isInteger(hours) ? "Etc/GMT" + (hours > 0 ? "-" : "+") + Math.abs(hours) : "UTC";
  }
  function dateParts(self, value) {
    const date = value === undefined ? new Date() : value instanceof Date ? value : new Date(value);
    if (Number.isNaN(date.getTime())) throw new RangeError("Intl.DateTimeFormat : date invalide");
    const { data, fields, utc } = self;
    const get = (name) => (utc ? date["getUTC" + name]() : date["get" + name]());
    const two = (number) => String(number).padStart(2, "0");
    const parts = [];
    const push = (type, text) => parts.push({ type, value: String(text) });
    const literal = (text) => {
      if (parts.length > 0 && text !== "") push("literal", text);
    };
    const textual = fields.month !== undefined && fields.month !== "numeric" && fields.month !== "2-digit";
    const month = () => {
      const index = get("Month");
      switch (fields.month) {
        case "long": return data.months[index];
        case "short": return data.monthsShort[index];
        case "narrow": return data.months[index][0].toUpperCase();
        case "2-digit": return two(index + 1);
        default: return index + 1;
      }
    };
    const year = () => (fields.year === "2-digit" ? two(get("FullYear") % 100) : get("FullYear"));
    const day = () => (fields.day === "2-digit" ? two(get("Date")) : get("Date"));
    const american = self.language === "en" && (self.region === undefined || self.region === "US");
    const hasDate = fields.day !== undefined || fields.month !== undefined || fields.year !== undefined;
    if (fields.weekday !== undefined) {
      const index = get("Day");
      push("weekday", fields.weekday === "long" ? data.days[index]
        : fields.weekday === "short" ? data.daysShort[index] : data.days[index][0].toUpperCase());
    }
    const order = american ? ["month", "day", "year"] : ["day", "month", "year"];
    const present = order.filter((name) => fields[name] !== undefined);
    present.forEach((name, index) => {
      if (index === 0) literal(self.language === "en" ? ", " : " ");
      else if (!textual) push("literal", "/");
      else if (american && name === "year" && fields.day !== undefined) push("literal", ", ");
      else push("literal", " ");
      push(name, name === "day" ? day() : name === "month" ? month() : year());
    });
    if (fields.hour !== undefined || fields.minute !== undefined || fields.second !== undefined) {
      if (parts.length > 0) {
        push("literal", !hasDate ? " " : textual ? data.at : self.language === "en" ? ", " : " ");
      }
      const twelve = self.hour12;
      const hours = get("Hours");
      let first = true;
      const separate = () => {
        if (!first) push("literal", ":");
        first = false;
      };
      if (fields.hour !== undefined) {
        separate();
        const shown = twelve ? hours % 12 || 12 : hours;
        push("hour", fields.hour === "2-digit" || (!twelve && self.language === "fr") ? two(shown) : shown);
      }
      if (fields.minute !== undefined) {
        separate();
        push("minute", fields.hour !== undefined || fields.minute === "2-digit" ? two(get("Minutes")) : get("Minutes"));
      }
      if (fields.second !== undefined) {
        separate();
        push("second", fields.minute !== undefined || fields.second === "2-digit" ? two(get("Seconds")) : get("Seconds"));
        if (self.fraction > 0) {
          push("literal", data.decimal);
          push("fractionalSecond", String(get("Milliseconds")).padStart(3, "0").slice(0, self.fraction));
        }
      }
      if (twelve && fields.hour !== undefined) {
        push("literal", NNBSP);
        push("dayPeriod", data.dayPeriods[hours < 12 ? 0 : 1]);
      }
    }
    if (fields.timeZoneName !== undefined) {
      literal(" ");
      const offset = utc ? 0 : -date.getTimezoneOffset();
      const sign = offset < 0 ? "−" : "+";
      const [hours, minutes] = [Math.floor(Math.abs(offset) / 60), Math.abs(offset) % 60];
      push("timeZoneName", offset === 0 ? "UTC" : "UTC" + sign + hours + (minutes ? ":" + two(minutes) : ""));
    }
    return parts;
  }
  const FIELDS = {
    weekday: ["narrow", "short", "long"], year: ["2-digit", "numeric"],
    month: ["2-digit", "numeric", "narrow", "short", "long"], day: ["2-digit", "numeric"],
    hour: ["2-digit", "numeric"], minute: ["2-digit", "numeric"], second: ["2-digit", "numeric"],
    timeZoneName: ["short", "long", "shortOffset", "longOffset", "shortGeneric", "longGeneric"],
  };
  function dateFormat(target, locales, options, defaults) {
    const resolved = resolve(locales);
    const dateStyle = option(options, "dateStyle", ["full", "long", "medium", "short"], undefined);
    const timeStyle = option(options, "timeStyle", ["full", "long", "medium", "short"], undefined);
    let fields = {};
    for (const [name, allowed] of Object.entries(FIELDS)) {
      const value = option(options, name, allowed, undefined);
      if (value !== undefined) fields[name] = value;
    }
    const fraction = numberOption(options, "fractionalSecondDigits", 1, 3, 0);
    if (dateStyle !== undefined || timeStyle !== undefined) {
      if (Object.keys(fields).length > 0) {
        throw new TypeError("Intl.DateTimeFormat : dateStyle et timeStyle excluent les champs séparés");
      }
      fields = { ...(dateStyle ? STYLES[dateStyle] : {}), ...(timeStyle ? TIMES[timeStyle] : {}) };
      if (dateStyle === "short" && resolved.language === "en") {
        fields = { ...fields, day: "numeric", month: "numeric", year: "2-digit" };
      }
    } else if (!Object.keys(fields).some((name) => name !== "timeZoneName") && fraction === 0) {
      fields = { ...defaults, ...fields };
    }
    const zone = options?.timeZone === undefined ? undefined : String(options.timeZone);
    const utc = zone !== undefined && /^(utc|gmt|etc\/utc|etc\/gmt|z)$/i.test(zone);
    const cycle = option(options, "hourCycle", ["h11", "h12", "h23", "h24"], undefined);
    const american = resolved.language === "en" && (resolved.region === undefined || resolved.region === "US");
    const hour12 = options?.hour12 !== undefined ? Boolean(options.hour12)
      : cycle !== undefined ? cycle === "h11" || cycle === "h12" : american;
    state.set(target, {
      ...resolved, fields, utc, hour12, fraction, dateStyle, timeStyle,
      timeZone: utc ? "UTC" : zone ?? systemZone(resolved.region),
    });
  }
  const DATE_DEFAULTS = { year: "numeric", month: "numeric", day: "numeric" };
  const TIME_DEFAULTS = { hour: "numeric", minute: "2-digit", second: "2-digit" };
  const joinParts = (parts) => parts.map((part) => part.value).join("");
  const DateTimeFormat = service("DateTimeFormat",
    (target, locales, options) => dateFormat(target, locales, options, DATE_DEFAULTS), {
      formatToParts(date) {
        return dateParts(internal(this, "DateTimeFormat"), date);
      },
      formatRange(start, end) {
        const self = internal(this, "DateTimeFormat");
        return joinParts(dateParts(self, start)) + " – " + joinParts(dateParts(self, end));
      },
      resolvedOptions() {
        const self = internal(this, "DateTimeFormat");
        const out = { locale: self.locale, calendar: "gregory", numberingSystem: "latn", timeZone: self.timeZone };
        if (self.fields.hour !== undefined) Object.assign(out, { hourCycle: self.hour12 ? "h12" : "h23", hour12: self.hour12 });
        if (self.dateStyle !== undefined) out.dateStyle = self.dateStyle;
        if (self.timeStyle !== undefined) out.timeStyle = self.timeStyle;
        if (self.dateStyle === undefined && self.timeStyle === undefined) Object.assign(out, self.fields);
        return out;
      },
    });
  bound(DateTimeFormat, "DateTimeFormat", "format", (self, date) => joinParts(dateParts(self, date)));

  // ---- Durées relatives, listes ------------------------------------------------------------------------------
  const RelativeTimeFormat = service("RelativeTimeFormat", (target, locales, options) => {
    state.set(target, {
      ...resolve(locales),
      numeric: option(options, "numeric", ["always", "auto"], "always"),
      style: option(options, "style", ["long", "short", "narrow"], "long"),
    });
  }, {
    format(value, unit) {
      return joinParts(this.formatToParts(value, unit));
    },
    formatToParts(value, unit) {
      const self = internal(this, "RelativeTimeFormat");
      const name = String(unit).replace(/s$/, "");
      const names = self.data.units[name];
      if (names === undefined) throw new RangeError("Intl.RelativeTimeFormat : unité inconnue : " + unit);
      const number = Number(value);
      if (!Number.isFinite(number)) throw new RangeError("Intl.RelativeTimeFormat : nombre fini attendu");
      const named = self.numeric === "auto" ? self.data.named[name]?.[String(number)] : undefined;
      if (named !== undefined) return [{ type: "literal", value: named }];
      const amount = new NumberFormat(self.locale).format(Math.abs(number));
      const plural = pluralCategory(self.language, "cardinal", Math.abs(number)) === "one" ? names[0] : names[1];
      const text = self.data.relative("\u0001 " + plural, number < 0 || Object.is(number, -0));
      const [before, after] = text.split("\u0001");
      const parts = [];
      if (before !== "") parts.push({ type: "literal", value: before });
      parts.push({ type: "integer", value: amount, unit: name });
      if (after !== "") parts.push({ type: "literal", value: after });
      return parts;
    },
    resolvedOptions() {
      const self = internal(this, "RelativeTimeFormat");
      return { locale: self.locale, style: self.style, numeric: self.numeric, numberingSystem: "latn" };
    },
  });
  const ListFormat = service("ListFormat", (target, locales, options) => {
    state.set(target, {
      ...resolve(locales),
      type: option(options, "type", ["conjunction", "disjunction", "unit"], "conjunction"),
      style: option(options, "style", ["long", "short", "narrow"], "long"),
    });
  }, {
    format(list) {
      return joinParts(this.formatToParts(list));
    },
    formatToParts(list) {
      const self = internal(this, "ListFormat");
      const items = Array.from(list, (item) => {
        if (typeof item !== "string") throw new TypeError("Intl.ListFormat : des chaînes sont attendues");
        return item;
      });
      const last = self.type === "unit" ? (self.style === "narrow" ? " " : ", ")
        : self.type === "disjunction" ? self.data.or : self.data.and;
      // L'anglais met une virgule avant « and » à partir de trois éléments.
      const serial = self.language === "en" && items.length > 2 && self.type !== "unit" ? "," : "";
      const parts = [];
      items.forEach((item, index) => {
        if (index > 0) {
          parts.push({ type: "literal", value: index === items.length - 1 ? serial + last : ", " });
        }
        parts.push({ type: "element", value: item });
      });
      return parts;
    },
    resolvedOptions() {
      const self = internal(this, "ListFormat");
      return { locale: self.locale, type: self.type, style: self.style };
    },
  });

  // ---- Comparaison, découpage, noms ------------------------------------------------------------------------
  const strip = (text) => text.normalize("NFD").replace(/\p{M}+/gu, "");
  function compare(self, left, right) {
    const [a, b] = [String(left), String(right)];
    const order = (x, y) => (x < y ? -1 : x > y ? 1 : 0);
    if (self.numeric) {
      const split = (text) => text.split(/(\d+)/).filter((piece) => piece !== "");
      const [as, bs] = [split(a), split(b)];
      for (let i = 0; i < Math.min(as.length, bs.length); i++) {
        const both = /^\d/.test(as[i]) && /^\d/.test(bs[i]);
        const result = both ? Math.sign(Number(as[i]) - Number(bs[i]))
          : compare({ ...self, numeric: false }, as[i], bs[i]);
        if (result !== 0) return result;
      }
      return Math.sign(as.length - bs.length);
    }
    const base = order(strip(a).toLowerCase(), strip(b).toLowerCase());
    if (base !== 0 || self.sensitivity === "base") return base;
    const accents = order(a.normalize("NFD").toLowerCase(), b.normalize("NFD").toLowerCase());
    if (accents !== 0 && self.sensitivity !== "case") return accents;
    if (self.sensitivity === "accent") return 0;
    // À lettres égales, la minuscule passe avant la majuscule.
    return -order(a, b);
  }
  const Collator = service("Collator", (target, locales, options) => {
    state.set(target, {
      ...resolve(locales),
      usage: option(options, "usage", ["sort", "search"], "sort"),
      sensitivity: option(options, "sensitivity", ["base", "accent", "case", "variant"], "variant"),
      numeric: Boolean(options?.numeric),
      ignorePunctuation: Boolean(options?.ignorePunctuation),
    });
  }, {
    resolvedOptions() {
      const self = internal(this, "Collator");
      return { locale: self.locale, usage: self.usage, sensitivity: self.sensitivity,
        ignorePunctuation: self.ignorePunctuation, collation: "default", numeric: self.numeric,
        caseFirst: "false" };
    },
  });
  bound(Collator, "Collator", "compare", compare);

  const Segmenter = service("Segmenter", (target, locales, options) => {
    state.set(target, {
      ...resolve(locales),
      granularity: option(options, "granularity", ["grapheme", "word", "sentence"], "grapheme"),
    });
  }, {
    segment(input) {
      const self = internal(this, "Segmenter");
      const text = String(input);
      const pattern = self.granularity === "word" ? /[\p{L}\p{N}_]+(?:['’][\p{L}\p{N}_]+)*|\s+|[^\s\p{L}\p{N}_]/gu
        : self.granularity === "sentence" ? /[^.!?…]+(?:[.!?…]+\s*|$)|[.!?…]+\s*/gu
          : /\P{M}\p{M}*(?:‍\P{M}\p{M}*)*/gu;
      const pieces = [];
      for (const match of text.matchAll(pattern)) {
        const piece = { segment: match[0], index: match.index, input: text };
        if (self.granularity === "word") piece.isWordLike = /[\p{L}\p{N}]/u.test(match[0]);
        pieces.push(piece);
      }
      return {
        containing(index = 0) {
          const at = Math.trunc(Number(index)) || 0;
          return pieces.find((piece) => at >= piece.index && at < piece.index + piece.segment.length);
        },
        [Symbol.iterator]() {
          return pieces[Symbol.iterator]();
        },
      };
    },
    resolvedOptions() {
      const self = internal(this, "Segmenter");
      return { locale: self.locale, granularity: self.granularity };
    },
  });

  const DisplayNames = service("DisplayNames", (target, locales, options) => {
    const type = option(options, "type",
      ["language", "region", "script", "currency", "calendar", "dateTimeField"], undefined);
    if (type === undefined) throw new TypeError("Intl.DisplayNames : l'option type est requise");
    state.set(target, {
      ...resolve(locales), type,
      style: option(options, "style", ["narrow", "short", "long"], "long"),
      fallback: option(options, "fallback", ["code", "none"], "code"),
    });
  }, {
    of(code) {
      const self = internal(this, "DisplayNames");
      const text = String(code);
      const table = self.type === "language" ? self.data.languages
        : self.type === "region" ? self.data.regions
          : self.type === "currency" ? self.data.currencies : {};
      const key = self.type === "language" ? languageOf(canonical(text)) : text.toUpperCase();
      return table[key] ?? (self.fallback === "code" ? text : undefined);
    },
    resolvedOptions() {
      const self = internal(this, "DisplayNames");
      return { locale: self.locale, style: self.style, type: self.type, fallback: self.fallback };
    },
  });

  class Locale {
    #tag;
    constructor(tag, options) {
      let text = canonical(tag);
      const parts = text.split("-");
      if (options?.language !== undefined) parts[0] = String(options.language).toLowerCase();
      if (options?.region !== undefined) {
        const index = parts.findIndex((part, at) => at > 0 && /^[A-Z]{2}$|^\d{3}$/.test(part));
        if (index > 0) parts[index] = String(options.region).toUpperCase();
        else parts.push(String(options.region).toUpperCase());
      }
      text = parts.join("-");
      this.#tag = canonical(text);
    }
    get baseName() { return this.#tag.replace(/-[a-z0-9](-[a-z0-9]{2,8})+$/, ""); }
    get language() { return languageOf(this.#tag); }
    get region() { return regionOf(this.#tag); }
    get script() { return this.#tag.split("-").slice(1).find((part) => /^[A-Z][a-z]{3}$/.test(part)); }
    get calendar() { return undefined; }
    get numberingSystem() { return undefined; }
    get hourCycle() { return undefined; }
    maximize() {
      const regions = { fr: "FR", en: "US", de: "DE", es: "ES", it: "IT" };
      const region = this.region ?? regions[this.language];
      return new Locale([this.language, this.script ?? "Latn", region].filter(Boolean).join("-"));
    }
    minimize() { return new Locale(this.language); }
    toString() { return this.#tag; }
  }
  define(Locale.prototype, Symbol.toStringTag, { value: "Intl.Locale", configurable: true });

  const Intl = {
    getCanonicalLocales: (locales) => requested(locales),
    supportedValuesOf(key) {
      switch (String(key)) {
        case "calendar": return ["gregory"];
        case "collation": return ["default"];
        case "currency": return Object.keys(SYMBOLS).sort();
        case "numberingSystem": return ["latn"];
        case "timeZone": return ["Europe/London", "Europe/Paris", "UTC"];
        case "unit": return ["day", "hour", "minute", "month", "second", "week", "year"];
        default: throw new RangeError("Intl.supportedValuesOf : clé inconnue : " + key);
      }
    },
  };
  for (const [name, value] of Object.entries({
    Collator, DateTimeFormat, DisplayNames, ListFormat, Locale, NumberFormat, PluralRules,
    RelativeTimeFormat, Segmenter,
  })) {
    define(Intl, name, { value, writable: true, enumerable: false, configurable: true });
  }
  define(Intl, Symbol.toStringTag, { value: "Intl", configurable: true });
  define(global, "Intl", { value: Intl, writable: true, enumerable: false, configurable: true });

  // ---- Méthodes « locales » du langage ------------------------------------------------------------------------
  const method = (target, name, value) => define(target, name, { value, writable: true, enumerable: false, configurable: true });
  const localized = (defaults) => function (locales, options) {
    if (!(this instanceof Date)) throw new TypeError("un objet Date est attendu");
    if (Number.isNaN(this.getTime())) return "Invalid Date";
    const target = Object.create(DateTimeFormat.prototype);
    dateFormat(target, locales, options === undefined ? undefined : Object(options), defaults);
    return joinParts(dateParts(internal(target, "DateTimeFormat"), this));
  };
  method(Date.prototype, "toLocaleDateString", localized(DATE_DEFAULTS));
  method(Date.prototype, "toLocaleTimeString", localized(TIME_DEFAULTS));
  method(Date.prototype, "toLocaleString", localized({ ...DATE_DEFAULTS, ...TIME_DEFAULTS }));
  method(Number.prototype, "toLocaleString", function toLocaleString(locales, options) {
    return new NumberFormat(locales, options).format(Number(this));
  });
  method(BigInt.prototype, "toLocaleString", function toLocaleString(locales, options) {
    return new NumberFormat(locales, options).format(Number(this));
  });
  method(String.prototype, "localeCompare", function localeCompare(that, locales, options) {
    return new Collator(locales, options).compare(String(this), String(that));
  });
})();
