  // Temporal, troisième partie : heures et dates sans fuseau.

  function requireRoundTo(roundTo) {
    if (roundTo === undefined) throw typeError('options d’arrondi requises');
    if (typeof roundTo === 'string') {
      const options = ObjectCreate(null);
      options.smallestUnit = roundTo;
      return options;
    }
    return getOptionsObject(roundTo);
  }
  function requirePartial(value) {
    if (!isObject(value)) throw typeError('objet de champs attendu');
    if (isPlainDate(value) || isPlainDateTime(value) || isPlainMonthDay(value) || isPlainTime(value) || isPlainYearMonth(value) || isZoned(value)) {
      throw typeError('un objet Temporal ne peut pas servir de champs partiels');
    }
    if (value.calendar !== undefined) throw typeError('le champ calendar est refusé ici');
    if (value.timeZone !== undefined) throw typeError('le champ timeZone est refusé ici');
  }
  function negateDuration(duration) {
    return durationFromFields(negatedFields(durationSlot(duration)));
  }
  function zeroDuration() {
    return makeDuration(0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
  }
  function calendarArgument(calendar) {
    if (calendar === undefined) return 'iso8601';
    if (typeof calendar !== 'string') throw typeError('le calendrier doit être une chaîne');
    return canonicalizeCalendar(calendar);
  }
  // Arrondit une heure du jour : `{ days, t }`.
  function roundTime(t, increment, unit, mode) {
    const rounded = roundNumber(t, increment * UNIT_NS[unit], mode);
    const days = MathFloor(rounded / NS_DAY);
    return { days, t: rounded - days * NS_DAY };
  }
  function roundDateTime(dateTime, increment, unit, mode) {
    const rounded = roundTime(dateTime.t, increment, unit, mode);
    return combine(balanceDate(dateTime.year, dateTime.month, dateTime.day + rounded.days), rounded.t);
  }
  function monthCodeOf(date) {
    return 'M' + pad(date.month, 2);
  }
  function daysInYearOf(date) {
    return isLeapYear(date.year) ? 366 : 365;
  }

  // ---- PlainTime ----

  function toTimeRecord(like, partial) {
    const filler = partial ? undefined : 0;
    const record = { hour: filler, minute: filler, second: filler, millisecond: filler, microsecond: filler, nanosecond: filler };
    let any = false;
    for (let i = 0; i < 6; i++) {
      const name = TIME_FIELDS[i];
      const value = like[name];
      if (value !== undefined) {
        record[name] = toIntegerWithTruncation(value);
        any = true;
      }
    }
    if (!any) throw typeError('aucun champ d’heure reconnu');
    return record;
  }
  // Heure du jour (nanosecondes) d'une valeur donnée sous toute forme admise.
  function toTimeNs(item, options) {
    if (isObject(item)) {
      if (isPlainTime(item)) {
        getOverflowOption(getOptionsObject(options));
        return timeSlot(item);
      }
      if (isPlainDateTime(item)) {
        getOverflowOption(getOptionsObject(options));
        return dateTimeSlot(item).t;
      }
      if (isZoned(item)) {
        const zoned = zonedSlot(item);
        const dateTime = getDateTimeFor(zoned.timeZone, zoned.ns);
        getOverflowOption(getOptionsObject(options));
        return dateTime.t;
      }
      const record = toTimeRecord(item, false);
      const overflow = getOverflowOption(getOptionsObject(options));
      return regulateTime(record.hour, record.minute, record.second, record.millisecond, record.microsecond, record.nanosecond, overflow);
    }
    if (typeof item !== 'string') throw typeError('heure attendue');
    const parsed = parseISO(item, 'time');
    getOverflowOption(getOptionsObject(options));
    return parsed.t;
  }
  function toTimeOrMidnight(item) {
    return item === undefined ? 0 : toTimeNs(item, undefined);
  }
  function makeTime(t) {
    return new PlainTime(timeField(t, 0), timeField(t, 1), timeField(t, 2), timeField(t, 3), timeField(t, 4), timeField(t, 5));
  }
  function differenceTime(since, t, other, options) {
    const otherT = toTimeNs(other, undefined);
    const settings = getDifferenceSettings(since, getOptionsObject(options), 'time', NONE, 'nanosecond', 'hour');
    const time = roundTimeDuration(ToBigInt(otherT - t), settings.roundingIncrement, settings.smallestUnit, settings.roundingMode);
    const result = durationFromInternal(timeOnly(time), settings.largestUnit);
    return since ? negateDuration(result) : result;
  }

  let timeSlot;
  let isPlainTime;
  class PlainTime {
    #s;
    constructor(hour = 0, minute = 0, second = 0, millisecond = 0, microsecond = 0, nanosecond = 0) {
      hour = toIntegerWithTruncation(hour);
      minute = toIntegerWithTruncation(minute);
      second = toIntegerWithTruncation(second);
      millisecond = toIntegerWithTruncation(millisecond);
      microsecond = toIntegerWithTruncation(microsecond);
      nanosecond = toIntegerWithTruncation(nanosecond);
      if (!isValidTime(hour, minute, second, millisecond, microsecond, nanosecond)) throw rangeError('heure invalide');
      this.#s = timeToNs(hour, minute, second, millisecond, microsecond, nanosecond);
    }
    static {
      timeSlot = (object) => object.#s;
      isPlainTime = (object) => #s in object;
    }
    static from(item, options = undefined) {
      return makeTime(toTimeNs(item, options));
    }
    static compare(one, two) {
      const t1 = toTimeNs(one, undefined);
      const t2 = toTimeNs(two, undefined);
      return t1 < t2 ? -1 : t1 > t2 ? 1 : 0;
    }
    get hour() {
      return timeField(this.#s, 0);
    }
    get minute() {
      return timeField(this.#s, 1);
    }
    get second() {
      return timeField(this.#s, 2);
    }
    get millisecond() {
      return timeField(this.#s, 3);
    }
    get microsecond() {
      return timeField(this.#s, 4);
    }
    get nanosecond() {
      return timeField(this.#s, 5);
    }
    add(temporalDurationLike) {
      const t = this.#s;
      return makeTime(addTime(t, timeDurationFromFields(toDurationFields(temporalDurationLike))).t);
    }
    subtract(temporalDurationLike) {
      const t = this.#s;
      return makeTime(addTime(t, -timeDurationFromFields(toDurationFields(temporalDurationLike))).t);
    }
    with(temporalTimeLike, options = undefined) {
      const t = this.#s;
      requirePartial(temporalTimeLike);
      const partial = toTimeRecord(temporalTimeLike, true);
      const pick = (value, index) => (value === undefined ? timeField(t, index) : value);
      const overflow = getOverflowOption(getOptionsObject(options));
      return makeTime(regulateTime(
        pick(partial.hour, 0), pick(partial.minute, 1), pick(partial.second, 2), pick(partial.millisecond, 3), pick(partial.microsecond, 4),
        pick(partial.nanosecond, 5), overflow,
      ));
    }
    until(other, options = undefined) {
      return differenceTime(false, this.#s, other, options);
    }
    since(other, options = undefined) {
      return differenceTime(true, this.#s, other, options);
    }
    round(roundTo) {
      const t = this.#s;
      roundTo = requireRoundTo(roundTo);
      const increment = getRoundingIncrementOption(roundTo);
      const mode = getRoundingModeOption(roundTo, 'halfExpand');
      const smallestUnit = getUnitOption(roundTo, 'smallestUnit', REQUIRED);
      validateUnit(smallestUnit, 'time', undefined);
      validateRoundingIncrement(increment, maximumRoundingIncrement(smallestUnit), false);
      return makeTime(roundTime(t, increment, smallestUnit, mode).t);
    }
    equals(other) {
      const t = this.#s;
      return toTimeNs(other, undefined) === t;
    }
    toString(options = undefined) {
      const t = this.#s;
      options = getOptionsObject(options);
      const digits = getFractionalSecondDigitsOption(options);
      const mode = getRoundingModeOption(options, 'trunc');
      const smallestUnit = getUnitOption(options, 'smallestUnit', undefined);
      validateUnit(smallestUnit, 'time', undefined);
      if (smallestUnit === 'hour') throw rangeError('smallestUnit trop grande');
      const precision = toSecondsStringPrecision(smallestUnit, digits);
      return formatTime(roundTime(t, precision.increment, precision.unit, mode).t, precision.precision);
    }
    toLocaleString(locales = undefined, options = undefined) {
      return formatTime(this.#s, 'auto');
    }
    toJSON() {
      return formatTime(this.#s, 'auto');
    }
    valueOf() {
      throw typeError('une heure Temporal ne se convertit pas en primitive : utiliser Temporal.PlainTime.compare');
    }
  }

  // ---- PlainDate ----

  // Date et calendrier d'une valeur donnée sous toute forme admise.
  function toDate(item, options) {
    if (isObject(item)) {
      if (isPlainDate(item)) {
        getOverflowOption(getOptionsObject(options));
        return dateSlot(item);
      }
      if (isZoned(item)) {
        const zoned = zonedSlot(item);
        const dateTime = getDateTimeFor(zoned.timeZone, zoned.ns);
        getOverflowOption(getOptionsObject(options));
        return { year: dateTime.year, month: dateTime.month, day: dateTime.day, calendar: zoned.calendar };
      }
      if (isPlainDateTime(item)) {
        getOverflowOption(getOptionsObject(options));
        return dateTimeSlot(item);
      }
      const calendar = getCalendarWithISODefault(item);
      const fields = prepareFields(item, DATE_FIELDS, NONE);
      const overflow = getOverflowOption(getOptionsObject(options));
      const date = dateFromFields(fields, overflow);
      date.calendar = calendar;
      return date;
    }
    if (typeof item !== 'string') throw typeError('date attendue');
    const parsed = parseISO(item, 'datetime');
    const calendar = canonicalizeCalendar(parsed.calendar === undefined ? 'iso8601' : parsed.calendar);
    getOverflowOption(getOptionsObject(options));
    if (!dateWithinLimits(parsed)) throw rangeError('date hors limites');
    return { year: parsed.year, month: parsed.month, day: parsed.day, calendar };
  }
  function makeDate(date, calendar) {
    return new PlainDate(date.year, date.month, date.day, calendar);
  }
  function addToDate(sign, slot, like, options) {
    let f = toDurationFields(like);
    if (sign < 0) f = negatedFields(f);
    const duration = toDateDurationWithoutTime(f);
    const overflow = getOverflowOption(getOptionsObject(options));
    return makeDate(dateAdd(slot, duration, overflow), slot.calendar);
  }
  function differenceDate(since, slot, other, options) {
    const target = toDate(other, undefined);
    if (slot.calendar !== target.calendar) throw rangeError('calendriers différents');
    const settings = getDifferenceSettings(since, getOptionsObject(options), 'date', NONE, 'day', 'day');
    if (compareDate(slot, target) === 0) return zeroDuration();
    let duration = dateUntil(slot, target, settings.largestUnit);
    duration.time = 0n;
    if (settings.smallestUnit !== 'day' || settings.roundingIncrement !== 1) {
      duration = roundRelativeDuration(
        duration, utcEpochNs(combine(target, 0)), combine(slot, 0), undefined, settings.largestUnit, settings.roundingIncrement,
        settings.smallestUnit, settings.roundingMode,
      );
    }
    const result = durationFromInternal(duration, 'day');
    return since ? negateDuration(result) : result;
  }

  let dateSlot;
  let isPlainDate;
  class PlainDate {
    #s;
    constructor(isoYear, isoMonth, isoDay, calendar = 'iso8601') {
      const year = toIntegerWithTruncation(isoYear);
      const month = toIntegerWithTruncation(isoMonth);
      const day = toIntegerWithTruncation(isoDay);
      calendar = calendarArgument(calendar);
      if (!isValidDate(year, month, day)) throw rangeError('date invalide');
      const slot = { year, month, day, calendar };
      if (!dateWithinLimits(slot)) throw rangeError('date hors limites');
      this.#s = slot;
    }
    static {
      dateSlot = (object) => object.#s;
      isPlainDate = (object) => #s in object;
    }
    static from(item, options = undefined) {
      const slot = toDate(item, options);
      return makeDate(slot, slot.calendar);
    }
    static compare(one, two) {
      const first = toDate(one, undefined);
      return compareDate(first, toDate(two, undefined));
    }
    get calendarId() {
      return this.#s.calendar;
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
      return this.#s.year;
    }
    get month() {
      return this.#s.month;
    }
    get monthCode() {
      return monthCodeOf(this.#s);
    }
    get day() {
      return this.#s.day;
    }
    get dayOfWeek() {
      return dayOfWeek(this.#s);
    }
    get dayOfYear() {
      return dayOfYear(this.#s);
    }
    get weekOfYear() {
      return weekOfYear(this.#s).week;
    }
    get yearOfWeek() {
      return weekOfYear(this.#s).year;
    }
    get daysInWeek() {
      this.#s;
      return 7;
    }
    get daysInMonth() {
      const slot = this.#s;
      return daysInMonth(slot.year, slot.month);
    }
    get daysInYear() {
      return daysInYearOf(this.#s);
    }
    get monthsInYear() {
      this.#s;
      return 12;
    }
    get inLeapYear() {
      return isLeapYear(this.#s.year);
    }
    toPlainYearMonth() {
      const slot = this.#s;
      return makeYearMonth(yearMonthFromFields(dateToFields(slot, 'date'), 'constrain'), slot.calendar);
    }
    toPlainMonthDay() {
      const slot = this.#s;
      return makeMonthDay(monthDayFromFields(dateToFields(slot, 'date'), 'constrain'), slot.calendar);
    }
    add(temporalDurationLike, options = undefined) {
      return addToDate(1, this.#s, temporalDurationLike, options);
    }
    subtract(temporalDurationLike, options = undefined) {
      return addToDate(-1, this.#s, temporalDurationLike, options);
    }
    with(temporalDateLike, options = undefined) {
      const slot = this.#s;
      requirePartial(temporalDateLike);
      const fields = mergeFields(dateToFields(slot, 'date'), prepareFields(temporalDateLike, DATE_FIELDS, PARTIAL));
      const overflow = getOverflowOption(getOptionsObject(options));
      return makeDate(dateFromFields(fields, overflow), slot.calendar);
    }
    withCalendar(calendarLike) {
      const slot = this.#s;
      return makeDate(slot, toCalendarIdentifier(calendarLike));
    }
    until(other, options = undefined) {
      return differenceDate(false, this.#s, other, options);
    }
    since(other, options = undefined) {
      return differenceDate(true, this.#s, other, options);
    }
    equals(other) {
      const slot = this.#s;
      const target = toDate(other, undefined);
      return compareDate(slot, target) === 0 && slot.calendar === target.calendar;
    }
    toPlainDateTime(temporalTime = undefined) {
      const slot = this.#s;
      return makeDateTime(combine(slot, toTimeOrMidnight(temporalTime)), slot.calendar);
    }
    toZonedDateTime(item) {
      const slot = this.#s;
      let timeZone;
      let temporalTime;
      if (isObject(item)) {
        const timeZoneLike = item.timeZone;
        if (timeZoneLike === undefined) {
          timeZone = toTimeZoneIdentifier(item);
        } else {
          timeZone = toTimeZoneIdentifier(timeZoneLike);
          temporalTime = item.plainTime;
        }
      } else {
        timeZone = toTimeZoneIdentifier(item);
      }
      let ns;
      if (temporalTime === undefined) {
        ns = getStartOfDay(timeZone, slot);
      } else {
        const dateTime = combine(slot, toTimeNs(temporalTime, undefined));
        if (!dateTimeWithinLimits(dateTime)) throw rangeError('date hors limites');
        ns = getEpochNsFor(timeZone, dateTime, 'compatible');
      }
      return makeZoned(ns, timeZone, slot.calendar);
    }
    toString(options = undefined) {
      const slot = this.#s;
      const show = getShowCalendarOption(getOptionsObject(options));
      return formatDate(slot) + formatCalendarAnnotation(slot.calendar, show);
    }
    toLocaleString(locales = undefined, options = undefined) {
      const slot = this.#s;
      return formatDate(slot) + formatCalendarAnnotation(slot.calendar, 'auto');
    }
    toJSON() {
      const slot = this.#s;
      return formatDate(slot) + formatCalendarAnnotation(slot.calendar, 'auto');
    }
    valueOf() {
      throw typeError('une date Temporal ne se convertit pas en primitive : utiliser Temporal.PlainDate.compare');
    }
  }

  // ---- PlainDateTime ----

  function toDateTime(item, options) {
    if (isObject(item)) {
      if (isPlainDateTime(item)) {
        getOverflowOption(getOptionsObject(options));
        return dateTimeSlot(item);
      }
      if (isZoned(item)) {
        const zoned = zonedSlot(item);
        const dateTime = getDateTimeFor(zoned.timeZone, zoned.ns);
        getOverflowOption(getOptionsObject(options));
        dateTime.calendar = zoned.calendar;
        return dateTime;
      }
      if (isPlainDate(item)) {
        getOverflowOption(getOptionsObject(options));
        const slot = dateSlot(item);
        const dateTime = combine(slot, 0);
        dateTime.calendar = slot.calendar;
        return dateTime;
      }
      const calendar = getCalendarWithISODefault(item);
      const fields = prepareFields(item, DATE_TIME_FIELDS, NONE);
      const overflow = getOverflowOption(getOptionsObject(options));
      const dateTime = interpretDateTimeFields(fields, overflow);
      dateTime.calendar = calendar;
      return dateTime;
    }
    if (typeof item !== 'string') throw typeError('date et heure attendues');
    const parsed = parseISO(item, 'datetime');
    const calendar = canonicalizeCalendar(parsed.calendar === undefined ? 'iso8601' : parsed.calendar);
    getOverflowOption(getOptionsObject(options));
    const dateTime = combine(parsed, parsed.t === null ? 0 : parsed.t);
    if (!dateTimeWithinLimits(dateTime)) throw rangeError('date hors limites');
    dateTime.calendar = calendar;
    return dateTime;
  }
  function makeDateTime(dateTime, calendar) {
    const t = dateTime.t;
    return new PlainDateTime(
      dateTime.year, dateTime.month, dateTime.day, timeField(t, 0), timeField(t, 1), timeField(t, 2), timeField(t, 3), timeField(t, 4),
      timeField(t, 5), calendar,
    );
  }
  function addToDateTime(sign, slot, like, options) {
    let f = toDurationFields(like);
    if (sign < 0) f = negatedFields(f);
    const overflow = getOverflowOption(getOptionsObject(options));
    const internal = toInternal24(f);
    const time = addTime(slot.t, internal.time);
    const date = dateAdd(slot, dateDuration(internal.years, internal.months, internal.weeks, time.days), overflow);
    return makeDateTime(combine(date, time.t), slot.calendar);
  }
  function differenceDateTimes(since, slot, other, options) {
    const target = toDateTime(other, undefined);
    if (slot.calendar !== target.calendar) throw rangeError('calendriers différents');
    const settings = getDifferenceSettings(since, getOptionsObject(options), 'datetime', NONE, 'nanosecond', 'day');
    if (compareDateTime(slot, target) === 0) return zeroDuration();
    const duration = differencePlainDateTimeWithRounding(
      slot, target, settings.largestUnit, settings.roundingIncrement, settings.smallestUnit, settings.roundingMode,
    );
    const result = durationFromInternal(duration, settings.largestUnit);
    return since ? negateDuration(result) : result;
  }

  let dateTimeSlot;
  let isPlainDateTime;
  class PlainDateTime {
    #s;
    constructor(isoYear, isoMonth, isoDay, hour = 0, minute = 0, second = 0, millisecond = 0, microsecond = 0, nanosecond = 0, calendar = 'iso8601') {
      const year = toIntegerWithTruncation(isoYear);
      const month = toIntegerWithTruncation(isoMonth);
      const day = toIntegerWithTruncation(isoDay);
      hour = toIntegerWithTruncation(hour);
      minute = toIntegerWithTruncation(minute);
      second = toIntegerWithTruncation(second);
      millisecond = toIntegerWithTruncation(millisecond);
      microsecond = toIntegerWithTruncation(microsecond);
      nanosecond = toIntegerWithTruncation(nanosecond);
      calendar = calendarArgument(calendar);
      if (!isValidDate(year, month, day)) throw rangeError('date invalide');
      if (!isValidTime(hour, minute, second, millisecond, microsecond, nanosecond)) throw rangeError('heure invalide');
      const slot = { year, month, day, t: timeToNs(hour, minute, second, millisecond, microsecond, nanosecond), calendar };
      if (!dateTimeWithinLimits(slot)) throw rangeError('date hors limites');
      this.#s = slot;
    }
    static {
      dateTimeSlot = (object) => object.#s;
      isPlainDateTime = (object) => #s in object;
    }
    static from(item, options = undefined) {
      const slot = toDateTime(item, options);
      return makeDateTime(slot, slot.calendar);
    }
    static compare(one, two) {
      const first = toDateTime(one, undefined);
      return compareDateTime(first, toDateTime(two, undefined));
    }
    get calendarId() {
      return this.#s.calendar;
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
      return this.#s.year;
    }
    get month() {
      return this.#s.month;
    }
    get monthCode() {
      return monthCodeOf(this.#s);
    }
    get day() {
      return this.#s.day;
    }
    get hour() {
      return timeField(this.#s.t, 0);
    }
    get minute() {
      return timeField(this.#s.t, 1);
    }
    get second() {
      return timeField(this.#s.t, 2);
    }
    get millisecond() {
      return timeField(this.#s.t, 3);
    }
    get microsecond() {
      return timeField(this.#s.t, 4);
    }
    get nanosecond() {
      return timeField(this.#s.t, 5);
    }
    get dayOfWeek() {
      return dayOfWeek(this.#s);
    }
    get dayOfYear() {
      return dayOfYear(this.#s);
    }
    get weekOfYear() {
      return weekOfYear(this.#s).week;
    }
    get yearOfWeek() {
      return weekOfYear(this.#s).year;
    }
    get daysInWeek() {
      this.#s;
      return 7;
    }
    get daysInMonth() {
      const slot = this.#s;
      return daysInMonth(slot.year, slot.month);
    }
    get daysInYear() {
      return daysInYearOf(this.#s);
    }
    get monthsInYear() {
      this.#s;
      return 12;
    }
    get inLeapYear() {
      return isLeapYear(this.#s.year);
    }
    with(temporalDateTimeLike, options = undefined) {
      const slot = this.#s;
      requirePartial(temporalDateTimeLike);
      let fields = dateToFields(slot, 'date');
      fields.hour = timeField(slot.t, 0);
      fields.minute = timeField(slot.t, 1);
      fields.second = timeField(slot.t, 2);
      fields.millisecond = timeField(slot.t, 3);
      fields.microsecond = timeField(slot.t, 4);
      fields.nanosecond = timeField(slot.t, 5);
      fields = mergeFields(fields, prepareFields(temporalDateTimeLike, DATE_TIME_FIELDS, PARTIAL));
      const overflow = getOverflowOption(getOptionsObject(options));
      return makeDateTime(interpretDateTimeFields(fields, overflow), slot.calendar);
    }
    withPlainTime(plainTimeLike = undefined) {
      const slot = this.#s;
      return makeDateTime(combine(slot, toTimeOrMidnight(plainTimeLike)), slot.calendar);
    }
    withCalendar(calendarLike) {
      const slot = this.#s;
      return makeDateTime(slot, toCalendarIdentifier(calendarLike));
    }
    add(temporalDurationLike, options = undefined) {
      return addToDateTime(1, this.#s, temporalDurationLike, options);
    }
    subtract(temporalDurationLike, options = undefined) {
      return addToDateTime(-1, this.#s, temporalDurationLike, options);
    }
    until(other, options = undefined) {
      return differenceDateTimes(false, this.#s, other, options);
    }
    since(other, options = undefined) {
      return differenceDateTimes(true, this.#s, other, options);
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
      if (smallestUnit === 'nanosecond' && increment === 1) return makeDateTime(slot, slot.calendar);
      return makeDateTime(roundDateTime(slot, increment, smallestUnit, mode), slot.calendar);
    }
    equals(other) {
      const slot = this.#s;
      const target = toDateTime(other, undefined);
      return compareDateTime(slot, target) === 0 && slot.calendar === target.calendar;
    }
    toString(options = undefined) {
      const slot = this.#s;
      options = getOptionsObject(options);
      const show = getShowCalendarOption(options);
      const digits = getFractionalSecondDigitsOption(options);
      const mode = getRoundingModeOption(options, 'trunc');
      const smallestUnit = getUnitOption(options, 'smallestUnit', undefined);
      validateUnit(smallestUnit, 'time', undefined);
      if (smallestUnit === 'hour') throw rangeError('smallestUnit trop grande');
      const precision = toSecondsStringPrecision(smallestUnit, digits);
      const rounded = roundDateTime(slot, precision.increment, precision.unit, mode);
      if (!dateTimeWithinLimits(rounded)) throw rangeError('date hors limites');
      return formatDateTime(rounded, slot.calendar, precision.precision, show);
    }
    toLocaleString(locales = undefined, options = undefined) {
      const slot = this.#s;
      return formatDateTime(slot, slot.calendar, 'auto', 'auto');
    }
    toJSON() {
      const slot = this.#s;
      return formatDateTime(slot, slot.calendar, 'auto', 'auto');
    }
    valueOf() {
      throw typeError('une date Temporal ne se convertit pas en primitive : utiliser Temporal.PlainDateTime.compare');
    }
    toZonedDateTime(temporalTimeZoneLike, options = undefined) {
      const slot = this.#s;
      const timeZone = toTimeZoneIdentifier(temporalTimeZoneLike);
      const disambiguation = getDisambiguationOption(getOptionsObject(options));
      return makeZoned(getEpochNsFor(timeZone, slot, disambiguation), timeZone, slot.calendar);
    }
    toPlainDate() {
      const slot = this.#s;
      return makeDate(slot, slot.calendar);
    }
    toPlainTime() {
      return makeTime(this.#s.t);
    }
  }

  // ---- PlainYearMonth ----

  function toYearMonth(item, options) {
    if (isObject(item)) {
      if (isPlainYearMonth(item)) {
        getOverflowOption(getOptionsObject(options));
        return yearMonthSlot(item);
      }
      const calendar = getCalendarWithISODefault(item);
      const fields = prepareFields(item, YEAR_MONTH_FIELDS, NONE);
      const overflow = getOverflowOption(getOptionsObject(options));
      const date = yearMonthFromFields(fields, overflow);
      date.calendar = calendar;
      return date;
    }
    if (typeof item !== 'string') throw typeError('mois attendu');
    const parsed = parseISO(item, 'yearmonth');
    const calendar = canonicalizeCalendar(parsed.calendar === undefined ? 'iso8601' : parsed.calendar);
    getOverflowOption(getOptionsObject(options));
    if (!yearMonthWithinLimits(parsed.year, parsed.month)) throw rangeError('mois hors limites');
    const date = yearMonthFromFields(dateToFields(parsed, 'year-month'), 'constrain');
    date.calendar = calendar;
    return date;
  }
  function makeYearMonth(date, calendar) {
    return new PlainYearMonth(date.year, date.month, calendar, date.day);
  }
  function firstOfMonth(slot) {
    const fields = dateToFields(slot, 'year-month');
    fields.day = 1;
    return dateFromFields(fields, 'constrain');
  }
  function addToYearMonth(sign, slot, like, options) {
    let f = toDurationFields(like);
    if (sign < 0) f = negatedFields(f);
    const overflow = getOverflowOption(getOptionsObject(options));
    for (let i = 2; i < 10; i++) {
      if (f[i] !== 0) throw rangeError('seuls des années et des mois s’ajoutent à un mois');
    }
    const added = dateAdd(firstOfMonth(slot), dateDuration(f[0], f[1], 0, 0), overflow);
    return makeYearMonth(yearMonthFromFields(dateToFields(added, 'year-month'), overflow), slot.calendar);
  }
  function differenceYearMonth(since, slot, other, options) {
    const target = toYearMonth(other, undefined);
    if (slot.calendar !== target.calendar) throw rangeError('calendriers différents');
    const settings = getDifferenceSettings(since, getOptionsObject(options), 'date', ['week', 'day'], 'month', 'year');
    if (compareDate(slot, target) === 0) return zeroDuration();
    const thisDate = firstOfMonth(slot);
    const otherDate = firstOfMonth(target);
    const difference = dateUntil(thisDate, otherDate, settings.largestUnit);
    let duration = dateDuration(difference.years, difference.months, 0, 0);
    if (settings.smallestUnit !== 'month' || settings.roundingIncrement !== 1) {
      duration = roundRelativeDuration(
        duration, utcEpochNs(combine(otherDate, 0)), combine(thisDate, 0), undefined, settings.largestUnit, settings.roundingIncrement,
        settings.smallestUnit, settings.roundingMode,
      );
    }
    const result = durationFromInternal(duration, 'day');
    return since ? negateDuration(result) : result;
  }
  function yearMonthToString(slot, show) {
    let text = padYear(slot.year) + '-' + pad(slot.month, 2);
    if (show === 'always' || show === 'critical' || slot.calendar !== 'iso8601') text += '-' + pad(slot.day, 2);
    return text + formatCalendarAnnotation(slot.calendar, show);
  }

  let yearMonthSlot;
  let isPlainYearMonth;
  class PlainYearMonth {
    #s;
    constructor(isoYear, isoMonth, calendar = 'iso8601', referenceISODay = 1) {
      const year = toIntegerWithTruncation(isoYear);
      const month = toIntegerWithTruncation(isoMonth);
      calendar = calendarArgument(calendar);
      const day = toIntegerWithTruncation(referenceISODay);
      if (!isValidDate(year, month, day)) throw rangeError('mois invalide');
      if (!yearMonthWithinLimits(year, month)) throw rangeError('mois hors limites');
      this.#s = { year, month, day, calendar };
    }
    static {
      yearMonthSlot = (object) => object.#s;
      isPlainYearMonth = (object) => #s in object;
    }
    static from(item, options = undefined) {
      const slot = toYearMonth(item, options);
      return makeYearMonth(slot, slot.calendar);
    }
    static compare(one, two) {
      const first = toYearMonth(one, undefined);
      return compareDate(first, toYearMonth(two, undefined));
    }
    get calendarId() {
      return this.#s.calendar;
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
      return this.#s.year;
    }
    get month() {
      return this.#s.month;
    }
    get monthCode() {
      return monthCodeOf(this.#s);
    }
    get daysInYear() {
      return daysInYearOf(this.#s);
    }
    get daysInMonth() {
      const slot = this.#s;
      return daysInMonth(slot.year, slot.month);
    }
    get monthsInYear() {
      this.#s;
      return 12;
    }
    get inLeapYear() {
      return isLeapYear(this.#s.year);
    }
    with(temporalYearMonthLike, options = undefined) {
      const slot = this.#s;
      requirePartial(temporalYearMonthLike);
      const fields = mergeFields(dateToFields(slot, 'year-month'), prepareFields(temporalYearMonthLike, YEAR_MONTH_FIELDS, PARTIAL));
      const overflow = getOverflowOption(getOptionsObject(options));
      return makeYearMonth(yearMonthFromFields(fields, overflow), slot.calendar);
    }
    add(temporalDurationLike, options = undefined) {
      return addToYearMonth(1, this.#s, temporalDurationLike, options);
    }
    subtract(temporalDurationLike, options = undefined) {
      return addToYearMonth(-1, this.#s, temporalDurationLike, options);
    }
    until(other, options = undefined) {
      return differenceYearMonth(false, this.#s, other, options);
    }
    since(other, options = undefined) {
      return differenceYearMonth(true, this.#s, other, options);
    }
    equals(other) {
      const slot = this.#s;
      const target = toYearMonth(other, undefined);
      return compareDate(slot, target) === 0 && slot.calendar === target.calendar;
    }
    toString(options = undefined) {
      const slot = this.#s;
      return yearMonthToString(slot, getShowCalendarOption(getOptionsObject(options)));
    }
    toLocaleString(locales = undefined, options = undefined) {
      return yearMonthToString(this.#s, 'auto');
    }
    toJSON() {
      return yearMonthToString(this.#s, 'auto');
    }
    valueOf() {
      throw typeError('un mois Temporal ne se convertit pas en primitive : utiliser Temporal.PlainYearMonth.compare');
    }
    toPlainDate(item) {
      const slot = this.#s;
      if (!isObject(item)) throw typeError('objet portant le jour attendu');
      const fields = mergeFields(dateToFields(slot, 'year-month'), prepareFields(item, ['day'], NONE));
      return makeDate(dateFromFields(fields, 'constrain'), slot.calendar);
    }
  }

  // ---- PlainMonthDay ----

  function toMonthDay(item, options) {
    if (isObject(item)) {
      if (isPlainMonthDay(item)) {
        getOverflowOption(getOptionsObject(options));
        return monthDaySlot(item);
      }
      const calendar = getCalendarWithISODefault(item);
      const fields = prepareFields(item, DATE_FIELDS, NONE);
      const overflow = getOverflowOption(getOptionsObject(options));
      const date = monthDayFromFields(fields, overflow);
      date.calendar = calendar;
      return date;
    }
    if (typeof item !== 'string') throw typeError('jour de l’année attendu');
    const parsed = parseISO(item, 'monthday');
    const calendar = canonicalizeCalendar(parsed.calendar === undefined ? 'iso8601' : parsed.calendar);
    getOverflowOption(getOptionsObject(options));
    return { year: 1972, month: parsed.month, day: parsed.day, calendar };
  }
  function makeMonthDay(date, calendar) {
    return new PlainMonthDay(date.month, date.day, calendar, date.year);
  }
  function monthDayToString(slot, show) {
    let text = pad(slot.month, 2) + '-' + pad(slot.day, 2);
    if (show === 'always' || show === 'critical' || slot.calendar !== 'iso8601') text = padYear(slot.year) + '-' + text;
    return text + formatCalendarAnnotation(slot.calendar, show);
  }

  let monthDaySlot;
  let isPlainMonthDay;
  class PlainMonthDay {
    #s;
    constructor(isoMonth, isoDay, calendar = 'iso8601', referenceISOYear = 1972) {
      const month = toIntegerWithTruncation(isoMonth);
      const day = toIntegerWithTruncation(isoDay);
      calendar = calendarArgument(calendar);
      const year = toIntegerWithTruncation(referenceISOYear);
      if (!isValidDate(year, month, day)) throw rangeError('jour invalide');
      const slot = { year, month, day, calendar };
      if (!dateWithinLimits(slot)) throw rangeError('date hors limites');
      this.#s = slot;
    }
    static {
      monthDaySlot = (object) => object.#s;
      isPlainMonthDay = (object) => #s in object;
    }
    static from(item, options = undefined) {
      const slot = toMonthDay(item, options);
      return makeMonthDay(slot, slot.calendar);
    }
    get calendarId() {
      return this.#s.calendar;
    }
    get monthCode() {
      return monthCodeOf(this.#s);
    }
    get day() {
      return this.#s.day;
    }
    with(temporalMonthDayLike, options = undefined) {
      const slot = this.#s;
      requirePartial(temporalMonthDayLike);
      const fields = mergeFields(dateToFields(slot, 'month-day'), prepareFields(temporalMonthDayLike, DATE_FIELDS, PARTIAL));
      const overflow = getOverflowOption(getOptionsObject(options));
      return makeMonthDay(monthDayFromFields(fields, overflow), slot.calendar);
    }
    equals(other) {
      const slot = this.#s;
      const target = toMonthDay(other, undefined);
      return compareDate(slot, target) === 0 && slot.calendar === target.calendar;
    }
    toString(options = undefined) {
      const slot = this.#s;
      return monthDayToString(slot, getShowCalendarOption(getOptionsObject(options)));
    }
    toLocaleString(locales = undefined, options = undefined) {
      return monthDayToString(this.#s, 'auto');
    }
    toJSON() {
      return monthDayToString(this.#s, 'auto');
    }
    valueOf() {
      throw typeError('un jour Temporal ne se convertit pas en primitive');
    }
    toPlainDate(item) {
      const slot = this.#s;
      if (!isObject(item)) throw typeError('objet portant l’année attendu');
      const fields = mergeFields(dateToFields(slot, 'month-day'), prepareFields(item, ['year'], NONE));
      return makeDate(dateFromFields(fields, 'constrain'), slot.calendar);
    }
  }
