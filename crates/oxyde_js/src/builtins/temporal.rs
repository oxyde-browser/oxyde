//! `Temporal` : dates, heures et durées (proposition « Temporal »).
//!
//! L'objet est écrit en JavaScript (dossier `temporal/`) et n'est construit qu'à
//! son premier usage dans un royaume : la plupart des scripts ne s'en
//! servent pas, et son installation coûte plus cher que celle de tout le
//! reste de la bibliothèque.
//!
//! LIMITES CONNUES : seul le calendrier `iso8601` existe ; les fuseaux
//! horaires se limitent à `UTC` et aux décalages fixes (`+01:00`).

use std::time::{SystemTime, UNIX_EPOCH};

use crate::value::{ObjId, Value};
use crate::vm::{JsResult, Vm};

/// Nanosecondes écoulées depuis l'époque Unix, en chaîne décimale : le
/// script en fait un `BigInt`.
fn now_nanoseconds(_vm: &mut Vm, _this: &Value, _args: &[Value]) -> JsResult {
    let nanoseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    Ok(Value::string(&nanoseconds.to_string()))
}

/// Le script : quatre fichiers qui forment une seule fonction.
const SOURCE: &str = concat!(
    include_str!("temporal/1_base.js"),
    include_str!("temporal/2_duration.js"),
    include_str!("temporal/3_plain.js"),
    include_str!("temporal/4_zoned.js"),
);

/// Emplacement interne de l'objet global où vivent les parties du script.
const PARTS_SLOT: &str = "temporalParts";

/// `Date.prototype.toTemporalInstant()`.
fn to_temporal_instant(vm: &mut Vm, this: &Value, _args: &[Value]) -> JsResult {
    let global = vm.global;
    vm.materialize_temporal(global)?;
    let Some(parts) = vm.slot(global, PARTS_SLOT) else {
        return Err(vm.type_error("Temporal n'est pas disponible dans ce royaume"));
    };
    let convert = vm.get(&parts, "toTemporalInstant")?;
    vm.call(&convert, &Value::Undefined, std::slice::from_ref(this))
}

impl Vm {
    /// Construit `Temporal` dans le royaume de l'objet global `global`, s'il
    /// ne l'est pas déjà.
    ///
    /// # Errors
    /// Ce que lève le script d'installation.
    pub(crate) fn materialize_temporal(&mut self, global: ObjId) -> JsResult<()> {
        let before = self.lazy_temporal.len();
        self.lazy_temporal.retain(|pending| *pending != global);
        if self.lazy_temporal.len() == before {
            return Ok(());
        }
        let realm = usize::from(self.heap.get(global).realm);
        let previous = self.realm();
        self.set_realm(realm);
        let built = self.eval_script(SOURCE).and_then(|factory| {
            let clock = Value::Object(self.new_function("now", 0, now_nanoseconds));
            self.call(&factory, &Value::Undefined, &[clock])
        });
        self.set_realm(previous);
        let parts = built?;
        self.set_slot(global, PARTS_SLOT, parts.clone());
        let temporal = self.get(&parts, "Temporal")?;
        self.define_hidden(global, "Temporal", temporal);
        // Ses fonctions sont des intégrés : `toString` les montre natives.
        let natives = self.get(&parts, "natives")?;
        let mut index = 0;
        while let Ok(Value::Object(id)) = self.get(&natives, &index.to_string()) {
            self.self_hosted.push(id);
            index += 1;
        }
        Ok(())
    }
}

/// Inscrit le royaume courant parmi ceux dont `Temporal` reste à construire.
pub(super) fn install(vm: &mut Vm) {
    vm.lazy_temporal.push(vm.global);
    let date_proto = vm.intrinsics.date_proto;
    vm.define_method(date_proto, "toTemporalInstant", 0, to_temporal_instant);
}
