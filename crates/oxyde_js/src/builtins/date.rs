//! `Date` : valeurs temporelles, calendrier grégorien proleptique, analyse et
//! mise en forme (ECMA-262 §21.4).
//!
//! LIMITES CONNUES : le fuseau horaire local n'est pas consulté — l'heure
//! locale est l'heure UTC, `getTimezoneOffset()` vaut 0 ; les méthodes
//! `toLocale…String` rendent le même texte que leurs équivalents non
//! localisés ; `Date.parse` ne reconnaît que le format ISO et les formats
//! produits par `toString` et `toUTCString`.

use super::{arg, constructor};
use crate::heap::{Kind, Object, Property, Slot};
use crate::value::{PropertyKey, Value};
use crate::vm::{JsResult, NativeFn, Vm};

const MS_PER_DAY: f64 = 86_400_000.0;
/// Plus grande valeur temporelle : ±100 000 000 jours autour de 1970.
const MAX_TIME: f64 = 8.64e15;
const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Jours écoulés depuis le 1er janvier 1970 pour une date civile (mois de 1
/// à 12). Algorithme de Howard Hinnant, valable pour toute année.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// Date civile (année, mois de 1 à 12, jour) d'un nombre de jours depuis 1970.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// `MakeDay` : numéro du jour de (année, mois à partir de 0, jour du mois).
fn make_day(year: f64, month: f64, date: f64) -> f64 {
    if !(year.is_finite() && month.is_finite() && date.is_finite()) {
        return f64::NAN;
    }
    let (year, month, date) = (year.trunc(), month.trunc(), date.trunc());
    let full_year = year + (month / 12.0).floor();
    if full_year.abs() > 400_000.0 {
        return f64::NAN;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "année bornée à ±400 000, mois ramené dans [0, 11]"
    )]
    let days = days_from_civil(full_year as i64, month.rem_euclid(12.0) as i64 + 1, 1);
    #[expect(
        clippy::cast_precision_loss,
        reason = "nombre de jours petit devant 2^53"
    )]
    let days = days as f64;
    days + date - 1.0
}

/// `MakeTime`.
fn make_time(hour: f64, minute: f64, second: f64, ms: f64) -> f64 {
    if !(hour.is_finite() && minute.is_finite() && second.is_finite() && ms.is_finite()) {
        return f64::NAN;
    }
    hour.trunc() * 3_600_000.0 + minute.trunc() * 60_000.0 + second.trunc() * 1000.0 + ms.trunc()
}

/// `MakeDate`.
fn make_date(day: f64, time: f64) -> f64 {
    let value = day * MS_PER_DAY + time;
    if value.is_finite() { value } else { f64::NAN }
}

/// `TimeClip` : valeur temporelle valide, ou `NaN`.
fn time_clip(time: f64) -> f64 {
    if !time.is_finite() || time.abs() > MAX_TIME {
        return f64::NAN;
    }
    // `+ 0.0` change `-0` en `+0`.
    time.trunc() + 0.0
}

/// Champs d'une valeur temporelle finie : année, mois (à partir de 0), jour,
/// heures, minutes, secondes, millisecondes.
fn fields(time: f64) -> [f64; 7] {
    let day = (time / MS_PER_DAY).floor();
    let within = time - day * MS_PER_DAY;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "jour borné à ±100 000 000 par TimeClip"
    )]
    let (year, month, date) = civil_from_days(day as i64);
    #[expect(
        clippy::cast_precision_loss,
        reason = "champs de date petits devant 2^53"
    )]
    let (year, month, date) = (year as f64, month as f64, date as f64);
    [
        year,
        month - 1.0,
        date,
        (within / 3_600_000.0).floor(),
        (within / 60_000.0).floor() % 60.0,
        (within / 1000.0).floor() % 60.0,
        within % 1000.0,
    ]
}

/// Jour de la semaine (0 : dimanche).
fn weekday(time: f64) -> usize {
    let day = (time / MS_PER_DAY).floor();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "ramené dans [0, 6]"
    )]
    let index = (day + 4.0).rem_euclid(7.0) as usize;
    index
}

fn compose(f: &[f64; 7]) -> f64 {
    time_clip(make_date(
        make_day(f[0], f[1], f[2]),
        make_time(f[3], f[4], f[5], f[6]),
    ))
}

fn now_ms() -> f64 {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    #[expect(
        clippy::cast_precision_loss,
        reason = "millisecondes depuis 1970 : bien en deçà de 2^53"
    )]
    let ms = elapsed.as_millis() as f64;
    ms
}

// ---- Mise en forme ---------------------------------------------------------------

/// Année sur au moins quatre chiffres, avec signe si elle est négative.
fn year_text(year: f64) -> String {
    if year < 0.0 {
        format!("-{:06}", -year)
    } else {
        format!("{year:04}")
    }
}

fn date_text(time: f64) -> String {
    let f = fields(time);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "mois dans [0, 11]"
    )]
    let month = MONTHS[(f[1] as usize).min(11)];
    format!(
        "{} {} {:02} {}",
        WEEKDAYS[weekday(time)],
        month,
        f[2],
        year_text(f[0])
    )
}

fn time_text(time: f64) -> String {
    let f = fields(time);
    format!("{:02}:{:02}:{:02} GMT+0000 (UTC)", f[3], f[4], f[5])
}

fn iso_text(time: f64) -> String {
    let f = fields(time);
    let year = if (0.0..=9999.0).contains(&f[0]) {
        format!("{:04}", f[0])
    } else if f[0] < 0.0 {
        format!("-{:06}", -f[0])
    } else {
        format!("+{:06}", f[0])
    };
    format!(
        "{year}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        f[1] + 1.0,
        f[2],
        f[3],
        f[4],
        f[5],
        f[6]
    )
}

fn utc_text(time: f64) -> String {
    let f = fields(time);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "mois dans [0, 11]"
    )]
    let month = MONTHS[(f[1] as usize).min(11)];
    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        WEEKDAYS[weekday(time)],
        f[2],
        month,
        year_text(f[0]),
        f[3],
        f[4],
        f[5]
    )
}

// ---- Analyse ---------------------------------------------------------------------

/// Lit exactement `count` chiffres au début de `text`.
fn digits(text: &str, count: usize) -> Option<(f64, &str)> {
    let head = text.get(..count)?;
    if !head.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((head.parse().ok()?, text.get(count..)?))
}

fn days_in_month(year: f64, month: f64) -> f64 {
    make_day(year, month, 1.0) - make_day(year, month - 1.0, 1.0)
}

/// Format ISO 8601 simplifié d'ECMA-262 (`YYYY-MM-DDTHH:mm:ss.sssZ`).
fn parse_iso(text: &str) -> Option<f64> {
    let (year, mut rest) = match text.as_bytes().first()? {
        b'+' | b'-' => {
            let (year, rest) = digits(text.get(1..)?, 6)?;
            if text.starts_with('-') {
                // « -000000 » n'est pas une année valide.
                if year == 0.0 {
                    return None;
                }
                (-year, rest)
            } else {
                (year, rest)
            }
        }
        _ => digits(text, 4)?,
    };
    let (mut month, mut day) = (1.0, 1.0);
    if let Some(after) = rest.strip_prefix('-') {
        (month, rest) = digits(after, 2)?;
        if let Some(after) = rest.strip_prefix('-') {
            (day, rest) = digits(after, 2)?;
        }
    }
    if !(1.0..=12.0).contains(&month) || day < 1.0 || day > days_in_month(year, month) {
        return None;
    }
    let (mut hour, mut minute, mut second, mut ms) = (0.0, 0.0, 0.0, 0.0);
    let mut offset = 0.0;
    if let Some(after) = rest.strip_prefix('T') {
        (hour, rest) = digits(after, 2)?;
        (minute, rest) = digits(rest.strip_prefix(':')?, 2)?;
        if let Some(after) = rest.strip_prefix(':') {
            (second, rest) = digits(after, 2)?;
            if let Some(after) = rest.strip_prefix('.') {
                let count = after.bytes().take_while(u8::is_ascii_digit).count();
                if count == 0 {
                    return None;
                }
                // Au-delà de la milliseconde, les chiffres sont ignorés.
                let fraction = format!("{:0<3}", after.get(..count.min(3))?);
                ms = fraction.parse().ok()?;
                rest = after.get(count..)?;
            }
        }
        if let Some(after) = rest.strip_prefix('Z') {
            rest = after;
        } else if let Some(sign) = rest.chars().next().filter(|c| matches!(c, '+' | '-')) {
            let (hours, after) = digits(rest.get(1..)?, 2)?;
            let (minutes, after) = digits(after.strip_prefix(':')?, 2)?;
            if hours > 23.0 || minutes > 59.0 {
                return None;
            }
            offset = (hours * 60.0 + minutes) * 60_000.0 * if sign == '-' { -1.0 } else { 1.0 };
            rest = after;
        }
        let midnight = hour == 24.0 && minute == 0.0 && second == 0.0 && ms == 0.0;
        if (hour > 23.0 && !midnight) || minute > 59.0 || second > 59.0 {
            return None;
        }
    }
    if !rest.is_empty() {
        return None;
    }
    let time = make_date(
        make_day(year, month - 1.0, day),
        make_time(hour, minute, second, ms),
    );
    Some(time_clip(time - offset))
}

/// Formats de `toString` (« Tue Feb 01 2022 10:00:00 GMT+0100 (…) ») et de
/// `toUTCString` (« Tue, 01 Feb 2022 10:00:00 GMT »).
fn parse_legacy(text: &str) -> Option<f64> {
    let cleaned = text.replace(',', " ");
    let mut tokens = cleaned.split_whitespace().peekable();
    if tokens.peek().is_some_and(|t| WEEKDAYS.contains(t)) {
        tokens.next();
    }
    let month_of = |token: &str| MONTHS.iter().position(|m| *m == token);
    let first = tokens.next()?;
    let second = tokens.next()?;
    let (month, day) = match (month_of(first), month_of(second)) {
        (Some(month), None) => (month, second),
        (None, Some(month)) => (month, first),
        _ => return None,
    };
    let day: f64 = day.parse().ok()?;
    let year_token = tokens.next()?;
    let year: f64 = year_token.parse().ok()?;
    let mut time = [0.0_f64; 3];
    let mut offset = 0.0;
    if let Some(clock) = tokens.next() {
        let mut parts = clock.split(':');
        for slot in &mut time {
            *slot = parts.next()?.parse().ok()?;
        }
        if parts.next().is_some() {
            return None;
        }
        if let Some(zone) = tokens.next() {
            let zone = zone
                .strip_prefix("GMT")
                .or_else(|| zone.strip_prefix("UTC"))?;
            if !zone.is_empty() {
                let sign = if zone.starts_with('-') { -1.0 } else { 1.0 };
                let (hours, rest) = digits(zone.get(1..)?, 2)?;
                let (minutes, rest) = digits(rest, 2)?;
                if !rest.is_empty() {
                    return None;
                }
                offset = sign * (hours * 60.0 + minutes) * 60_000.0;
            }
        }
    }
    if !day.is_finite()
        || day < 1.0
        || day > 31.0
        || time[0] > 24.0
        || time[1] > 59.0
        || time[2] > 59.0
    {
        return None;
    }
    #[expect(clippy::cast_precision_loss, reason = "mois dans [0, 11]")]
    let month = month as f64;
    let value = make_date(
        make_day(year, month, day),
        make_time(time[0], time[1], time[2], 0.0),
    );
    Some(time_clip(value - offset))
}

fn parse(text: &str) -> f64 {
    let text = text.trim();
    parse_iso(text)
        .or_else(|| parse_legacy(text))
        .unwrap_or(f64::NAN)
}

// ---- Constructeur et méthodes statiques ------------------------------------------

fn new_date(vm: &mut Vm, time: f64) -> Value {
    Value::Object(vm.heap.alloc(Object::new(
        Some(vm.intrinsics.date_proto),
        Kind::Date(time),
    )))
}

/// Valeur temporelle composée des arguments (année, mois, jour, heures…).
fn from_components(vm: &mut Vm, args: &[Value]) -> JsResult<f64> {
    let mut f = [f64::NAN, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
    for (slot, value) in f.iter_mut().zip(args) {
        *slot = vm.to_number(value)?;
    }
    // Une année de 0 à 99 désigne le vingtième siècle.
    let year = f[0].trunc();
    if (0.0..=99.0).contains(&year) {
        f[0] = 1900.0 + year;
    }
    Ok(compose(&f))
}

fn date_ctor(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    if !vm.constructing {
        let now = now_ms();
        return Ok(Value::string(&format!(
            "{} {}",
            date_text(now),
            time_text(now)
        )));
    }
    let time = match args {
        [] => now_ms(),
        [value] => {
            let existing = value.as_object().and_then(|id| match vm.heap.get(id).kind {
                Kind::Date(time) => Some(time),
                _ => None,
            });
            match existing {
                Some(time) => time,
                None => match vm.to_primitive_hint(value, crate::ops::Hint::Default)? {
                    Value::String(text) => parse(&text),
                    other => time_clip(vm.to_number(&other)?),
                },
            }
        }
        _ => from_components(vm, args)?,
    };
    Ok(new_date(vm, time))
}

fn date_now(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    Ok(Value::Number(now_ms()))
}

fn date_parse(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    let text = vm.to_string(&arg(args, 0))?;
    Ok(Value::Number(parse(&text)))
}

fn date_utc(vm: &mut Vm, _this: &Value, args: &[Value]) -> JsResult {
    from_components(vm, args).map(Value::Number)
}

// ---- Prototype -------------------------------------------------------------------

fn this_time(vm: &mut Vm, this: &Value) -> JsResult<f64> {
    match this.as_object().map(|id| &vm.heap.get(id).kind) {
        Some(Kind::Date(time)) => Ok(*time),
        _ => Err(vm.type_error("le receveur n'est pas une date")),
    }
}

fn set_time(vm: &mut Vm, this: &Value, time: f64) {
    if let Some(id) = this.as_object()
        && let Kind::Date(slot) = &mut vm.heap.get_mut(id).kind
    {
        *slot = time;
    }
}

fn get_time(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    this_time(vm, this).map(Value::Number)
}

fn get_field(vm: &mut Vm, this: &Value, index: usize) -> JsResult {
    let time = this_time(vm, this)?;
    Ok(Value::Number(if time.is_nan() {
        f64::NAN
    } else {
        fields(time)[index]
    }))
}

macro_rules! field_getters {
    ($($name:ident => $index:expr),* $(,)?) => {
        $(
            fn $name(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
                get_field(vm, this, $index)
            }
        )*
    };
}
field_getters! {
    get_full_year => 0, get_month => 1, get_date => 2, get_hours => 3,
    get_minutes => 4, get_seconds => 5, get_milliseconds => 6,
}

fn get_day(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let time = this_time(vm, this)?;
    #[expect(clippy::cast_precision_loss, reason = "jour de la semaine dans [0, 6]")]
    Ok(Value::Number(if time.is_nan() {
        f64::NAN
    } else {
        weekday(time) as f64
    }))
}

fn get_year(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let time = this_time(vm, this)?;
    Ok(Value::Number(if time.is_nan() {
        f64::NAN
    } else {
        fields(time)[0] - 1900.0
    }))
}

fn get_timezone_offset(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let time = this_time(vm, this)?;
    Ok(Value::Number(if time.is_nan() { f64::NAN } else { 0.0 }))
}

/// Modifie les champs à partir de `first` avec les arguments fournis (au plus
/// `max`), puis recompose la valeur temporelle.
fn set_fields(vm: &mut Vm, this: &Value, args: &[Value], first: usize, max: usize) -> JsResult {
    let time = this_time(vm, this)?;
    // `setFullYear` sur une date invalide repart de l'époque.
    let mut f = if time.is_nan() {
        fields(0.0)
    } else {
        fields(time)
    };
    for offset in 0..max {
        // Le premier argument est toujours converti, même absent (NaN).
        if offset == 0 || offset < args.len() {
            f[first + offset] = vm.to_number(&arg(args, offset))?;
        }
    }
    let result = if time.is_nan() && first != 0 {
        f64::NAN
    } else {
        compose(&f)
    };
    set_time(vm, this, result);
    Ok(Value::Number(result))
}

macro_rules! field_setters {
    ($($name:ident => $first:expr, $max:expr);* $(;)?) => {
        $(
            fn $name(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
                set_fields(vm, this, args, $first, $max)
            }
        )*
    };
}
field_setters! {
    set_full_year => 0, 3; set_month => 1, 2; set_date => 2, 1; set_hours => 3, 4;
    set_minutes => 4, 3; set_seconds => 5, 2; set_milliseconds => 6, 1;
}

fn set_time_method(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    this_time(vm, this)?;
    let time = time_clip(vm.to_number(&arg(args, 0))?);
    set_time(vm, this, time);
    Ok(Value::Number(time))
}

fn set_year(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    let time = this_time(vm, this)?;
    let year = vm.to_number(&arg(args, 0))?;
    if year.is_nan() {
        set_time(vm, this, f64::NAN);
        return Ok(Value::Number(f64::NAN));
    }
    let mut f = if time.is_nan() {
        fields(0.0)
    } else {
        fields(time)
    };
    let truncated = year.trunc();
    f[0] = if (0.0..=99.0).contains(&truncated) {
        1900.0 + truncated
    } else {
        year
    };
    let result = compose(&f);
    set_time(vm, this, result);
    Ok(Value::Number(result))
}

/// Texte produit par `format`, ou « Invalid Date ».
fn text_of(vm: &mut Vm, this: &Value, format: fn(f64) -> String) -> JsResult {
    let time = this_time(vm, this)?;
    Ok(Value::string(&if time.is_nan() {
        "Invalid Date".to_owned()
    } else {
        format(time)
    }))
}

fn to_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    text_of(vm, this, |time| {
        format!("{} {}", date_text(time), time_text(time))
    })
}

fn to_date_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    text_of(vm, this, date_text)
}

fn to_time_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    text_of(vm, this, time_text)
}

fn to_utc_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    text_of(vm, this, utc_text)
}

fn to_iso_string(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let time = this_time(vm, this)?;
    if time.is_nan() {
        return Err(vm.error("RangeError", "date invalide"));
    }
    Ok(Value::string(&iso_text(time)))
}

fn to_json(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let object = vm.to_object(this)?;
    let primitive = vm.to_primitive(&object, false)?;
    if let Value::Number(n) = primitive
        && !n.is_finite()
    {
        return Ok(Value::Null);
    }
    let method = vm.get(&object, "toISOString")?;
    vm.call(&method, &object, &[])
}

/// `Date.prototype[Symbol.toPrimitive]` : une date préfère la chaîne.
fn to_primitive(vm: &mut Vm, this: &Value, args: &[Value]) -> JsResult {
    if this.as_object().is_none() {
        return Err(vm.type_error("Symbol.toPrimitive : le receveur doit être un objet"));
    }
    let prefer_string = match arg(args, 0) {
        Value::String(hint) if &*hint == "string" || &*hint == "default" => true,
        Value::String(hint) if &*hint == "number" => false,
        _ => return Err(vm.type_error("Symbol.toPrimitive : indication invalide")),
    };
    vm.ordinary_to_primitive(this, prefer_string)
}

pub(super) fn install(vm: &mut Vm) {
    let proto = vm.intrinsics.date_proto;
    let ctor = constructor(vm, "Date", 7, date_ctor, proto);
    vm.define_method(ctor, "now", 0, date_now);
    vm.define_method(ctor, "parse", 1, date_parse);
    vm.define_method(ctor, "UTC", 7, date_utc);
    // L'heure locale étant l'heure UTC, chaque accesseur sert aux deux noms.
    let both: &[(&str, &str, u32, NativeFn)] = &[
        ("getFullYear", "getUTCFullYear", 0, get_full_year),
        ("getMonth", "getUTCMonth", 0, get_month),
        ("getDate", "getUTCDate", 0, get_date),
        ("getDay", "getUTCDay", 0, get_day),
        ("getHours", "getUTCHours", 0, get_hours),
        ("getMinutes", "getUTCMinutes", 0, get_minutes),
        ("getSeconds", "getUTCSeconds", 0, get_seconds),
        ("getMilliseconds", "getUTCMilliseconds", 0, get_milliseconds),
        ("setFullYear", "setUTCFullYear", 3, set_full_year),
        ("setMonth", "setUTCMonth", 2, set_month),
        ("setDate", "setUTCDate", 1, set_date),
        ("setHours", "setUTCHours", 4, set_hours),
        ("setMinutes", "setUTCMinutes", 3, set_minutes),
        ("setSeconds", "setUTCSeconds", 2, set_seconds),
        ("setMilliseconds", "setUTCMilliseconds", 1, set_milliseconds),
    ];
    for (local, utc, arity, func) in both {
        vm.define_method(proto, local, *arity, *func);
        vm.define_method(proto, utc, *arity, *func);
    }
    let methods: &[(&str, u32, NativeFn)] = &[
        ("getTime", 0, get_time),
        ("valueOf", 0, get_time),
        ("getYear", 0, get_year),
        ("getTimezoneOffset", 0, get_timezone_offset),
        ("setTime", 1, set_time_method),
        ("setYear", 1, set_year),
        ("toString", 0, to_string),
        ("toDateString", 0, to_date_string),
        ("toTimeString", 0, to_time_string),
        ("toLocaleString", 0, to_string),
        ("toLocaleDateString", 0, to_date_string),
        ("toLocaleTimeString", 0, to_time_string),
        ("toUTCString", 0, to_utc_string),
        ("toISOString", 0, to_iso_string),
        ("toJSON", 1, to_json),
    ];
    for (name, arity, func) in methods {
        vm.define_method(proto, name, *arity, *func);
    }
    // `toGMTString` est la fonction même de `toUTCString` (annexe B).
    if let Ok(utc) = vm.get(&Value::Object(proto), "toUTCString") {
        vm.define_hidden(proto, "toGMTString", utc);
    }
    let symbol = vm.well_known.to_primitive.clone();
    let function = vm.new_function("[Symbol.toPrimitive]", 1, to_primitive);
    vm.heap.get_mut(proto).insert_prop(
        PropertyKey::Symbol(symbol),
        Property {
            slot: Slot::Data(Value::Object(function)),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calendar_round_trips() {
        for (y, m, d, days) in [
            (1970, 1, 1, 0),
            (1969, 12, 31, -1),
            (2000, 2, 29, 11_016),
            (2024, 3, 1, 19_783),
            (1, 1, 1, -719_162),
            (-1, 12, 31, -719_529),
            (275_760, 9, 13, 100_000_000),
        ] {
            assert_eq!(days_from_civil(y, m, d), days, "{y}-{m}-{d}");
            assert_eq!(civil_from_days(days), (y, m, d), "{days}");
        }
        assert_eq!(make_day(2022.0, 13.0, 1.0), make_day(2023.0, 1.0, 1.0));
        assert_eq!(make_day(2022.0, -1.0, 1.0), make_day(2021.0, 11.0, 1.0));
        assert!(make_day(f64::NAN, 0.0, 1.0).is_nan());
        assert!(time_clip(8.64e15 + 1.0).is_nan());
        assert_eq!(time_clip(-0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(weekday(0.0), 4);
        assert_eq!(weekday(-MS_PER_DAY), 3);
        assert_eq!(fields(-1.0), [1969.0, 11.0, 31.0, 23.0, 59.0, 59.0, 999.0]);
    }

    #[test]
    fn parsing_and_formatting() {
        let time = parse("2022-02-01T10:20:30.456Z");
        assert_eq!(time, 1_643_710_830_456.0);
        assert_eq!(iso_text(time), "2022-02-01T10:20:30.456Z");
        assert_eq!(utc_text(time), "Tue, 01 Feb 2022 10:20:30 GMT");
        assert_eq!(date_text(time), "Tue Feb 01 2022");
        assert_eq!(parse(&utc_text(time)), 1_643_710_830_000.0);
        assert_eq!(
            parse(&format!("{} {}", date_text(time), time_text(time))),
            1_643_710_830_000.0
        );
        assert_eq!(parse("2022"), 1_640_995_200_000.0);
        assert_eq!(parse("2022-02"), 1_643_673_600_000.0);
        assert_eq!(parse("2022-02-01T10:00+02:00"), parse("2022-02-01T08:00Z"));
        assert_eq!(parse("2022-02-01T24:00"), parse("2022-02-02T00:00"));
        assert_eq!(parse("+275760-09-13T00:00:00.000Z"), 8.64e15);
        assert_eq!(
            iso_text(parse("-000001-01-01T00:00:00Z")),
            "-000001-01-01T00:00:00.000Z"
        );
        assert_eq!(parse("2022-02-01T10:20:30.123456Z"), 1_643_710_830_123.0);
        for bad in [
            "",
            "x",
            "2022-13-01",
            "2022-02-30",
            "2022-02-01T25:00",
            "2022-02-01T10",
            "-000000-01-01",
            "2022-02-01T24:00:01",
            "2022-2-1",
            "2022-02-01T10:00:00.Z",
            "+275760-09-13T00:00:00.001Z",
            "Feb 31",
            "Tue Feb 01 2022 10:00:00 XYZ",
        ] {
            assert!(parse(bad).is_nan(), "{bad:?}");
        }
    }
}
