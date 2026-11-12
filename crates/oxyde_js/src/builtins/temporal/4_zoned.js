  // Temporal, quatrième partie : fuseaux horaires, instants, dates avec
  // fuseau, horloge, et assemblage de l'objet `Temporal`.

  // ---- Fuseaux horaires ----
  //
  // Un fuseau est soit `UTC`, soit un décalage fixe `±HH:MM`.

  function parseTimeZoneIdentifier(identifier) {
    const offset = readOffset(identifier, 0, false);
    if (offset !== null && offset.end === identifier.length) return { name: undefined, offsetMinutes: offset.ns / 60000000000 };
    if (isTimeZoneName(identifier)) return { name: identifier, offsetMinutes: undefined };
    throw rangeError(`fuseau horaire invalide : ${identifier}`);
  }
  function namedTimeZone(name) {
    if (name.toUpperCase() === 'UTC') return 'UTC';
    throw rangeError(`fuseau horaire inconnu : ${name}`);
  }
  function canonicalTimeZone(parsed) {
    if (parsed.offsetMinutes !== undefined) return formatOffsetMinutes(parsed.offsetMinutes);
    return namedTimeZone(parsed.name);
  }
  function toTimeZoneIdentifier(like) {
    if (isObject(like) && isZoned(like)) return zonedSlot(like).timeZone;
    if (typeof like !== 'string') throw typeError('fuseau horaire attendu');
    const offset = readOffset(like, 0, false);
    if ((offset !== null && offset.end === like.length) || isTimeZoneName(like)) return canonicalTimeZone(parseTimeZoneIdentifier(like));
    const parsed = parseISO(like, 'any');
    if (parsed.timeZone !== undefined) return canonicalTimeZone(parseTimeZoneIdentifier(parsed.timeZone));
    if (parsed.z) return 'UTC';
    if (parsed.offset !== undefined) return canonicalTimeZone(parseTimeZoneIdentifier(parsed.offset));
    throw rangeError(`fuseau horaire absent de la chaîne : ${like}`);
  }
  function offsetNsFor(timeZone, ns) {
    const first = timeZone.charCodeAt(0);
    return first === 43 || first === 45 ? readOffset(timeZone, 0, false).ns : 0;
  }
  function getDateTimeFor(timeZone, ns) {
    return dateTimeFromEpochNs(ns, offsetNsFor(timeZone, ns));
  }
  function getPossibleEpochNs(timeZone, dateTime) {
    const shifted = shiftDateTime(dateTime, -offsetNsFor(timeZone, 0n));
    checkDaysRange(shifted);
    const ns = utcEpochNs(shifted);
    if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
    return [ns];
  }
  function getEpochNsFor(timeZone, dateTime, disambiguation) {
    return getPossibleEpochNs(timeZone, dateTime)[0];
  }
  function getStartOfDay(timeZone, date) {
    return getPossibleEpochNs(timeZone, combine(date, 0))[0];
  }
  // `t` nul signifie « début du jour » ; `behaviour` vaut 'option', 'exact'
  // ou 'wall'.
  function interpretOffset(date, t, behaviour, offsetNs, timeZone, disambiguation, offsetOption, matchMinutes) {
    if (t === null) return getStartOfDay(timeZone, date);
    const dateTime = combine(date, t);
    if (behaviour === 'wall' || (behaviour === 'option' && offsetOption === 'ignore')) {
      return getEpochNsFor(timeZone, dateTime, disambiguation);
    }
    if (behaviour === 'exact' || (behaviour === 'option' && offsetOption === 'use')) {
      const balanced = shiftDateTime(dateTime, -offsetNs);
      checkDaysRange(balanced);
      const ns = utcEpochNs(balanced);
      if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
      return ns;
    }
    checkDaysRange(dateTime);
    const utc = utcEpochNs(dateTime);
    const possible = getPossibleEpochNs(timeZone, dateTime);
    for (let i = 0; i < possible.length; i++) {
      const candidateOffset = ToNumber(utc - possible[i]);
      if (candidateOffset === offsetNs) return possible[i];
      if (matchMinutes && roundNumber(candidateOffset, 60000000000, 'halfExpand') === offsetNs) return possible[i];
    }
    if (offsetOption === 'reject') throw rangeError('le décalage ne correspond pas au fuseau horaire');
    return possible[0];
  }
  function toPrimitiveNumber(value) {
    if (!isObject(value)) return value;
    const exotic = value[SymbolToPrimitive];
    if (exotic !== undefined && exotic !== null) {
      if (typeof exotic !== 'function') throw typeError('@@toPrimitive doit être une fonction');
      const result = ReflectApply(exotic, value, ['number']);
      if (isObject(result)) throw typeError('conversion en primitive impossible');
      return result;
    }
    const names = ['valueOf', 'toString'];
    for (let i = 0; i < 2; i++) {
      const method = value[names[i]];
      if (typeof method === 'function') {
        const result = ReflectApply(method, value, []);
        if (!isObject(result)) return result;
      }
    }
    throw typeError('conversion en primitive impossible');
  }
  // L'opération abstraite ToBigInt : un nombre est refusé.
  function toBigIntStrict(value) {
    const primitive = toPrimitiveNumber(value);
    switch (typeof primitive) {
      case 'bigint': return primitive;
      case 'boolean': return primitive ? 1n : 0n;
      case 'string': return ToBigInt(primitive);
      default: throw typeError('grand entier attendu');
    }
  }
  function floorDivide(value, divisor) {
    const quotient = value / divisor;
    return value % divisor < 0n ? quotient - 1n : quotient;
  }

  // ---- Instant ----

  function toInstantNs(item) {
    if (isObject(item)) {
      if (isInstant(item)) return instantSlot(item);
      if (isZoned(item)) return zonedSlot(item).ns;
      item = toPrimitiveString(item);
    }
    if (typeof item !== 'string') throw typeError('instant attendu');
    const parsed = parseISO(item, 'instant');
    const offsetNs = parsed.z ? 0 : parseOffsetString(parsed.offset);
    const balanced = shiftDateTime(combine(parsed, parsed.t), -offsetNs);
    checkDaysRange(balanced);
    const ns = utcEpochNs(balanced);
    if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
    return ns;
  }
  function roundInstant(ns, increment, unit, mode) {
    return roundBigAsIfPositive(ns, ToBigInt(increment * UNIT_NS[unit]), mode);
  }
  function instantToString(ns, timeZone, precision) {
    const zone = timeZone === undefined ? 'UTC' : timeZone;
    const text = formatDateTime(getDateTimeFor(zone, ns), 'iso8601', precision, 'never');
    return text + (timeZone === undefined ? 'Z' : formatOffsetRounded(offsetNsFor(zone, ns)));
  }
  function addToInstant(sign, ns, like) {
    let f = toDurationFields(like);
    if (sign < 0) f = negatedFields(f);
    if (isDateUnit(defaultLargestUnit(f))) throw rangeError('seules des unités d’heure s’ajoutent à un instant');
    return new Instant(addInstant(ns, toInternal24(f).time));
  }
  function differenceInstants(since, ns, other, options) {
    const target = toInstantNs(other);
    const settings = getDifferenceSettings(since, getOptionsObject(options), 'time', NONE, 'nanosecond', 'second');
    const duration = differenceInstant(ns, target, settings.roundingIncrement, settings.smallestUnit, settings.roundingMode);
    const result = durationFromInternal(duration, settings.largestUnit);
    return since ? negateDuration(result) : result;
  }

  let instantSlot;
  let isInstant;
  class Instant {
    #s;
    constructor(epochNanoseconds) {
      const ns = toBigIntStrict(epochNanoseconds);
      if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
      this.#s = ns;
    }
    static {
      instantSlot = (object) => object.#s;
      isInstant = (object) => #s in object;
    }
    static from(item) {
      return new Instant(toInstantNs(item));
    }
    static fromEpochMilliseconds(epochMilliseconds) {
      const ns = ToBigInt(+epochMilliseconds) * 1000000n;
      if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
      return new Instant(ns);
    }
    static fromEpochNanoseconds(epochNanoseconds) {
      const ns = toBigIntStrict(epochNanoseconds);
      if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
      return new Instant(ns);
    }
    static compare(one, two) {
      const ns1 = toInstantNs(one);
      const ns2 = toInstantNs(two);
      return ns1 < ns2 ? -1 : ns1 > ns2 ? 1 : 0;
    }
    get epochMilliseconds() {
      return ToNumber(floorDivide(this.#s, 1000000n));
    }
    get epochNanoseconds() {
      return this.#s;
    }
    add(temporalDurationLike) {
      return addToInstant(1, this.#s, temporalDurationLike);
    }
    subtract(temporalDurationLike) {
      return addToInstant(-1, this.#s, temporalDurationLike);
    }
    until(other, options = undefined) {
      return differenceInstants(false, this.#s, other, options);
    }
    since(other, options = undefined) {
      return differenceInstants(true, this.#s, other, options);
    }
    round(roundTo) {
      const ns = this.#s;
      roundTo = requireRoundTo(roundTo);
      const increment = getRoundingIncrementOption(roundTo);
      const mode = getRoundingModeOption(roundTo, 'halfExpand');
      const smallestUnit = getUnitOption(roundTo, 'smallestUnit', REQUIRED);
      validateUnit(smallestUnit, 'time', undefined);
      validateRoundingIncrement(increment, NS_DAY / UNIT_NS[smallestUnit], true);
      return new Instant(roundInstant(ns, increment, smallestUnit, mode));
    }
    equals(other) {
      const ns = this.#s;
      return toInstantNs(other) === ns;
    }
    toString(options = undefined) {
      const ns = this.#s;
      options = getOptionsObject(options);
      const digits = getFractionalSecondDigitsOption(options);
      const mode = getRoundingModeOption(options, 'trunc');
      const smallestUnit = getUnitOption(options, 'smallestUnit', undefined);
      let timeZone = options.timeZone;
      validateUnit(smallestUnit, 'time', undefined);
      if (smallestUnit === 'hour') throw rangeError('smallestUnit trop grande');
      if (timeZone !== undefined) timeZone = toTimeZoneIdentifier(timeZone);
      const precision = toSecondsStringPrecision(smallestUnit, digits);
      const rounded = roundInstant(ns, precision.increment, precision.unit, mode);
      if (!isValidEpochNs(rounded)) throw rangeError('instant hors limites');
      return instantToString(rounded, timeZone, precision.precision);
    }
    toLocaleString(locales = undefined, options = undefined) {
      return instantToString(this.#s, undefined, 'auto');
    }
    toJSON() {
      return instantToString(this.#s, undefined, 'auto');
    }
    valueOf() {
      throw typeError('un instant Temporal ne se convertit pas en primitive : utiliser Temporal.Instant.compare');
    }
    toZonedDateTimeISO(timeZone) {
      const ns = this.#s;
      return makeZoned(ns, toTimeZoneIdentifier(timeZone), 'iso8601');
    }
  }

  // ---- ZonedDateTime ----

  function makeZoned(ns, timeZone, calendar) {
    return new ZonedDateTime(ns, timeZone, calendar);
  }
  function toZoned(item, options) {
    let offsetBehaviour = 'option';
    let matchMinutes = false;
    let dateTime;
    let timeZone;
    let offsetString;
    let calendar;
    let disambiguation;
    let offsetOption;
    if (isObject(item)) {
      if (isZoned(item)) {
        options = getOptionsObject(options);
        getDisambiguationOption(options);
        getOffsetOption(options, 'reject');
        getOverflowOption(options);
        return zonedSlot(item);
      }
      calendar = getCalendarWithISODefault(item);
      const fields = prepareFields(item, ZONED_FIELDS, ['timeZone']);
      timeZone = fields.timeZone;
      offsetString = fields.offset;
      if (offsetString === undefined) offsetBehaviour = 'wall';
      options = getOptionsObject(options);
      disambiguation = getDisambiguationOption(options);
      offsetOption = getOffsetOption(options, 'reject');
      const overflow = getOverflowOption(options);
      dateTime = interpretDateTimeFields(fields, overflow);
    } else {
      if (typeof item !== 'string') throw typeError('date avec fuseau attendue');
      const parsed = parseISO(item, 'zoned');
      timeZone = toTimeZoneIdentifier(parsed.timeZone);
      offsetString = parsed.offset;
      if (parsed.z) {
        offsetBehaviour = 'exact';
      } else if (offsetString === undefined) {
        offsetBehaviour = 'wall';
      }
      calendar = canonicalizeCalendar(parsed.calendar === undefined ? 'iso8601' : parsed.calendar);
      matchMinutes = !parsed.offsetHasSeconds;
      options = getOptionsObject(options);
      disambiguation = getDisambiguationOption(options);
      offsetOption = getOffsetOption(options, 'reject');
      getOverflowOption(options);
      dateTime = parsed;
    }
    const offsetNs = offsetBehaviour === 'option' ? parseOffsetString(offsetString) : 0;
    const ns = interpretOffset(dateTime, dateTime.t, offsetBehaviour, offsetNs, timeZone, disambiguation, offsetOption, matchMinutes);
    return { ns, timeZone, calendar };
  }
  function addToZoned(sign, slot, like, options) {
    let f = toDurationFields(like);
    if (sign < 0) f = negatedFields(f);
    const overflow = getOverflowOption(getOptionsObject(options));
    return makeZoned(addZoned(slot.ns, slot.timeZone, toInternal(f), overflow), slot.timeZone, slot.calendar);
  }
  function differenceZonedDateTimes(since, slot, other, options) {
    const target = toZoned(other, undefined);
    if (slot.calendar !== target.calendar) throw rangeError('calendriers différents');
    const settings = getDifferenceSettings(since, getOptionsObject(options), 'datetime', NONE, 'nanosecond', 'hour');
    let result;
    if (!isDateUnit(settings.largestUnit)) {
      const duration = differenceInstant(slot.ns, target.ns, settings.roundingIncrement, settings.smallestUnit, settings.roundingMode);
      result = durationFromInternal(duration, settings.largestUnit);
    } else {
      if (slot.timeZone !== target.timeZone) throw rangeError('fuseaux horaires différents');
      if (slot.ns === target.ns) return zeroDuration();
      const duration = differenceZonedWithRounding(
        slot.ns, target.ns, slot.timeZone, settings.largestUnit, settings.roundingIncrement, settings.smallestUnit, settings.roundingMode,
      );
      result = durationFromInternal(duration, 'hour');
    }
    return since ? negateDuration(result) : result;
  }
  function zonedToString(slot, precision, showCalendar, showTimeZone, showOffset, increment, unit, mode) {
    const ns = roundInstant(slot.ns, increment, unit, mode);
    const offsetNs = offsetNsFor(slot.timeZone, ns);
    let text = formatDateTime(dateTimeFromEpochNs(ns, offsetNs), 'iso8601', precision, 'never');
    if (showOffset !== 'never') text += formatOffsetRounded(offsetNs);
    if (showTimeZone !== 'never') text += '[' + (showTimeZone === 'critical' ? '!' : '') + slot.timeZone + ']';
    return text + formatCalendarAnnotation(slot.calendar, showCalendar);
  }

  let zonedSlot;
  let isZoned;
  let zonedLocal;
  class ZonedDateTime {
    #s;
    constructor(epochNanoseconds, timeZone, calendar = 'iso8601') {
      const ns = toBigIntStrict(epochNanoseconds);
      if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
      if (typeof timeZone !== 'string') throw typeError('le fuseau horaire doit être une chaîne');
      timeZone = canonicalTimeZone(parseTimeZoneIdentifier(timeZone));
      calendar = calendarArgument(calendar);
      this.#s = { ns, timeZone, calendar };
    }
    static {
      zonedSlot = (object) => object.#s;
      isZoned = (object) => #s in object;
      // Date et heure locales de l'instant.
      zonedLocal = (object) => {
        const slot = object.#s;
        return getDateTimeFor(slot.timeZone, slot.ns);
      };
    }
    static from(item, options = undefined) {
      const slot = toZoned(item, options);
      return makeZoned(slot.ns, slot.timeZone, slot.calendar);
    }
    static compare(one, two) {
      const ns1 = toZoned(one, undefined).ns;
      const ns2 = toZoned(two, undefined).ns;
      return ns1 < ns2 ? -1 : ns1 > ns2 ? 1 : 0;
    }
    get calendarId() {
      return this.#s.calendar;
    }
    get timeZoneId() {
      return this.#s.timeZone;
    }
    get era() {
      this.#s;
      return undefined;
    }
    get eraYear() {
      this.#s;
      return undefined;
    }
    get year() {
      return zonedLocal(this).year;
    }
    get month() {
      return zonedLocal(this).month;
    }
    get monthCode() {
      return monthCodeOf(zonedLocal(this));
    }
    get day() {
      return zonedLocal(this).day;
    }
    get hour() {
      return timeField(zonedLocal(this).t, 0);
    }
    get minute() {
      return timeField(zonedLocal(this).t, 1);
    }
    get second() {
      return timeField(zonedLocal(this).t, 2);
    }
    get millisecond() {
      return timeField(zonedLocal(this).t, 3);
    }
    get microsecond() {
      return timeField(zonedLocal(this).t, 4);
    }
    get nanosecond() {
      return timeField(zonedLocal(this).t, 5);
    }
    get epochMilliseconds() {
      return ToNumber(floorDivide(this.#s.ns, 1000000n));
    }
    get epochNanoseconds() {
      return this.#s.ns;
    }
    get dayOfWeek() {
      return dayOfWeek(zonedLocal(this));
    }
    get dayOfYear() {
      return dayOfYear(zonedLocal(this));
    }
    get weekOfYear() {
      return weekOfYear(zonedLocal(this)).week;
    }
    get yearOfWeek() {
      return weekOfYear(zonedLocal(this)).year;
    }
    get hoursInDay() {
      const slot = this.#s;
      const today = getDateTimeFor(slot.timeZone, slot.ns);
      const tomorrow = balanceDate(today.year, today.month, today.day + 1);
      const start = getStartOfDay(slot.timeZone, today);
      const end = getStartOfDay(slot.timeZone, tomorrow);
      return totalTimeDuration(end - start, 'hour');
    }
    get daysInWeek() {
      this.#s;
      return 7;
    }
    get daysInMonth() {
      const local = zonedLocal(this);
      return daysInMonth(local.year, local.month);
    }
    get daysInYear() {
      return daysInYearOf(zonedLocal(this));
    }
    get monthsInYear() {
      this.#s;
      return 12;
    }
    get inLeapYear() {
      return isLeapYear(zonedLocal(this).year);
    }
    get offsetNanoseconds() {
      const slot = this.#s;
      return offsetNsFor(slot.timeZone, slot.ns);
    }
    get offset() {
      const slot = this.#s;
      return formatOffsetNs(offsetNsFor(slot.timeZone, slot.ns));
    }
    with(temporalZonedDateTimeLike, options = undefined) {
      const slot = this.#s;
      requirePartial(temporalZonedDateTimeLike);
      const offsetNs = offsetNsFor(slot.timeZone, slot.ns);
      const local = dateTimeFromEpochNs(slot.ns, offsetNs);
      let fields = dateToFields(local, 'date');
      fields.hour = timeField(local.t, 0);
      fields.minute = timeField(local.t, 1);
      fields.second = timeField(local.t, 2);
      fields.millisecond = timeField(local.t, 3);
      fields.microsecond = timeField(local.t, 4);
      fields.nanosecond = timeField(local.t, 5);
      fields.offset = formatOffsetNs(offsetNs);
      fields = mergeFields(fields, prepareFields(temporalZonedDateTimeLike, ZONED_WITH_FIELDS, PARTIAL));
      options = getOptionsObject(options);
      const disambiguation = getDisambiguationOption(options);
      const offsetOption = getOffsetOption(options, 'prefer');
      const overflow = getOverflowOption(options);
      const dateTime = interpretDateTimeFields(fields, overflow);
      const newOffsetNs = parseOffsetString(fields.offset);
      const ns = interpretOffset(dateTime, dateTime.t, 'option', newOffsetNs, slot.timeZone, disambiguation, offsetOption, false);
      return makeZoned(ns, slot.timeZone, slot.calendar);
    }
    withPlainTime(plainTimeLike = undefined) {
      const slot = this.#s;
      const local = getDateTimeFor(slot.timeZone, slot.ns);
      let ns;
      if (plainTimeLike === undefined) {
        ns = getStartOfDay(slot.timeZone, local);
      } else {
        ns = getEpochNsFor(slot.timeZone, combine(local, toTimeNs(plainTimeLike, undefined)), 'compatible');
      }
      return makeZoned(ns, slot.timeZone, slot.calendar);
    }
    withTimeZone(timeZoneLike) {
      const slot = this.#s;
      return makeZoned(slot.ns, toTimeZoneIdentifier(timeZoneLike), slot.calendar);
    }
    withCalendar(calendarLike) {
      const slot = this.#s;
      return makeZoned(slot.ns, slot.timeZone, toCalendarIdentifier(calendarLike));
    }
    add(temporalDurationLike, options = undefined) {
      return addToZoned(1, this.#s, temporalDurationLike, options);
    }
    subtract(temporalDurationLike, options = undefined) {
      return addToZoned(-1, this.#s, temporalDurationLike, options);
    }
    until(other, options = undefined) {
      return differenceZonedDateTimes(false, this.#s, other, options);
    }
    since(other, options = undefined) {
      return differenceZonedDateTimes(true, this.#s, other, options);
    }
    round(roundTo) {
      const slot = this.#s;
      roundTo = requireRoundTo(roundTo);
      const increment = getRoundingIncrementOption(roundTo);
      const mode = getRoundingModeOption(roundTo, 'halfExpand');
      const smallestUnit = getUnitOption(roundTo, 'smallestUnit', REQUIRED);
      validateUnit(smallestUnit, 'time', 'day');
      if (smallestUnit === 'day') {
        validateRoundingIncrement(increment, 1, true);
      } else {
        validateRoundingIncrement(increment, maximumRoundingIncrement(smallestUnit), false);
      }
      if (smallestUnit === 'nanosecond' && increment === 1) return makeZoned(slot.ns, slot.timeZone, slot.calendar);
      const local = getDateTimeFor(slot.timeZone, slot.ns);
      let ns;
      if (smallestUnit === 'day') {
        const start = getStartOfDay(slot.timeZone, local);
        const end = getStartOfDay(slot.timeZone, balanceDate(local.year, local.month, local.day + 1));
        ns = roundBig(slot.ns - start, end - start, mode) + start;
        if (!isValidEpochNs(ns)) throw rangeError('instant hors limites');
      } else {
        const rounded = roundDateTime(local, increment, smallestUnit, mode);
        const offsetNs = offsetNsFor(slot.timeZone, slot.ns);
        ns = interpretOffset(rounded, rounded.t, 'option', offsetNs, slot.timeZone, 'compatible', 'prefer', false);
      }
      return makeZoned(ns, slot.timeZone, slot.calendar);
    }
    equals(other) {
      const slot = this.#s;
      const target = toZoned(other, undefined);
      return slot.ns === target.ns && slot.timeZone === target.timeZone && slot.calendar === target.calendar;
    }
    toString(options = undefined) {
      const slot = this.#s;
      options = getOptionsObject(options);
      const showCalendar = getShowCalendarOption(options);
      const digits = getFractionalSecondDigitsOption(options);
      const showOffset = getOption(options, 'offset', ['auto', 'never'], 'auto');
      const mode = getRoundingModeOption(options, 'trunc');
      const smallestUnit = getUnitOption(options, 'smallestUnit', undefined);
      const showTimeZone = getOption(options, 'timeZoneName', ['auto', 'never', 'critical'], 'auto');
      validateUnit(smallestUnit, 'time', undefined);
      if (smallestUnit === 'hour') throw rangeError('smallestUnit trop grande');
      const precision = toSecondsStringPrecision(smallestUnit, digits);
      return zonedToString(slot, precision.precision, showCalendar, showTimeZone, showOffset, precision.increment, precision.unit, mode);
    }
    toLocaleString(locales = undefined, options = undefined) {
      return zonedToString(this.#s, 'auto', 'auto', 'auto', 'auto', 1, 'nanosecond', 'trunc');
    }
    toJSON() {
      return zonedToString(this.#s, 'auto', 'auto', 'auto', 'auto', 1, 'nanosecond', 'trunc');
    }
    valueOf() {
      throw typeError('une date Temporal ne se convertit pas en primitive : utiliser Temporal.ZonedDateTime.compare');
    }
    startOfDay() {
      const slot = this.#s;
      return makeZoned(getStartOfDay(slot.timeZone, getDateTimeFor(slot.timeZone, slot.ns)), slot.timeZone, slot.calendar);
    }
    getTimeZoneTransition(directionParam) {
      this.#s;
      if (directionParam === undefined) throw typeError('direction requise');
      if (typeof directionParam === 'string') {
        const text = directionParam;
        directionParam = ObjectCreate(null);
        directionParam.direction = text;
      } else {
        directionParam = getOptionsObject(directionParam);
      }
      getOption(directionParam, 'direction', ['next', 'previous'], REQUIRED);
      // Ni UTC ni un décalage fixe ne changent jamais de décalage.
      return null;
    }
    toInstant() {
      return new Instant(this.#s.ns);
    }
    toPlainDate() {
      const slot = this.#s;
      return makeDate(getDateTimeFor(slot.timeZone, slot.ns), slot.calendar);
    }
    toPlainTime() {
      return makeTime(zonedLocal(this).t);
    }
    toPlainDateTime() {
      const slot = this.#s;
      return makeDateTime(getDateTimeFor(slot.timeZone, slot.ns), slot.calendar);
    }
  }

  // ---- Now ----

  function systemNs() {
    return ToBigInt(clockNow());
  }
  function systemDateTime(temporalTimeZoneLike) {
    const timeZone = temporalTimeZoneLike === undefined ? 'UTC' : toTimeZoneIdentifier(temporalTimeZoneLike);
    return getDateTimeFor(timeZone, systemNs());
  }
  const nowMethods = {
    timeZoneId() {
      return 'UTC';
    },
    instant() {
      return new Instant(systemNs());
    },
    plainDateTimeISO(temporalTimeZoneLike = undefined) {
      return makeDateTime(systemDateTime(temporalTimeZoneLike), 'iso8601');
    },
    zonedDateTimeISO(temporalTimeZoneLike = undefined) {
      const timeZone = temporalTimeZoneLike === undefined ? 'UTC' : toTimeZoneIdentifier(temporalTimeZoneLike);
      return makeZoned(systemNs(), timeZone, 'iso8601');
    },
    plainDateISO(temporalTimeZoneLike = undefined) {
      return makeDate(systemDateTime(temporalTimeZoneLike), 'iso8601');
    },
    plainTimeISO(temporalTimeZoneLike = undefined) {
      return makeTime(systemDateTime(temporalTimeZoneLike).t);
    },
  };

  // ---- Assemblage ----

  function hidden(target, name, value) {
    ObjectDefineProperty(target, name, { value, writable: true, enumerable: false, configurable: true });
  }
  function tag(target, name) {
    ObjectDefineProperty(target, SymbolToStringTag, { value: name, writable: false, enumerable: false, configurable: true });
  }
  function register(holder) {
    const names = ObjectGetOwnPropertyNames(holder);
    for (let i = 0; i < names.length; i++) {
      const descriptor = ObjectGetOwnPropertyDescriptor(holder, names[i]);
      if (typeof descriptor.value === 'function') natives.push(descriptor.value);
      if (typeof descriptor.get === 'function') natives.push(descriptor.get);
    }
  }
  const Temporal = {};
  const Now = {};
  const classes = [Instant, PlainDateTime, PlainDate, PlainTime, PlainYearMonth, PlainMonthDay, Duration, ZonedDateTime];
  for (let i = 0; i < classes.length; i++) {
    const constructor = classes[i];
    tag(constructor.prototype, 'Temporal.' + constructor.name);
    register(constructor);
    register(constructor.prototype);
    hidden(Temporal, constructor.name, constructor);
  }
  const nowNames = ObjectGetOwnPropertyNames(nowMethods);
  for (let i = 0; i < nowNames.length; i++) hidden(Now, nowNames[i], nowMethods[nowNames[i]]);
  register(Now);
  tag(Now, 'Temporal.Now');
  hidden(Temporal, 'Now', Now);
  tag(Temporal, 'Temporal');

  const dateValueOf = Date.prototype.valueOf;
  function toTemporalInstant(date) {
    const milliseconds = ReflectApply(dateValueOf, date, []);
    return new Instant(ToBigInt(milliseconds) * 1000000n);
  }

  return { Temporal, natives, toTemporalInstant };
})
