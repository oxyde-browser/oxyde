// Temporal, première partie : conversions, options, arrondis, arithmétique
// du calendrier ISO et lecture des chaînes ISO 8601.
//
// Les quatre fichiers de ce dossier forment une seule fonction : celui-ci
// l'ouvre, le dernier la ferme et rend `{ Temporal, natives }`.
(function (clockNow) {
  'use strict';

  const natives = [];
  const MathFloor = Math.floor;
  const MathTrunc = Math.trunc;
  const MathAbs = Math.abs;
  const ObjectCreate = Object.create;
  const ObjectDefineProperty = Object.defineProperty;
  const ObjectGetOwnPropertyNames = Object.getOwnPropertyNames;
  const ObjectGetOwnPropertyDescriptor = Object.getOwnPropertyDescriptor;
  const ReflectApply = Reflect.apply;
  const SymbolToStringTag = Symbol.toStringTag;
  const SymbolToPrimitive = Symbol.toPrimitive;
  const ToBigInt = BigInt;
  const ToNumber = Number;
  const RangeErr = RangeError;
  const TypeErr = TypeError;

  const REQUIRED = { __proto__: null };
  const NS_DAY = 86400000000000;
  const NS_DAY_BIG = 86400000000000n;
  const NS_MAX_INSTANT = 8640000000000000000000n;
  const MAX_TIME_DURATION = 9007199254740991999999999n;
  const TWO_128 = 2 ** 128;

  const UNITS = ['year', 'month', 'week', 'day', 'hour', 'minute', 'second', 'millisecond', 'microsecond', 'nanosecond'];
  const UNIT_INDEX = {
    __proto__: null, year: 0, month: 1, week: 2, day: 3, hour: 4, minute: 5, second: 6, millisecond: 7, microsecond: 8, nanosecond: 9,
  };
  const PLURAL_UNITS = {
    __proto__: null, years: 'year', months: 'month', weeks: 'week', days: 'day', hours: 'hour', minutes: 'minute', seconds: 'second',
    milliseconds: 'millisecond', microseconds: 'microsecond', nanoseconds: 'nanosecond',
  };
  const UNIT_NS = {
    __proto__: null, day: NS_DAY, hour: 3600000000000, minute: 60000000000, second: 1000000000, millisecond: 1000000, microsecond: 1000, nanosecond: 1,
  };
  const ROUNDING_MODES = ['ceil', 'floor', 'expand', 'trunc', 'halfCeil', 'halfFloor', 'halfExpand', 'halfTrunc', 'halfEven'];

  function rangeError(message) {
    return new RangeErr(message);
  }
  function typeError(message) {
    return new TypeErr(message);
  }
  // Sans passer par `Number.isFinite`, qu'un script peut avoir remplacé.
  function NumberIsFinite(value) {
    return typeof value === 'number' && value - value === 0;
  }
  function NumberIsInteger(value) {
    return NumberIsFinite(value) && MathTrunc(value) === value;
  }
  function isObject(value) {
    return (typeof value === 'object' && value !== null) || typeof value === 'function';
  }
  function inList(list, value) {
    for (let i = 0; i < list.length; i++) {
      if (list[i] === value) return true;
    }
    return false;
  }
  function pad(value, width) {
    let text = `${value}`;
    while (text.length < width) text = '0' + text;
    return text;
  }

  // ---- Conversions ----

  function toIntegerWithTruncation(value) {
    const number = +value;
    if (!NumberIsFinite(number)) throw rangeError('nombre fini attendu');
    return MathTrunc(number) + 0;
  }
  function toPositiveIntegerWithTruncation(value) {
    const integer = toIntegerWithTruncation(value);
    if (integer <= 0) throw rangeError('entier strictement positif attendu');
    return integer;
  }
  function toIntegerIfIntegral(value) {
    const number = +value;
    if (!NumberIsInteger(number)) throw rangeError('entier attendu');
    return number + 0;
  }
  function toPrimitiveString(value) {
    if (!isObject(value)) return value;
    const exotic = value[SymbolToPrimitive];
    if (exotic !== undefined && exotic !== null) {
      if (typeof exotic !== 'function') throw typeError('@@toPrimitive doit être une fonction');
      const result = ReflectApply(exotic, value, ['string']);
      if (isObject(result)) throw typeError('conversion en primitive impossible');
      return result;
    }
    const names = ['toString', 'valueOf'];
    for (let i = 0; i < 2; i++) {
      const method = value[names[i]];
      if (typeof method === 'function') {
        const result = ReflectApply(method, value, []);
        if (!isObject(result)) return result;
      }
    }
    throw typeError('conversion en primitive impossible');
  }

  // ---- Options ----

  function getOptionsObject(options) {
    if (options === undefined) return ObjectCreate(null);
    if (isObject(options)) return options;
    throw typeError('les options doivent être un objet');
  }
  function getOption(options, name, values, fallback) {
    const value = options[name];
    if (value === undefined) {
      if (fallback === REQUIRED) throw rangeError(`option ${name} requise`);
      return fallback;
    }
    const text = `${value}`;
    if (!inList(values, text)) throw rangeError(`valeur invalide pour ${name} : ${text}`);
    return text;
  }
  function getRoundingModeOption(options, fallback) {
    return getOption(options, 'roundingMode', ROUNDING_MODES, fallback);
  }
  function getRoundingIncrementOption(options) {
    const value = options.roundingIncrement;
    if (value === undefined) return 1;
    const increment = toIntegerWithTruncation(value);
    if (increment < 1 || increment > 1000000000) throw rangeError('roundingIncrement hors limites');
    return increment;
  }
  function getOverflowOption(options) {
    return getOption(options, 'overflow', ['constrain', 'reject'], 'constrain');
  }
  function getDisambiguationOption(options) {
    return getOption(options, 'disambiguation', ['compatible', 'earlier', 'later', 'reject'], 'compatible');
  }
  function getOffsetOption(options, fallback) {
    return getOption(options, 'offset', ['prefer', 'use', 'ignore', 'reject'], fallback);
  }
  function getShowCalendarOption(options) {
    return getOption(options, 'calendarName', ['auto', 'always', 'never', 'critical'], 'auto');
  }
  function negateRoundingMode(mode) {
    switch (mode) {
      case 'ceil': return 'floor';
      case 'floor': return 'ceil';
      case 'halfCeil': return 'halfFloor';
      case 'halfFloor': return 'halfCeil';
      default: return mode;
    }
  }
  function getFractionalSecondDigitsOption(options) {
    const value = options.fractionalSecondDigits;
    if (value === undefined) return 'auto';
    if (typeof value !== 'number') {
      if (`${value}` !== 'auto') throw rangeError('fractionalSecondDigits invalide');
      return 'auto';
    }
    if (!NumberIsFinite(value)) throw rangeError('fractionalSecondDigits invalide');
    const count = MathFloor(value);
    if (count < 0 || count > 9) throw rangeError('fractionalSecondDigits hors limites');
    return count;
  }
  function toSecondsStringPrecision(smallestUnit, digits) {
    switch (smallestUnit) {
      case 'minute': return { precision: 'minute', unit: 'minute', increment: 1 };
      case 'second': return { precision: 0, unit: 'second', increment: 1 };
      case 'millisecond': return { precision: 3, unit: 'millisecond', increment: 1 };
      case 'microsecond': return { precision: 6, unit: 'microsecond', increment: 1 };
      case 'nanosecond': return { precision: 9, unit: 'nanosecond', increment: 1 };
      default: break;
    }
    if (digits === 'auto') return { precision: 'auto', unit: 'nanosecond', increment: 1 };
    if (digits === 0) return { precision: 0, unit: 'second', increment: 1 };
    if (digits <= 3) return { precision: digits, unit: 'millisecond', increment: 10 ** (3 - digits) };
    if (digits <= 6) return { precision: digits, unit: 'microsecond', increment: 10 ** (6 - digits) };
    return { precision: digits, unit: 'nanosecond', increment: 10 ** (9 - digits) };
  }
  // Lit une unité : `undefined` si elle est absente et facultative, `'auto'`
  // tel quel (la validation tranche), sinon le nom au singulier.
  function getUnitOption(options, key, fallback) {
    const value = options[key];
    if (value === undefined) {
      if (fallback === REQUIRED) throw rangeError(`option ${key} requise`);
      return undefined;
    }
    const text = `${value}`;
    if (text === 'auto') return 'auto';
    if (text in UNIT_INDEX) return text;
    if (text in PLURAL_UNITS) return PLURAL_UNITS[text];
    throw rangeError(`unité inconnue : ${text}`);
  }
  // `group` : 'date', 'time' ou 'datetime' ; `extra` : 'auto' ou 'day'.
  function validateUnit(unit, group, extra) {
    if (unit === undefined || unit === extra) return;
    if (unit === 'auto') throw rangeError('unité « auto » refusée ici');
    const isDate = UNIT_INDEX[unit] <= 3;
    if (isDate ? group === 'time' : group === 'date') throw rangeError(`unité refusée ici : ${unit}`);
  }
  function largerUnit(one, two) {
    return UNIT_INDEX[one] <= UNIT_INDEX[two] ? one : two;
  }
  function isCalendarUnit(unit) {
    return UNIT_INDEX[unit] <= 2;
  }
  function isDateUnit(unit) {
    return UNIT_INDEX[unit] <= 3;
  }
  function maximumRoundingIncrement(unit) {
    switch (unit) {
      case 'hour': return 24;
      case 'minute':
      case 'second': return 60;
      case 'millisecond':
      case 'microsecond':
      case 'nanosecond': return 1000;
      default: return undefined;
    }
  }
  function validateRoundingIncrement(increment, dividend, inclusive) {
    const maximum = inclusive ? dividend : dividend > 1 ? dividend - 1 : 1;
    if (increment > maximum) throw rangeError('roundingIncrement trop grand');
    if (dividend % increment !== 0) throw rangeError('roundingIncrement ne divise pas l’unité supérieure');
  }
  function getDifferenceSettings(since, options, group, disallowed, fallbackSmallest, smallestLargestDefault) {
    let largestUnit = getUnitOption(options, 'largestUnit', undefined);
    const roundingIncrement = getRoundingIncrementOption(options);
    let roundingMode = getRoundingModeOption(options, 'trunc');
    let smallestUnit = getUnitOption(options, 'smallestUnit', undefined);
    validateUnit(largestUnit, group, 'auto');
    if (largestUnit === undefined) largestUnit = 'auto';
    if (inList(disallowed, largestUnit)) throw rangeError(`largestUnit refusée : ${largestUnit}`);
    if (since) roundingMode = negateRoundingMode(roundingMode);
    validateUnit(smallestUnit, group, undefined);
    if (smallestUnit === undefined) smallestUnit = fallbackSmallest;
    if (inList(disallowed, smallestUnit)) throw rangeError(`smallestUnit refusée : ${smallestUnit}`);
    const defaultLargest = largerUnit(smallestLargestDefault, smallestUnit);
    if (largestUnit === 'auto') largestUnit = defaultLargest;
    if (largerUnit(largestUnit, smallestUnit) !== largestUnit) throw rangeError('largestUnit plus petite que smallestUnit');
    const maximum = maximumRoundingIncrement(smallestUnit);
    if (maximum !== undefined) validateRoundingIncrement(roundingIncrement, maximum, false);
    return { smallestUnit, largestUnit, roundingMode, roundingIncrement };
  }

  // ---- Arrondis ----

  function unsignedRoundingMode(mode, negative) {
    switch (mode) {
      case 'ceil': return negative ? 'zero' : 'infinity';
      case 'floor': return negative ? 'infinity' : 'zero';
      case 'expand': return 'infinity';
      case 'trunc': return 'zero';
      case 'halfCeil': return negative ? 'half-zero' : 'half-infinity';
      case 'halfFloor': return negative ? 'half-infinity' : 'half-zero';
      case 'halfExpand': return 'half-infinity';
      case 'halfTrunc': return 'half-zero';
      default: return 'half-even';
    }
  }
  // Faut-il s'éloigner de la borne proche de zéro ? `half` compare le double
  // du reste à l'incrément (-1, 0, 1) ; `lowerEven` dit si cette borne est paire.
  function roundsAway(mode, half, lowerEven) {
    if (mode === 'zero') return false;
    if (mode === 'infinity') return true;
    if (half < 0) return false;
    if (half > 0) return true;
    if (mode === 'half-zero') return false;
    if (mode === 'half-infinity') return true;
    return !lowerEven;
  }
  function roundBig(value, increment, mode) {
    let quotient = value / increment;
    const remainder = value % increment;
    if (remainder === 0n) return value;
    const negative = remainder < 0n;
    const twice = (negative ? -remainder : remainder) * 2n;
    const half = twice < increment ? -1 : twice > increment ? 1 : 0;
    if (roundsAway(unsignedRoundingMode(mode, negative), half, quotient % 2n === 0n)) quotient += negative ? -1n : 1n;
    return quotient * increment;
  }
  function roundNumber(value, increment, mode) {
    const remainder = value % increment;
    if (remainder === 0) return value;
    let quotient = (value - remainder) / increment;
    const negative = remainder < 0;
    const twice = MathAbs(remainder) * 2;
    const half = twice < increment ? -1 : twice > increment ? 1 : 0;
    if (roundsAway(unsignedRoundingMode(mode, negative), half, quotient % 2 === 0)) quotient += negative ? -1 : 1;
    return quotient * increment;
  }
  function roundBigAsIfPositive(value, increment, mode) {
    let quotient = value / increment;
    let remainder = value % increment;
    if (remainder === 0n) return value;
    if (remainder < 0n) {
      quotient -= 1n;
      remainder += increment;
    }
    const twice = remainder * 2n;
    const half = twice < increment ? -1 : twice > increment ? 1 : 0;
    if (roundsAway(unsignedRoundingMode(mode, false), half, quotient % 2n === 0n)) quotient += 1n;
    return quotient * increment;
  }
  // Quotient exact de deux grands entiers, arrondi au flottant le plus proche.
  function divideToNumber(numerator, denominator) {
    const negative = numerator < 0n !== denominator < 0n;
    if (numerator < 0n) numerator = -numerator;
    if (denominator < 0n) denominator = -denominator;
    let result;
    if (numerator % denominator === 0n) {
      result = ToNumber(numerator / denominator);
    } else if (numerator < 9007199254740992n && denominator < 9007199254740992n) {
      result = ToNumber(numerator) / ToNumber(denominator);
    } else {
      // 128 bits de plus, et un bit témoin pour le reste perdu.
      const shifted = numerator << 128n;
      let scaled = shifted / denominator;
      if (shifted % denominator !== 0n) scaled |= 1n;
      result = ToNumber(scaled) / TWO_128;
    }
    return negative ? -result : result;
  }

  // ---- Calendrier ISO ----

  function isLeapYear(year) {
    return year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  }
  function daysInMonth(year, month) {
    if (month === 2) return isLeapYear(year) ? 29 : 28;
    return month === 4 || month === 6 || month === 9 || month === 11 ? 30 : 31;
  }
  // Jours depuis le 1er janvier 1970 ; `day` peut déborder du mois.
  function epochDays(year, month, day) {
    if (month <= 2) year -= 1;
    const era = MathFloor(year / 400);
    const yearOfEra = year - era * 400;
    const dayOfYear = MathFloor((153 * (month + (month > 2 ? -3 : 9)) + 2) / 5) + day - 1;
    const dayOfEra = yearOfEra * 365 + MathFloor(yearOfEra / 4) - MathFloor(yearOfEra / 100) + dayOfYear;
    return era * 146097 + dayOfEra - 719468;
  }
  function dateFromEpochDays(days) {
    days += 719468;
    const era = MathFloor(days / 146097);
    const dayOfEra = days - era * 146097;
    const yearOfEra = MathFloor((dayOfEra - MathFloor(dayOfEra / 1460) + MathFloor(dayOfEra / 36524) - MathFloor(dayOfEra / 146096)) / 365);
    const dayOfYear = dayOfEra - (365 * yearOfEra + MathFloor(yearOfEra / 4) - MathFloor(yearOfEra / 100));
    const shifted = MathFloor((5 * dayOfYear + 2) / 153);
    const day = dayOfYear - MathFloor((153 * shifted + 2) / 5) + 1;
    const month = shifted < 10 ? shifted + 3 : shifted - 9;
    return { year: yearOfEra + era * 400 + (month <= 2 ? 1 : 0), month, day };
  }
  function dateEpochDays(date) {
    return epochDays(date.year, date.month, date.day);
  }
  function balanceDate(year, month, day) {
    return dateFromEpochDays(epochDays(year, month, day));
  }
  function balanceYearMonth(year, month) {
    const shift = MathFloor((month - 1) / 12);
    return { year: year + shift, month: month - shift * 12 };
  }
  function isValidDate(year, month, day) {
    return month >= 1 && month <= 12 && day >= 1 && day <= daysInMonth(year, month);
  }
  function regulateDate(year, month, day, overflow) {
    if (overflow === 'constrain') {
      month = month < 1 ? 1 : month > 12 ? 12 : month;
      const last = daysInMonth(year, month);
      day = day < 1 ? 1 : day > last ? last : day;
    } else if (!isValidDate(year, month, day)) {
      throw rangeError('date invalide');
    }
    return { year, month, day };
  }
  function dateWithinLimits(date) {
    if (!(date.year >= -271821 && date.year <= 275760)) return false;
    const days = dateEpochDays(date);
    return days >= -100000001 && days <= 100000000;
  }
  function dateTimeWithinLimits(dateTime) {
    if (!(dateTime.year >= -271821 && dateTime.year <= 275760)) return false;
    const days = dateEpochDays(dateTime);
    if (days === -100000001) return dateTime.t > 0;
    return days > -100000001 && days <= 100000000;
  }
  function yearMonthWithinLimits(year, month) {
    if (!(year >= -271821 && year <= 275760)) return false;
    if (year === -271821 && month < 4) return false;
    return !(year === 275760 && month > 9);
  }
  function checkDaysRange(date) {
    if (!(MathAbs(dateEpochDays(date)) <= 100000000)) throw rangeError('date hors de la plage des instants');
  }
  function compareDate(one, two) {
    if (one.year !== two.year) return one.year < two.year ? -1 : 1;
    if (one.month !== two.month) return one.month < two.month ? -1 : 1;
    if (one.day !== two.day) return one.day < two.day ? -1 : 1;
    return 0;
  }
  function compareDateTime(one, two) {
    const order = compareDate(one, two);
    if (order !== 0) return order;
    return one.t < two.t ? -1 : one.t > two.t ? 1 : 0;
  }
  function dayOfWeek(date) {
    const rest = (dateEpochDays(date) + 3) % 7;
    return (rest < 0 ? rest + 7 : rest) + 1;
  }
  function dayOfYear(date) {
    return dateEpochDays(date) - epochDays(date.year, 1, 1) + 1;
  }
  function weekOfYear(date) {
    const ordinal = dayOfYear(date);
    const weekday = dayOfWeek(date);
    const week = MathFloor((ordinal - weekday + 10) / 7);
    if (week < 1) {
      const first = dayOfWeek({ year: date.year, month: 1, day: 1 });
      const long = first === 5 || (first === 6 && isLeapYear(date.year - 1));
      return { week: long ? 53 : 52, year: date.year - 1 };
    }
    if (week === 53 && (isLeapYear(date.year) ? 366 : 365) - ordinal < 4 - weekday) {
      return { week: 1, year: date.year + 1 };
    }
    return { week, year: date.year };
  }
  function timeToNs(hour, minute, second, millisecond, microsecond, nanosecond) {
    return hour * 3600000000000 + minute * 60000000000 + second * 1000000000 + millisecond * 1000000 + microsecond * 1000 + nanosecond;
  }
  function isValidTime(hour, minute, second, millisecond, microsecond, nanosecond) {
    return hour >= 0 && hour <= 23 && minute >= 0 && minute <= 59 && second >= 0 && second <= 59
      && millisecond >= 0 && millisecond <= 999 && microsecond >= 0 && microsecond <= 999 && nanosecond >= 0 && nanosecond <= 999;
  }
  function timeField(t, index) {
    switch (index) {
      case 0: return MathFloor(t / 3600000000000);
      case 1: return MathFloor(t / 60000000000) % 60;
      case 2: return MathFloor(t / 1000000000) % 60;
      case 3: return MathFloor(t / 1000000) % 1000;
      case 4: return MathFloor(t / 1000) % 1000;
      default: return t % 1000;
    }
  }
  function combine(date, t) {
    return { year: date.year, month: date.month, day: date.day, t };
  }
  // Nanosecondes d'époque d'une date-heure lue comme UTC.
  function utcEpochNs(dateTime) {
    return ToBigInt(dateEpochDays(dateTime)) * NS_DAY_BIG + ToBigInt(dateTime.t);
  }
  // Date-heure UTC d'un instant décalé de `offset` nanosecondes.
  function dateTimeFromEpochNs(ns, offset) {
    ns += ToBigInt(offset);
    let days = ns / NS_DAY_BIG;
    let rest = ns % NS_DAY_BIG;
    if (rest < 0n) {
      days -= 1n;
      rest += NS_DAY_BIG;
    }
    return combine(dateFromEpochDays(ToNumber(days)), ToNumber(rest));
  }
  // Date-heure décalée de `delta` nanosecondes (nombre entier exact).
  function shiftDateTime(dateTime, delta) {
    const total = dateTime.t + delta;
    const days = MathFloor(total / NS_DAY);
    return combine(balanceDate(dateTime.year, dateTime.month, dateTime.day + days), total - days * NS_DAY);
  }
  function isValidEpochNs(ns) {
    return ns >= -NS_MAX_INSTANT && ns <= NS_MAX_INSTANT;
  }

  // ---- Mise en forme ----

  function padYear(year) {
    if (year >= 0 && year <= 9999) return pad(year, 4);
    return (year < 0 ? '-' : '+') + pad(MathAbs(year), 6);
  }
  function formatFraction(ns, precision) {
    if (precision === 'auto') {
      if (ns === 0) return '';
      let text = pad(ns, 9);
      let end = 9;
      while (text[end - 1] === '0') end--;
      return '.' + text.slice(0, end);
    }
    if (precision === 0) return '';
    return '.' + pad(ns, 9).slice(0, precision);
  }
  function formatTime(t, precision) {
    const head = pad(timeField(t, 0), 2) + ':' + pad(timeField(t, 1), 2);
    if (precision === 'minute') return head;
    return head + ':' + pad(timeField(t, 2), 2) + formatFraction(t % 1000000000, precision);
  }
  function formatCalendarAnnotation(calendar, show) {
    if (show === 'never') return '';
    if (show === 'auto' && calendar === 'iso8601') return '';
    return '[' + (show === 'critical' ? '!' : '') + 'u-ca=' + calendar + ']';
  }
  function formatDate(date) {
    return padYear(date.year) + '-' + pad(date.month, 2) + '-' + pad(date.day, 2);
  }
  function formatDateTime(dateTime, calendar, precision, showCalendar) {
    return formatDate(dateTime) + 'T' + formatTime(dateTime.t, precision) + formatCalendarAnnotation(calendar, showCalendar);
  }
  function formatOffsetMinutes(minutes) {
    const sign = minutes < 0 ? '-' : '+';
    minutes = MathAbs(minutes);
    return sign + pad(MathFloor(minutes / 60), 2) + ':' + pad(minutes % 60, 2);
  }
  function formatOffsetNs(ns) {
    const sign = ns < 0 ? '-' : '+';
    ns = MathAbs(ns);
    const head = sign + pad(MathFloor(ns / 3600000000000), 2) + ':' + pad(MathFloor(ns / 60000000000) % 60, 2);
    const rest = ns % 60000000000;
    if (rest === 0) return head;
    return head + ':' + pad(MathFloor(rest / 1000000000), 2) + formatFraction(rest % 1000000000, 'auto');
  }
  function formatOffsetRounded(ns) {
    return formatOffsetMinutes(roundNumber(ns, 60000000000, 'halfExpand') / 60000000000);
  }

  // ---- Lecture des chaînes ISO 8601 ----

  function isDigit(text, index) {
    const code = text.charCodeAt(index);
    return code >= 48 && code <= 57;
  }
  function isAlpha(code) {
    return (code >= 65 && code <= 90) || (code >= 97 && code <= 122);
  }
  // Entier des `count` chiffres à partir de `index`, ou -1.
  function readDigits(text, index, count) {
    let value = 0;
    for (let i = 0; i < count; i++) {
      if (!isDigit(text, index + i)) return -1;
      value = value * 10 + (text.charCodeAt(index + i) - 48);
    }
    return value;
  }
  // Année à `index` : `{ year, end }` ou null.
  function readYear(text, index) {
    const first = text[index];
    if (first === '+' || first === '-') {
      const value = readDigits(text, index + 1, 6);
      if (value < 0 || (first === '-' && value === 0)) return null;
      return { year: first === '-' ? -value : value, end: index + 7 };
    }
    const value = readDigits(text, index, 4);
    return value < 0 ? null : { year: value, end: index + 4 };
  }
  function readDate(text, index) {
    const head = readYear(text, index);
    if (head === null) return null;
    let at = head.end;
    const extended = text[at] === '-';
    if (extended) at++;
    const month = readDigits(text, at, 2);
    if (month < 1 || month > 12) return null;
    at += 2;
    if (extended) {
      if (text[at] !== '-') return null;
      at++;
    }
    const day = readDigits(text, at, 2);
    if (day < 1 || day > 31) return null;
    return { year: head.year, month, day, end: at + 2 };
  }
  // Fraction décimale à `index` (après le séparateur) : `{ ns, end }` ou null.
  function readFraction(text, index) {
    let at = index;
    let value = 0;
    while (at - index < 9 && isDigit(text, at)) {
      value = value * 10 + (text.charCodeAt(at) - 48);
      at++;
    }
    if (at === index) return null;
    for (let i = at - index; i < 9; i++) value *= 10;
    return { ns: value, end: at };
  }
  // Heure à `index` : `{ t, end }` ou null. `leap` admet la seconde 60.
  function readClock(text, index, leap, subMinute) {
    const hour = readDigits(text, index, 2);
    if (hour < 0 || hour > 23) return null;
    let at = index + 2;
    let minute = 0;
    let second = 0;
    let fraction = 0;
    let hasSeconds = false;
    const extended = text[at] === ':';
    if (extended || isDigit(text, at)) {
      minute = readDigits(text, extended ? at + 1 : at, 2);
      if (minute < 0 || minute > 59) return null;
      at += extended ? 3 : 2;
      if (subMinute && (extended ? text[at] === ':' : isDigit(text, at))) {
        second = readDigits(text, extended ? at + 1 : at, 2);
        if (second < 0 || second > (leap ? 60 : 59)) return null;
        if (second === 60) second = 59;
        hasSeconds = true;
        at += extended ? 3 : 2;
        if (text[at] === '.' || text[at] === ',') {
          const part = readFraction(text, at + 1);
          if (part === null) return null;
          fraction = part.ns;
          at = part.end;
        }
      }
    }
    return { t: timeToNs(hour, minute, second, 0, 0, fraction), end: at, hasSeconds };
  }
  // Décalage UTC à `index` : `{ ns, end, hasSeconds }` ou null.
  function readOffset(text, index, subMinute) {
    const sign = text[index] === '+' ? 1 : text[index] === '-' ? -1 : 0;
    if (sign === 0) return null;
    const clock = readClock(text, index + 1, false, subMinute);
    if (clock === null) return null;
    return { ns: sign * clock.t, end: clock.end, hasSeconds: clock.hasSeconds };
  }
  function isTimeZoneName(text) {
    if (text.length === 0) return false;
    let start = true;
    for (let i = 0; i < text.length; i++) {
      const code = text.charCodeAt(i);
      if (code === 47) {
        if (start) return false;
        start = true;
        continue;
      }
      const lead = isAlpha(code) || code === 46 || code === 95;
      if (!(lead || (!start && ((code >= 48 && code <= 57) || code === 45 || code === 43)))) return false;
      start = false;
    }
    return !start;
  }
  function isOffsetIdentifier(text) {
    const offset = readOffset(text, 0, false);
    return offset !== null && offset.end === text.length;
  }
  function isAnnotationValue(text) {
    let start = true;
    for (let i = 0; i < text.length; i++) {
      const code = text.charCodeAt(i);
      if (code === 45) {
        if (start) return false;
        start = true;
      } else if (isAlpha(code) || (code >= 48 && code <= 57)) {
        start = false;
      } else {
        return false;
      }
    }
    return !start;
  }
  function isAnnotationKey(text) {
    for (let i = 0; i < text.length; i++) {
      const code = text.charCodeAt(i);
      const lead = (code >= 97 && code <= 122) || code === 95;
      if (!(lead || (i > 0 && ((code >= 48 && code <= 57) || code === 45)))) return false;
    }
    return text.length > 0;
  }
  // Annotations entre crochets à partir de `index` ; complète `out` et rend
  // la position de fin, ou -1.
  function readAnnotations(text, index, out) {
    let first = true;
    let calendarCritical = false;
    let calendars = 0;
    while (text[index] === '[') {
      const close = text.indexOf(']', index);
      if (close < 0) return -1;
      let body = text.slice(index + 1, close);
      const critical = body[0] === '!';
      if (critical) body = body.slice(1);
      const equals = body.indexOf('=');
      if (equals < 0) {
        if (!first || !(isOffsetIdentifier(body) || isTimeZoneName(body))) return -1;
        out.timeZone = body;
      } else {
        const key = body.slice(0, equals);
        const value = body.slice(equals + 1);
        if (!isAnnotationKey(key) || !isAnnotationValue(value)) return -1;
        if (key === 'u-ca') {
          calendars++;
          if (critical) calendarCritical = true;
          if (out.calendar === undefined) out.calendar = value;
        } else if (critical) {
          return -1;
        }
      }
      first = false;
      index = close + 1;
    }
    if (calendars > 1 && calendarCritical) return -1;
    return index;
  }
  function newParseResult() {
    return { year: undefined, month: 1, day: 1, t: null, z: false, offset: undefined, offsetHasSeconds: false, timeZone: undefined, calendar: undefined };
  }
  // Date, puis heure et décalage facultatifs, puis annotations.
  function parseDateTimeForm(text) {
    const date = readDate(text, 0);
    if (date === null) return null;
    const result = newParseResult();
    result.year = date.year;
    result.month = date.month;
    result.day = date.day;
    let at = date.end;
    const separator = text[at];
    if (separator === 'T' || separator === 't' || separator === ' ') {
      const clock = readClock(text, at + 1, true, true);
      if (clock === null) return null;
      result.t = clock.t;
      at = clock.end;
      if (text[at] === 'Z' || text[at] === 'z') {
        result.z = true;
        at++;
      } else if (text[at] === '+' || text[at] === '-') {
        const offset = readOffset(text, at, true);
        if (offset === null) return null;
        result.offset = text.slice(at, offset.end);
        result.offsetHasSeconds = offset.hasSeconds;
        at = offset.end;
      }
    }
    if (readAnnotations(text, at, result) !== text.length) return null;
    if (date.day > daysInMonth(date.year, date.month)) return null;
    return result;
  }
  function looksLikeMonthDay(text) {
    let at = text[0] === '-' && text[1] === '-' ? 2 : 0;
    const month = readDigits(text, at, 2);
    if (month < 1 || month > 12) return false;
    at += 2;
    if (text[at] === '-') at++;
    const day = readDigits(text, at, 2);
    return day >= 1 && day <= daysInMonth(1972, month) && at + 2 === text.length;
  }
  function looksLikeYearMonth(text) {
    const head = readYear(text, 0);
    if (head === null) return false;
    let at = head.end;
    if (text[at] === '-') at++;
    const month = readDigits(text, at, 2);
    return month >= 1 && month <= 12 && at + 2 === text.length;
  }
  function parseTimeForm(text) {
    const designator = text[0] === 'T' || text[0] === 't';
    const clock = readClock(text, designator ? 1 : 0, true, true);
    if (clock === null) return null;
    const result = newParseResult();
    result.t = clock.t;
    let at = clock.end;
    if (text[at] === '+' || text[at] === '-') {
      const offset = readOffset(text, at, true);
      if (offset === null) return null;
      result.offset = text.slice(at, offset.end);
      result.offsetHasSeconds = offset.hasSeconds;
      at = offset.end;
    }
    if (!designator) {
      const bare = text.slice(0, at);
      if (looksLikeMonthDay(bare) || looksLikeYearMonth(bare)) return null;
    }
    if (readAnnotations(text, at, result) !== text.length) return null;
    return result;
  }
  function isIsoCalendarName(name) {
    return name === undefined || name.toLowerCase() === 'iso8601';
  }
  function parseYearMonthForm(text) {
    const head = readYear(text, 0);
    if (head === null) return null;
    let at = head.end;
    if (text[at] === '-') at++;
    const month = readDigits(text, at, 2);
    if (month < 1 || month > 12) return null;
    const result = newParseResult();
    result.year = head.year;
    result.month = month;
    if (readAnnotations(text, at + 2, result) !== text.length) return null;
    if (!isIsoCalendarName(result.calendar)) return null;
    return result;
  }
  function parseMonthDayForm(text) {
    let at = text[0] === '-' && text[1] === '-' ? 2 : 0;
    const month = readDigits(text, at, 2);
    if (month < 1 || month > 12) return null;
    at += 2;
    if (text[at] === '-') at++;
    const day = readDigits(text, at, 2);
    if (day < 1 || day > daysInMonth(1972, month)) return null;
    const result = newParseResult();
    result.month = month;
    result.day = day;
    if (readAnnotations(text, at + 2, result) !== text.length) return null;
    if (!isIsoCalendarName(result.calendar)) return null;
    return result;
  }
  // `format` : 'datetime', 'zoned', 'relative', 'instant', 'time',
  // 'yearmonth', 'monthday' ou 'any'.
  function parseISO(text, format) {
    let result = parseDateTimeForm(text);
    if (result !== null) {
      let accepted;
      switch (format) {
        case 'zoned': accepted = result.timeZone !== undefined; break;
        case 'relative': accepted = result.timeZone !== undefined || !result.z; break;
        case 'instant': accepted = result.t !== null && (result.z || result.offset !== undefined); break;
        case 'time': accepted = result.t !== null && !result.z; break;
        case 'any': accepted = true; break;
        default: accepted = !result.z; break;
      }
      if (accepted) return result;
      result = null;
    }
    if (format === 'time' || format === 'any') result = parseTimeForm(text);
    if (result === null && (format === 'yearmonth' || format === 'any')) result = parseYearMonthForm(text);
    if (result === null && (format === 'monthday' || format === 'any')) result = parseMonthDayForm(text);
    if (result === null) throw rangeError(`chaîne ISO 8601 invalide : ${text}`);
    return result;
  }
  function parseOffsetString(text) {
    const offset = readOffset(text, 0, true);
    if (offset === null || offset.end !== text.length) throw rangeError(`décalage UTC invalide : ${text}`);
    return offset.ns;
  }
  function parseDurationString(text) {
    const bad = () => rangeError(`durée ISO 8601 invalide : ${text}`);
    const length = text.length;
    let at = 0;
    let sign = 1;
    if (text[at] === '+' || text[at] === '-') {
      if (text[at] === '-') sign = -1;
      at++;
    }
    if (text[at] !== 'P' && text[at] !== 'p') throw bad();
    at++;
    const fields = [0, 0, 0, 0, 0, 0, 0];
    let fraction = -1;
    let fractionUnit = -1;
    let order = 0;
    let inTime = false;
    let any = false;
    while (at < length) {
      if (!inTime && (text[at] === 'T' || text[at] === 't')) {
        inTime = true;
        order = 4;
        at++;
        if (at >= length) throw bad();
        continue;
      }
      const start = at;
      while (isDigit(text, at)) at++;
      if (at === start) throw bad();
      const whole = ToNumber(text.slice(start, at));
      let part = null;
      if (inTime && (text[at] === '.' || text[at] === ',')) {
        part = readFraction(text, at + 1);
        if (part === null) throw bad();
        at = part.end;
      }
      const letter = at < length ? text[at].toUpperCase() : '';
      const unit = inTime ? 4 + 'HMS'.indexOf(letter) : 'YMWD'.indexOf(letter);
      if (letter === '' || unit < order || (inTime && unit < 4)) throw bad();
      order = unit + 1;
      fields[unit] = whole;
      at++;
      any = true;
      if (part !== null) {
        if (at !== length) throw bad();
        fraction = part.ns;
        fractionUnit = unit;
      }
    }
    if (!any) throw bad();
    let minutes = fields[5];
    let seconds = fields[6];
    let subSecond = 0;
    if (fractionUnit === 4) {
      // Fraction d'heure : `fraction` milliardièmes d'heure.
      const ns = fraction * 3600;
      minutes = MathFloor(ns / 60000000000);
      seconds = MathFloor(ns / 1000000000) % 60;
      subSecond = ns % 1000000000;
    } else if (fractionUnit === 5) {
      const ns = fraction * 60;
      seconds = MathFloor(ns / 1000000000);
      subSecond = ns % 1000000000;
    } else if (fractionUnit === 6) {
      subSecond = fraction;
    }
    return makeDuration(
      sign * fields[0], sign * fields[1], sign * fields[2], sign * fields[3], sign * fields[4], sign * minutes, sign * seconds,
      sign * MathFloor(subSecond / 1000000), sign * (MathFloor(subSecond / 1000) % 1000), sign * (subSecond % 1000),
    );
  }
