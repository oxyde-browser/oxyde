  // Temporal, deuxième partie : opérations du calendrier, durées et arrondis
  // relatifs à une date.

  // ---- Calendrier ----

  const PARTIAL = { __proto__: null };
  const DATE_FIELDS = ['day', 'month', 'monthCode', 'year'];
  const YEAR_MONTH_FIELDS = ['month', 'monthCode', 'year'];
  const DATE_TIME_FIELDS = ['day', 'hour', 'microsecond', 'millisecond', 'minute', 'month', 'monthCode', 'nanosecond', 'second', 'year'];
  const ZONED_WITH_FIELDS = ['day', 'hour', 'microsecond', 'millisecond', 'minute', 'month', 'monthCode', 'nanosecond', 'offset', 'second', 'year'];
  const ZONED_FIELDS = ['day', 'hour', 'microsecond', 'millisecond', 'minute', 'month', 'monthCode', 'nanosecond', 'offset', 'second', 'timeZone', 'year'];
  const TIME_FIELDS = ['hour', 'microsecond', 'millisecond', 'minute', 'nanosecond', 'second'];
  const NONE = [];

  function canonicalizeCalendar(identifier) {
    if (identifier.toLowerCase() !== 'iso8601') throw rangeError(`calendrier inconnu : ${identifier}`);
    return 'iso8601';
  }
  // Calendrier d'un objet Temporal qui en porte un, sinon `undefined`.
  function temporalCalendar(value) {
    if (isPlainDate(value)) return dateSlot(value).calendar;
    if (isPlainDateTime(value)) return dateTimeSlot(value).calendar;
    if (isPlainMonthDay(value)) return monthDaySlot(value).calendar;
    if (isPlainYearMonth(value)) return yearMonthSlot(value).calendar;
    if (isZoned(value)) return zonedSlot(value).calendar;
    return undefined;
  }
  function toCalendarIdentifier(like) {
    if (isObject(like)) {
      const calendar = temporalCalendar(like);
      if (calendar !== undefined) return calendar;
    }
    if (typeof like !== 'string') throw typeError('identifiant de calendrier attendu');
    let parsed = null;
    try {
      parsed = parseISO(like, 'any');
    } catch (error) {
      if (!isAnnotationValue(like)) throw rangeError(`calendrier invalide : ${like}`);
      return canonicalizeCalendar(like);
    }
    return canonicalizeCalendar(parsed.calendar === undefined ? 'iso8601' : parsed.calendar);
  }
  function getCalendarWithISODefault(item) {
    const calendar = temporalCalendar(item);
    if (calendar !== undefined) return calendar;
    const like = item.calendar;
    if (like === undefined) return 'iso8601';
    return toCalendarIdentifier(like);
  }
  function toMonthCode(value) {
    const code = toPrimitiveString(value);
    if (typeof code !== 'string') throw typeError('monthCode doit être une chaîne');
    const ok = (code.length === 3 || (code.length === 4 && code[3] === 'L')) && code[0] === 'M' && readDigits(code, 1, 2) >= 0
      && (code.length === 4 || code !== 'M00');
    if (!ok) throw rangeError(`monthCode invalide : ${code}`);
    return code;
  }
  function toOffsetString(value) {
    const offset = toPrimitiveString(value);
    if (typeof offset !== 'string') throw typeError('offset doit être une chaîne');
    parseOffsetString(offset);
    return offset;
  }
  function convertField(name, value) {
    switch (name) {
      case 'month':
      case 'day': return toPositiveIntegerWithTruncation(value);
      case 'monthCode': return toMonthCode(value);
      case 'offset': return toOffsetString(value);
      case 'timeZone': return toTimeZoneIdentifier(value);
      default: return toIntegerWithTruncation(value);
    }
  }
  // Lit dans `bag` les champs `names` (triés). `required` : liste de champs
  // obligatoires, ou PARTIAL pour exiger seulement qu'un champ soit présent.
  function prepareFields(bag, names, required) {
    const result = ObjectCreate(null);
    let any = false;
    for (let i = 0; i < names.length; i++) {
      const name = names[i];
      const value = bag[name];
      if (value !== undefined) {
        any = true;
        result[name] = convertField(name, value);
      } else if (required !== PARTIAL) {
        if (inList(required, name)) throw typeError(`champ ${name} requis`);
        if (inList(TIME_FIELDS, name)) result[name] = 0;
      }
    }
    if (required === PARTIAL && !any) throw typeError('aucun champ reconnu');
    return result;
  }
  // `type` : 'date', 'year-month' ou 'month-day'. Fixe `fields.month`.
  function resolveFields(fields, type) {
    if (type !== 'month-day' && fields.year === undefined) throw typeError('champ year requis');
    if (type !== 'year-month' && fields.day === undefined) throw typeError('champ day requis');
    const code = fields.monthCode;
    if (code === undefined) {
      if (fields.month === undefined) throw typeError('champ month ou monthCode requis');
      return;
    }
    const month = readDigits(code, 1, 2);
    if (code.length === 4 || month < 1 || month > 12) throw rangeError(`monthCode invalide : ${code}`);
    if (fields.month !== undefined && fields.month !== month) throw rangeError('month et monthCode se contredisent');
    fields.month = month;
  }
  function dateFromFields(fields, overflow) {
    resolveFields(fields, 'date');
    const date = regulateDate(fields.year, fields.month, fields.day, overflow);
    if (!dateWithinLimits(date)) throw rangeError('date hors limites');
    return date;
  }
  function yearMonthFromFields(fields, overflow) {
    fields.day = 1;
    resolveFields(fields, 'year-month');
    const date = regulateDate(fields.year, fields.month, 1, overflow);
    if (!yearMonthWithinLimits(date.year, date.month)) throw rangeError('mois hors limites');
    return date;
  }
  function monthDayFromFields(fields, overflow) {
    resolveFields(fields, 'month-day');
    const year = fields.year === undefined ? 1972 : fields.year;
    const date = regulateDate(year, fields.month, fields.day, overflow);
    return { year: 1972, month: date.month, day: date.day };
  }
  function interpretDateTimeFields(fields, overflow) {
    const date = dateFromFields(fields, overflow);
    return combine(date, regulateTime(fields.hour, fields.minute, fields.second, fields.millisecond, fields.microsecond, fields.nanosecond, overflow));
  }
  function regulateTime(hour, minute, second, millisecond, microsecond, nanosecond, overflow) {
    if (overflow === 'constrain') {
      const clamp = (value, maximum) => (value < 0 ? 0 : value > maximum ? maximum : value);
      return timeToNs(clamp(hour, 23), clamp(minute, 59), clamp(second, 59), clamp(millisecond, 999), clamp(microsecond, 999), clamp(nanosecond, 999));
    }
    if (!isValidTime(hour, minute, second, millisecond, microsecond, nanosecond)) throw rangeError('heure invalide');
    return timeToNs(hour, minute, second, millisecond, microsecond, nanosecond);
  }
  // Champs d'une date : `type` vaut 'date', 'year-month' ou 'month-day'.
  function dateToFields(date, type) {
    const fields = ObjectCreate(null);
    fields.monthCode = 'M' + pad(date.month, 2);
    if (type !== 'month-day') fields.year = date.year;
    if (type !== 'year-month') fields.day = date.day;
    return fields;
  }
  function mergeFields(fields, additional) {
    const merged = ObjectCreate(null);
    const replacesMonth = additional.month !== undefined || additional.monthCode !== undefined;
    for (const key in fields) {
      if (replacesMonth && (key === 'month' || key === 'monthCode')) continue;
      merged[key] = fields[key];
    }
    for (const key in additional) merged[key] = additional[key];
    return merged;
  }
  function dateAdd(date, duration, overflow) {
    const yearMonth = balanceYearMonth(date.year + duration.years, date.month + duration.months);
    const start = regulateDate(yearMonth.year, yearMonth.month, date.day, overflow);
    const result = balanceDate(start.year, start.month, start.day + duration.days + 7 * duration.weeks);
    if (!dateWithinLimits(result)) throw rangeError('date hors limites');
    return result;
  }
  function dateSurpasses(sign, year, month, day, target) {
    if (year !== target.year) return sign * (year - target.year) > 0;
    if (month !== target.month) return sign * (month - target.month) > 0;
    return day !== target.day && sign * (day - target.day) > 0;
  }
  function dateUntil(one, two, largestUnit) {
    const sign = -compareDate(one, two);
    if (sign === 0) return { years: 0, months: 0, weeks: 0, days: 0 };
    let years = 0;
    let months = 0;
    if (largestUnit === 'year' || largestUnit === 'month') {
      let candidateYears = two.year - one.year;
      if (candidateYears !== 0) candidateYears -= sign;
      while (!dateSurpasses(sign, one.year + candidateYears, one.month, one.day, two)) {
        years = candidateYears;
        candidateYears += sign;
      }
      let candidateMonths = sign;
      let step = balanceYearMonth(one.year + years, one.month + candidateMonths);
      while (!dateSurpasses(sign, step.year, step.month, one.day, two)) {
        months = candidateMonths;
        candidateMonths += sign;
        step = balanceYearMonth(step.year, step.month + sign);
      }
      if (largestUnit === 'month') {
        months += years * 12;
        years = 0;
      }
    }
    const step = balanceYearMonth(one.year + years, one.month + months);
    const constrained = regulateDate(step.year, step.month, one.day, 'constrain');
    let weeks = 0;
    let days = dateEpochDays(two) - dateEpochDays(constrained);
    if (largestUnit === 'week') {
      weeks = MathTrunc(days / 7) + 0;
      days = (days % 7) + 0;
    }
    return { years, months, weeks, days };
  }

  // ---- Durées internes ----
  //
  // Une durée interne porte des années, mois, semaines et jours (nombres) et
  // une durée horaire `time` en nanosecondes (grand entier).

  const DURATION_PROPERTIES = ['years', 'months', 'weeks', 'days', 'hours', 'minutes', 'seconds', 'milliseconds', 'microseconds', 'nanoseconds'];
  const DURATION_READ_ORDER = [3, 4, 8, 7, 5, 1, 9, 6, 2, 0];

  function timeDurationFromFields(fields) {
    return ToBigInt(fields[4]) * 3600000000000n + ToBigInt(fields[5]) * 60000000000n + ToBigInt(fields[6]) * 1000000000n
      + ToBigInt(fields[7]) * 1000000n + ToBigInt(fields[8]) * 1000n + ToBigInt(fields[9]);
  }
  function isValidDuration(fields) {
    let sign = 0;
    for (let i = 0; i < 10; i++) {
      const value = fields[i];
      if (!NumberIsFinite(value)) return false;
      if (value !== 0) {
        const one = value < 0 ? -1 : 1;
        if (sign !== 0 && one !== sign) return false;
        sign = one;
      }
    }
    if (MathAbs(fields[0]) >= 4294967296 || MathAbs(fields[1]) >= 4294967296 || MathAbs(fields[2]) >= 4294967296) return false;
    const total = ToBigInt(fields[3]) * NS_DAY_BIG + timeDurationFromFields(fields);
    return total <= MAX_TIME_DURATION && total >= -MAX_TIME_DURATION;
  }
  function makeDuration(years, months, weeks, days, hours, minutes, seconds, milliseconds, microseconds, nanoseconds) {
    return new Duration(years, months, weeks, days, hours, minutes, seconds, milliseconds, microseconds, nanoseconds);
  }
  function durationFromFields(f) {
    return new Duration(f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8], f[9]);
  }
  function negatedFields(f) {
    const out = [];
    for (let i = 0; i < 10; i++) out[i] = -f[i] + 0;
    return out;
  }
  function durationSign(f) {
    for (let i = 0; i < 10; i++) {
      if (f[i] !== 0) return f[i] < 0 ? -1 : 1;
    }
    return 0;
  }
  function defaultLargestUnit(f) {
    for (let i = 0; i < 9; i++) {
      if (f[i] !== 0) return UNITS[i];
    }
    return 'nanosecond';
  }
  function checkTimeDuration(time) {
    if (time > MAX_TIME_DURATION || time < -MAX_TIME_DURATION) throw rangeError('durée hors limites');
    return time;
  }
  function add24HourDays(time, days) {
    return checkTimeDuration(time + ToBigInt(days) * NS_DAY_BIG);
  }
  function toInternal(f) {
    return { years: f[0], months: f[1], weeks: f[2], days: f[3], time: timeDurationFromFields(f) };
  }
  function toInternal24(f) {
    return { years: f[0], months: f[1], weeks: f[2], days: 0, time: add24HourDays(timeDurationFromFields(f), f[3]) };
  }
  function dateDuration(years, months, weeks, days) {
    return { years, months, weeks, days, time: 0n };
  }
  function toDateDurationWithoutTime(f) {
    const internal = toInternal24(f);
    return dateDuration(f[0], f[1], f[2], ToNumber(internal.time / NS_DAY_BIG));
  }
  function timeOnly(time) {
    return { years: 0, months: 0, weeks: 0, days: 0, time };
  }
  function internalSign(duration) {
    const date = [duration.years, duration.months, duration.weeks, duration.days];
    for (let i = 0; i < 4; i++) {
      if (date[i] !== 0) return date[i] < 0 ? -1 : 1;
    }
    return duration.time < 0n ? -1 : duration.time > 0n ? 1 : 0;
  }
  function durationFromInternal(duration, largestUnit) {
    let ns = duration.time;
    const sign = ns < 0n ? -1 : 1;
    if (sign < 0) ns = -ns;
    const index = UNIT_INDEX[largestUnit];
    let days = 0n;
    let hours = 0n;
    let minutes = 0n;
    let seconds = 0n;
    let milliseconds = 0n;
    let microseconds = 0n;
    if (index <= 8) {
      microseconds = ns / 1000n;
      ns %= 1000n;
    }
    if (index <= 7) {
      milliseconds = microseconds / 1000n;
      microseconds %= 1000n;
    }
    if (index <= 6) {
      seconds = milliseconds / 1000n;
      milliseconds %= 1000n;
    }
    if (index <= 5) {
      minutes = seconds / 60n;
      seconds %= 60n;
    }
    if (index <= 4) {
      hours = minutes / 60n;
      minutes %= 60n;
    }
    if (index <= 3) {
      days = hours / 24n;
      hours %= 24n;
    }
    return makeDuration(
      duration.years, duration.months, duration.weeks, duration.days + sign * ToNumber(days), sign * ToNumber(hours), sign * ToNumber(minutes),
      sign * ToNumber(seconds), sign * ToNumber(milliseconds), sign * ToNumber(microseconds), sign * ToNumber(ns),
    );
  }
  function roundTimeDuration(time, increment, unit, mode) {
    return checkTimeDuration(roundBig(time, ToBigInt(increment) * ToBigInt(UNIT_NS[unit]), mode));
  }
  function totalTimeDuration(time, unit) {
    return divideToNumber(time, ToBigInt(UNIT_NS[unit]));
  }
  // Heure du jour décalée d'une durée : `{ days, t }`.
  function addTime(t, time) {
    const total = ToBigInt(t) + time;
    let days = total / NS_DAY_BIG;
    let rest = total % NS_DAY_BIG;
    if (rest < 0n) {
      days -= 1n;
      rest += NS_DAY_BIG;
    }
    return { days: ToNumber(days), t: ToNumber(rest) };
  }
  function toPartialDuration(like) {
    if (!isObject(like)) throw typeError('durée attendue');
    const out = [undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined, undefined];
    let any = false;
    for (let i = 0; i < 10; i++) {
      const index = DURATION_READ_ORDER[i];
      const value = like[DURATION_PROPERTIES[index]];
      if (value !== undefined) {
        out[index] = toIntegerIfIntegral(value);
        any = true;
      }
    }
    if (!any) throw typeError('aucun champ de durée reconnu');
    return out;
  }
  // Les dix champs d'une durée donnée sous toute forme admise.
  function toDurationFields(item) {
    if (isObject(item)) {
      if (isDuration(item)) return durationSlot(item);
      const partial = toPartialDuration(item);
      for (let i = 0; i < 10; i++) {
        if (partial[i] === undefined) partial[i] = 0;
      }
      if (!isValidDuration(partial)) throw rangeError('durée invalide');
      return partial;
    }
    if (typeof item !== 'string') throw typeError('durée attendue');
    return durationSlot(parseDurationString(item));
  }

  // ---- Différences et arrondis relatifs ----

  function epochNsOf(dateTime, timeZone) {
    return timeZone === undefined ? utcEpochNs(dateTime) : getEpochNsFor(timeZone, dateTime, 'compatible');
  }
  function differenceDateTime(one, two, largestUnit) {
    let time = ToBigInt(two.t - one.t);
    const timeSign = time < 0n ? -1 : time > 0n ? 1 : 0;
    const dateSign = compareDate(one, two);
    let adjusted = two;
    if (timeSign === dateSign && timeSign !== 0) {
      adjusted = balanceDate(two.year, two.month, two.day + timeSign);
      time = add24HourDays(time, -timeSign);
    }
    const dateLargestUnit = largerUnit('day', largestUnit);
    const difference = dateUntil(one, adjusted, dateLargestUnit);
    if (largestUnit !== dateLargestUnit) {
      time = add24HourDays(time, difference.days);
      difference.days = 0;
    }
    difference.time = time;
    return difference;
  }
  function differencePlainDateTimeWithRounding(one, two, largestUnit, increment, smallestUnit, mode) {
    if (compareDateTime(one, two) === 0) return timeOnly(0n);
    if (!dateTimeWithinLimits(one) || !dateTimeWithinLimits(two)) throw rangeError('date hors limites');
    const difference = differenceDateTime(one, two, largestUnit);
    if (smallestUnit === 'nanosecond' && increment === 1) return difference;
    return roundRelativeDuration(difference, utcEpochNs(two), one, undefined, largestUnit, increment, smallestUnit, mode);
  }
  function differencePlainDateTimeWithTotal(one, two, unit) {
    if (compareDateTime(one, two) === 0) return 0;
    if (!dateTimeWithinLimits(one) || !dateTimeWithinLimits(two)) throw rangeError('date hors limites');
    const difference = differenceDateTime(one, two, unit);
    if (unit === 'nanosecond') return ToNumber(difference.time);
    return totalRelativeDuration(difference, utcEpochNs(two), one, undefined, unit);
  }
  function differenceZoned(ns1, ns2, timeZone, largestUnit) {
    if (ns1 === ns2) return timeOnly(0n);
    const start = getDateTimeFor(timeZone, ns1);
    const end = getDateTimeFor(timeZone, ns2);
    const sign = ns2 < ns1 ? -1 : 1;
    const maxCorrection = sign === 1 ? 2 : 1;
    let correction = 0;
    let time = ToBigInt(end.t - start.t);
    if ((time < 0n ? -1 : time > 0n ? 1 : 0) === -sign) correction++;
    let intermediate = end;
    let success = false;
    while (correction <= maxCorrection && !success) {
      intermediate = balanceDate(end.year, end.month, end.day - correction * sign);
      const intermediateNs = getEpochNsFor(timeZone, combine(intermediate, start.t), 'compatible');
      time = ns2 - intermediateNs;
      const timeSign = time < 0n ? -1 : time > 0n ? 1 : 0;
      if (sign !== -timeSign) success = true;
      correction++;
    }
    const difference = dateUntil(start, intermediate, largerUnit(largestUnit, 'day'));
    difference.time = time;
    return difference;
  }
  function differenceInstant(ns1, ns2, increment, smallestUnit, mode) {
    return timeOnly(roundTimeDuration(ns2 - ns1, increment, smallestUnit, mode));
  }
  function differenceZonedWithRounding(ns1, ns2, timeZone, largestUnit, increment, smallestUnit, mode) {
    if (!isDateUnit(largestUnit)) return differenceInstant(ns1, ns2, increment, smallestUnit, mode);
    const difference = differenceZoned(ns1, ns2, timeZone, largestUnit);
    if (smallestUnit === 'nanosecond' && increment === 1) return difference;
    return roundRelativeDuration(difference, ns2, getDateTimeFor(timeZone, ns1), timeZone, largestUnit, increment, smallestUnit, mode);
  }
  function differenceZonedWithTotal(ns1, ns2, timeZone, unit) {
    if (!isDateUnit(unit)) return totalTimeDuration(ns2 - ns1, unit);
    const difference = differenceZoned(ns1, ns2, timeZone, unit);
    return totalRelativeDuration(difference, ns2, getDateTimeFor(timeZone, ns1), timeZone, unit);
  }
  // Fenêtre d'arrondi d'une unité de calendrier : les deux durées qui
  // encadrent `duration`, et leurs instants. `shift` la décale d'un cran.
  function nudgeWindow(sign, duration, dateTime, timeZone, increment, unit, shift) {
    const step = increment * sign;
    let lower;
    let startDuration;
    let endDuration;
    if (unit === 'year') {
      lower = roundNumber(duration.years, increment, 'trunc') + shift * step;
      startDuration = dateDuration(lower, 0, 0, 0);
      endDuration = dateDuration(lower + step, 0, 0, 0);
    } else if (unit === 'month') {
      lower = roundNumber(duration.months, increment, 'trunc') + shift * step;
      startDuration = dateDuration(duration.years, lower, 0, 0);
      endDuration = dateDuration(duration.years, lower + step, 0, 0);
    } else if (unit === 'week') {
      const weeksStart = dateAdd(dateTime, dateDuration(duration.years, duration.months, 0, 0), 'constrain');
      const weeksEnd = balanceDate(weeksStart.year, weeksStart.month, weeksStart.day + duration.days);
      const until = dateUntil(weeksStart, weeksEnd, 'week');
      lower = roundNumber(duration.weeks + until.weeks, increment, 'trunc') + shift * step;
      startDuration = dateDuration(duration.years, duration.months, lower, 0);
      endDuration = dateDuration(duration.years, duration.months, lower + step, 0);
    } else {
      lower = roundNumber(duration.days, increment, 'trunc') + shift * step;
      startDuration = dateDuration(duration.years, duration.months, duration.weeks, lower);
      endDuration = dateDuration(duration.years, duration.months, duration.weeks, lower + step);
    }
    const start = dateAdd(dateTime, startDuration, 'constrain');
    const end = dateAdd(dateTime, endDuration, 'constrain');
    return {
      lower,
      startDuration,
      endDuration,
      startNs: epochNsOf(combine(start, dateTime.t), timeZone),
      endNs: epochNsOf(combine(end, dateTime.t), timeZone),
    };
  }
  function nudgeToCalendarUnit(sign, duration, destEpochNs, dateTime, timeZone, increment, unit, mode) {
    let window = nudgeWindow(sign, duration, dateTime, timeZone, increment, unit, 0);
    // La destination peut dépasser la fenêtre quand le jour de départ
    // n'existe pas dans le mois d'arrivée : la fenêtre suivante la contient.
    if (sign > 0 ? destEpochNs > window.endNs : destEpochNs < window.endNs) {
      window = nudgeWindow(sign, duration, dateTime, timeZone, increment, unit, 1);
    }
    const lower = window.lower;
    const startDuration = window.startDuration;
    const endDuration = window.endDuration;
    const startNs = window.startNs;
    const endNs = window.endNs;
    const numerator = destEpochNs - startNs;
    const denominator = endNs - startNs;
    if (denominator === 0n) throw rangeError('intervalle d’arrondi vide');
    const total = divideToNumber(ToBigInt(lower) * denominator + numerator * ToBigInt(increment * sign), denominator);
    let expand;
    if (numerator === denominator) {
      expand = true;
    } else if (numerator === 0n) {
      expand = false;
    } else {
      const twice = (numerator < 0n ? -numerator : numerator) * 2n;
      const span = denominator < 0n ? -denominator : denominator;
      const half = twice < span ? -1 : twice > span ? 1 : 0;
      expand = roundsAway(unsignedRoundingMode(mode, sign < 0), half, (MathAbs(lower) / increment) % 2 === 0);
    }
    return { duration: expand ? endDuration : startDuration, nudgedEpochNs: expand ? endNs : startNs, didExpand: expand, total };
  }
  function nudgeToZonedTime(sign, duration, dateTime, timeZone, increment, unit, mode) {
    const start = dateAdd(dateTime, dateDuration(duration.years, duration.months, duration.weeks, duration.days), 'constrain');
    const end = balanceDate(start.year, start.month, start.day + sign);
    const startNs = getEpochNsFor(timeZone, combine(start, dateTime.t), 'compatible');
    const endNs = getEpochNsFor(timeZone, combine(end, dateTime.t), 'compatible');
    const daySpan = endNs - startNs;
    const step = ToBigInt(increment) * ToBigInt(UNIT_NS[unit]);
    let rounded = roundBig(duration.time, step, mode);
    const beyond = rounded - daySpan;
    const beyondSign = beyond < 0n ? -1 : beyond > 0n ? 1 : 0;
    let didExpand = false;
    let dayDelta = 0;
    let nudged;
    if (beyondSign !== -sign) {
      didExpand = true;
      dayDelta = sign;
      rounded = roundBig(beyond, step, mode);
      nudged = rounded + endNs;
    } else {
      nudged = rounded + startNs;
    }
    return {
      duration: { years: duration.years, months: duration.months, weeks: duration.weeks, days: duration.days + dayDelta, time: rounded },
      nudgedEpochNs: nudged,
      didExpand,
    };
  }
  function nudgeToDayOrTime(duration, destEpochNs, largestUnit, increment, smallestUnit, mode) {
    const time = add24HourDays(duration.time, duration.days);
    const rounded = checkTimeDuration(roundBig(time, ToBigInt(increment) * ToBigInt(UNIT_NS[smallestUnit]), mode));
    const wholeDays = time / NS_DAY_BIG;
    const roundedWholeDays = rounded / NS_DAY_BIG;
    const delta = roundedWholeDays - wholeDays;
    const deltaSign = delta < 0n ? -1 : delta > 0n ? 1 : 0;
    const timeSign = time < 0n ? -1 : time > 0n ? 1 : 0;
    let days = 0;
    let remainder = rounded;
    if (isDateUnit(largestUnit)) {
      days = ToNumber(roundedWholeDays);
      remainder = rounded - roundedWholeDays * NS_DAY_BIG;
    }
    return {
      duration: { years: duration.years, months: duration.months, weeks: duration.weeks, days, time: remainder },
      nudgedEpochNs: rounded - time + destEpochNs,
      didExpand: deltaSign === timeSign,
    };
  }
  function bubbleRelativeDuration(sign, duration, nudgedEpochNs, dateTime, timeZone, largestUnit, smallestUnit) {
    if (smallestUnit === largestUnit) return duration;
    const largestIndex = UNIT_INDEX[largestUnit];
    for (let index = UNIT_INDEX[smallestUnit] - 1; index >= largestIndex; index--) {
      const unit = UNITS[index];
      if (unit === 'week' && largestUnit !== 'week') continue;
      let endDuration;
      if (unit === 'year') {
        endDuration = dateDuration(duration.years + sign, 0, 0, 0);
      } else if (unit === 'month') {
        endDuration = dateDuration(duration.years, duration.months + sign, 0, 0);
      } else {
        endDuration = dateDuration(duration.years, duration.months, duration.weeks + sign, 0);
      }
      const end = dateAdd(dateTime, endDuration, 'constrain');
      const endNs = epochNsOf(combine(end, dateTime.t), timeZone);
      const beyond = nudgedEpochNs - endNs;
      const beyondSign = beyond < 0n ? -1 : beyond > 0n ? 1 : 0;
      if (beyondSign === -sign) break;
      duration = endDuration;
    }
    return duration;
  }
  function roundRelativeDuration(duration, destEpochNs, dateTime, timeZone, largestUnit, increment, smallestUnit, mode) {
    const irregular = isCalendarUnit(smallestUnit) || (timeZone !== undefined && smallestUnit === 'day');
    const sign = internalSign(duration) < 0 ? -1 : 1;
    let nudge;
    if (irregular) {
      nudge = nudgeToCalendarUnit(sign, duration, destEpochNs, dateTime, timeZone, increment, smallestUnit, mode);
    } else if (timeZone !== undefined) {
      nudge = nudgeToZonedTime(sign, duration, dateTime, timeZone, increment, smallestUnit, mode);
    } else {
      nudge = nudgeToDayOrTime(duration, destEpochNs, largestUnit, increment, smallestUnit, mode);
    }
    duration = nudge.duration;
    if (nudge.didExpand && smallestUnit !== 'week') {
      duration = bubbleRelativeDuration(sign, duration, nudge.nudgedEpochNs, dateTime, timeZone, largestUnit, largerUnit(smallestUnit, 'day'));
    }
    return duration;
  }
  function totalRelativeDuration(duration, destEpochNs, dateTime, timeZone, unit) {
    if (isCalendarUnit(unit) || (timeZone !== undefined && unit === 'day')) {
      const sign = internalSign(duration) < 0 ? -1 : 1;
      return nudgeToCalendarUnit(sign, duration, destEpochNs, dateTime, timeZone, 1, unit, 'trunc').total;
    }
    return totalTimeDuration(add24HourDays(duration.time, duration.days), unit);
  }
  function addZoned(ns, timeZone, duration, overflow) {
    if (duration.years === 0 && duration.months === 0 && duration.weeks === 0 && duration.days === 0) return addInstant(ns, duration.time);
    const dateTime = getDateTimeFor(timeZone, ns);
    const added = combine(dateAdd(dateTime, duration, overflow), dateTime.t);
    if (!dateTimeWithinLimits(added)) throw rangeError('date hors limites');
    return addInstant(getEpochNsFor(timeZone, added, 'compatible'), duration.time);
  }
  function addInstant(ns, time) {
    const result = ns + time;
    if (!isValidEpochNs(result)) throw rangeError('instant hors limites');
    return result;
  }
  // Lit `relativeTo` : `{ plain, zoned }`, l'un des deux au plus étant défini.
  function getRelativeToOption(options) {
    const value = options.relativeTo;
    if (value === undefined) return { plain: undefined, zoned: undefined };
    let offsetBehaviour = 'option';
    let matchMinutes = false;
    let dateTime;
    let timeZone;
    let offsetString;
    let calendar;
    if (isObject(value)) {
      if (isZoned(value)) return { plain: undefined, zoned: zonedSlot(value) };
      if (isPlainDate(value)) return { plain: dateSlot(value), zoned: undefined };
      if (isPlainDateTime(value)) {
        const slot = dateTimeSlot(value);
        return { plain: { year: slot.year, month: slot.month, day: slot.day, calendar: slot.calendar }, zoned: undefined };
      }
      calendar = getCalendarWithISODefault(value);
      const fields = prepareFields(value, ZONED_FIELDS, NONE);
      dateTime = interpretDateTimeFields(fields, 'constrain');
      timeZone = fields.timeZone;
      offsetString = fields.offset;
      if (offsetString === undefined) offsetBehaviour = 'wall';
    } else {
      if (typeof value !== 'string') throw typeError('relativeTo doit être une date ou une chaîne');
      const parsed = parseISO(value, 'relative');
      offsetString = parsed.offset;
      if (parsed.timeZone !== undefined) timeZone = toTimeZoneIdentifier(parsed.timeZone);
      if (parsed.z) {
        offsetBehaviour = 'exact';
      } else if (offsetString === undefined) {
        offsetBehaviour = 'wall';
      }
      matchMinutes = !parsed.offsetHasSeconds;
      calendar = canonicalizeCalendar(parsed.calendar === undefined ? 'iso8601' : parsed.calendar);
      dateTime = parsed;
    }
    if (timeZone === undefined) {
      if (!dateWithinLimits(dateTime)) throw rangeError('date hors limites');
      return { plain: { year: dateTime.year, month: dateTime.month, day: dateTime.day, calendar }, zoned: undefined };
    }
    const offsetNs = offsetBehaviour === 'option' ? parseOffsetString(offsetString) : 0;
    const ns = interpretOffset(dateTime, dateTime.t, offsetBehaviour, offsetNs, timeZone, 'compatible', 'reject', matchMinutes);
    return { plain: undefined, zoned: { ns, timeZone, calendar } };
  }
  function durationToString(f, precision) {
    const sign = durationSign(f);
    const whole = (value) => `${ToBigInt(MathAbs(value))}`;
    let datePart = '';
    if (f[0] !== 0) datePart += whole(f[0]) + 'Y';
    if (f[1] !== 0) datePart += whole(f[1]) + 'M';
    if (f[2] !== 0) datePart += whole(f[2]) + 'W';
    if (f[3] !== 0) datePart += whole(f[3]) + 'D';
    let timePart = '';
    if (f[4] !== 0) timePart += whole(f[4]) + 'H';
    if (f[5] !== 0) timePart += whole(f[5]) + 'M';
    const zeroMinutesAndHigher = UNIT_INDEX[defaultLargestUnit(f)] >= 6;
    let seconds = ToBigInt(f[6]) * 1000000000n + ToBigInt(f[7]) * 1000000n + ToBigInt(f[8]) * 1000n + ToBigInt(f[9]);
    if (seconds !== 0n || zeroMinutesAndHigher || precision !== 'auto') {
      if (seconds < 0n) seconds = -seconds;
      timePart += `${seconds / 1000000000n}` + formatFraction(ToNumber(seconds % 1000000000n), precision) + 'S';
    }
    let result = (sign < 0 ? '-' : '') + 'P' + datePart;
    if (timePart !== '') result += 'T' + timePart;
    return result;
  }

  let durationSlot;
  let isDuration;
  class Duration {
    #s;
    constructor(years = 0, months = 0, weeks = 0, days = 0, hours = 0, minutes = 0, seconds = 0, milliseconds = 0, microseconds = 0, nanoseconds = 0) {
      const fields = [
        toIntegerIfIntegral(years), toIntegerIfIntegral(months), toIntegerIfIntegral(weeks), toIntegerIfIntegral(days),
        toIntegerIfIntegral(hours), toIntegerIfIntegral(minutes), toIntegerIfIntegral(seconds), toIntegerIfIntegral(milliseconds),
        toIntegerIfIntegral(microseconds), toIntegerIfIntegral(nanoseconds),
      ];
      if (!isValidDuration(fields)) throw rangeError('durée invalide');
      this.#s = fields;
    }
    static {
      durationSlot = (object) => object.#s;
      isDuration = (object) => #s in object;
    }
    static from(item) {
      return durationFromFields(toDurationFields(item));
    }
    static compare(one, two, options = undefined) {
      const f1 = toDurationFields(one);
      const f2 = toDurationFields(two);
      const relativeTo = getRelativeToOption(getOptionsObject(options));
      let same = true;
      for (let i = 0; i < 10; i++) {
        if (f1[i] !== f2[i]) same = false;
      }
      if (same) return 0;
      const largest1 = defaultLargestUnit(f1);
      const largest2 = defaultLargestUnit(f2);
      const duration1 = toInternal(f1);
      const duration2 = toInternal(f2);
      const compareBig = (a, b) => (a < b ? -1 : a > b ? 1 : 0);
      if (relativeTo.zoned !== undefined && (isDateUnit(largest1) || isDateUnit(largest2))) {
        const zoned = relativeTo.zoned;
        const after1 = addZoned(zoned.ns, zoned.timeZone, duration1, 'constrain');
        const after2 = addZoned(zoned.ns, zoned.timeZone, duration2, 'constrain');
        return compareBig(after1, after2);
      }
      let days1 = f1[3];
      let days2 = f2[3];
      if (isCalendarUnit(largest1) || isCalendarUnit(largest2)) {
        if (relativeTo.plain === undefined) throw rangeError('relativeTo requis pour comparer des années, mois ou semaines');
        const dateDays = (duration) => {
          if (duration.years === 0 && duration.months === 0 && duration.weeks === 0) return duration.days;
          const later = dateAdd(relativeTo.plain, dateDuration(duration.years, duration.months, duration.weeks, 0), 'constrain');
          return duration.days + dateEpochDays(later) - dateEpochDays(relativeTo.plain);
        };
        days1 = dateDays(duration1);
        days2 = dateDays(duration2);
      }
      return compareBig(add24HourDays(duration1.time, days1), add24HourDays(duration2.time, days2));
    }
    get years() {
      return this.#s[0];
    }
    get months() {
      return this.#s[1];
    }
    get weeks() {
      return this.#s[2];
    }
    get days() {
      return this.#s[3];
    }
    get hours() {
      return this.#s[4];
    }
    get minutes() {
      return this.#s[5];
    }
    get seconds() {
      return this.#s[6];
    }
    get milliseconds() {
      return this.#s[7];
    }
    get microseconds() {
      return this.#s[8];
    }
    get nanoseconds() {
      return this.#s[9];
    }
    get sign() {
      return durationSign(this.#s);
    }
    get blank() {
      return durationSign(this.#s) === 0;
    }
    with(temporalDurationLike) {
      const f = this.#s;
      const partial = toPartialDuration(temporalDurationLike);
      for (let i = 0; i < 10; i++) {
        if (partial[i] === undefined) partial[i] = f[i];
      }
      return durationFromFields(partial);
    }
    negated() {
      return durationFromFields(negatedFields(this.#s));
    }
    abs() {
      const f = this.#s;
      const out = [];
      for (let i = 0; i < 10; i++) out[i] = MathAbs(f[i]);
      return durationFromFields(out);
    }
    add(other) {
      return addDurations(this.#s, toDurationFields(other));
    }
    subtract(other) {
      return addDurations(this.#s, negatedFields(toDurationFields(other)));
    }
    round(roundTo) {
      const f = this.#s;
      if (roundTo === undefined) throw typeError('options d’arrondi requises');
      if (typeof roundTo === 'string') {
        const text = roundTo;
        roundTo = ObjectCreate(null);
        roundTo.smallestUnit = text;
      } else {
        roundTo = getOptionsObject(roundTo);
      }
      let smallestUnitPresent = true;
      let largestUnitPresent = true;
      let largestUnit = getUnitOption(roundTo, 'largestUnit', undefined);
      const relativeTo = getRelativeToOption(roundTo);
      const increment = getRoundingIncrementOption(roundTo);
      const mode = getRoundingModeOption(roundTo, 'halfExpand');
      let smallestUnit = getUnitOption(roundTo, 'smallestUnit', undefined);
      validateUnit(smallestUnit, 'datetime', undefined);
      if (smallestUnit === undefined) {
        smallestUnitPresent = false;
        smallestUnit = 'nanosecond';
      }
      const existingLargest = defaultLargestUnit(f);
      const defaultLargest = largerUnit(existingLargest, smallestUnit);
      if (largestUnit === undefined) {
        largestUnitPresent = false;
        largestUnit = defaultLargest;
      } else if (largestUnit === 'auto') {
        largestUnit = defaultLargest;
      }
      if (!smallestUnitPresent && !largestUnitPresent) throw rangeError('smallestUnit ou largestUnit requis');
      if (largerUnit(largestUnit, smallestUnit) !== largestUnit) throw rangeError('largestUnit plus petite que smallestUnit');
      const maximum = maximumRoundingIncrement(smallestUnit);
      if (maximum !== undefined) validateRoundingIncrement(increment, maximum, false);
      if (increment > 1 && largestUnit !== smallestUnit && isDateUnit(smallestUnit)) {
        throw rangeError('roundingIncrement refusé pour cette paire d’unités');
      }
      if (relativeTo.zoned !== undefined) {
        const zoned = relativeTo.zoned;
        const target = addZoned(zoned.ns, zoned.timeZone, toInternal(f), 'constrain');
        const rounded = differenceZonedWithRounding(zoned.ns, target, zoned.timeZone, largestUnit, increment, smallestUnit, mode);
        return durationFromInternal(rounded, isDateUnit(largestUnit) ? 'hour' : largestUnit);
      }
      if (relativeTo.plain !== undefined) {
        const plain = relativeTo.plain;
        const internal = toInternal24(f);
        const targetTime = addTime(0, internal.time);
        const targetDate = dateAdd(plain, dateDuration(internal.years, internal.months, internal.weeks, targetTime.days), 'constrain');
        const rounded = differencePlainDateTimeWithRounding(combine(plain, 0), combine(targetDate, targetTime.t), largestUnit, increment, smallestUnit, mode);
        return durationFromInternal(rounded, largestUnit);
      }
      if (isCalendarUnit(existingLargest) || isCalendarUnit(largestUnit)) throw rangeError('relativeTo requis pour arrondir des années, mois ou semaines');
      const internal = toInternal24(f);
      if (smallestUnit === 'day') {
        const days = roundBig(internal.time, ToBigInt(increment) * NS_DAY_BIG, mode) / NS_DAY_BIG;
        return durationFromInternal(dateDuration(0, 0, 0, ToNumber(days)), largestUnit);
      }
      return durationFromInternal(timeOnly(roundTimeDuration(internal.time, increment, smallestUnit, mode)), largestUnit);
    }
    total(totalOf) {
      const f = this.#s;
      if (totalOf === undefined) throw typeError('unité requise');
      if (typeof totalOf === 'string') {
        const text = totalOf;
        totalOf = ObjectCreate(null);
        totalOf.unit = text;
      } else {
        totalOf = getOptionsObject(totalOf);
      }
      const relativeTo = getRelativeToOption(totalOf);
      const unit = getUnitOption(totalOf, 'unit', REQUIRED);
      validateUnit(unit, 'datetime', undefined);
      if (relativeTo.zoned !== undefined) {
        const zoned = relativeTo.zoned;
        const target = addZoned(zoned.ns, zoned.timeZone, toInternal(f), 'constrain');
        return differenceZonedWithTotal(zoned.ns, target, zoned.timeZone, unit);
      }
      if (relativeTo.plain !== undefined) {
        const plain = relativeTo.plain;
        const internal = toInternal24(f);
        const targetTime = addTime(0, internal.time);
        const targetDate = dateAdd(plain, dateDuration(internal.years, internal.months, internal.weeks, targetTime.days), 'constrain');
        return differencePlainDateTimeWithTotal(combine(plain, 0), combine(targetDate, targetTime.t), unit);
      }
      if (isCalendarUnit(defaultLargestUnit(f)) || isCalendarUnit(unit)) throw rangeError('relativeTo requis pour des années, mois ou semaines');
      return totalTimeDuration(toInternal24(f).time, unit);
    }
    toString(options = undefined) {
      const f = this.#s;
      options = getOptionsObject(options);
      const digits = getFractionalSecondDigitsOption(options);
      const mode = getRoundingModeOption(options, 'trunc');
      const smallestUnit = getUnitOption(options, 'smallestUnit', undefined);
      validateUnit(smallestUnit, 'time', undefined);
      if (smallestUnit === 'hour' || smallestUnit === 'minute') throw rangeError('smallestUnit trop grande');
      const precision = toSecondsStringPrecision(smallestUnit, digits);
      if (precision.unit === 'nanosecond' && precision.increment === 1) return durationToString(f, precision.precision);
      const internal = toInternal(f);
      internal.time = roundTimeDuration(internal.time, precision.increment, precision.unit, mode);
      const rounded = durationFromInternal(internal, largerUnit(defaultLargestUnit(f), 'second'));
      return durationToString(durationSlot(rounded), precision.precision);
    }
    toJSON() {
      return durationToString(this.#s, 'auto');
    }
    toLocaleString(locales = undefined, options = undefined) {
      return durationToString(this.#s, 'auto');
    }
    valueOf() {
      throw typeError('une durée Temporal ne se convertit pas en primitive : utiliser Temporal.Duration.compare');
    }
  }
  function addDurations(f1, f2) {
    const largestUnit = largerUnit(defaultLargestUnit(f1), defaultLargestUnit(f2));
    if (isCalendarUnit(largestUnit)) throw rangeError('années, mois et semaines ne s’additionnent pas sans date de référence');
    const one = toInternal24(f1);
    const two = toInternal24(f2);
    return durationFromInternal(timeOnly(checkTimeDuration(one.time + two.time)), largestUnit);
  }
