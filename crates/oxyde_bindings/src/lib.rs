//! Générateur de liaisons Web IDL (pendant des bindings de Blink).
//!
//! Les interfaces du DOM sont décrites, comme dans les standards du W3C et du
//! WHATWG, par des fichiers Web IDL. Ce crate les analyse ([`parse`]),
//! fusionne interfaces partielles et mixins ([`resolve`]) et produit le code
//! Rust qui les décrit au moteur de script ([`generate`]) : une table de
//! [`InterfaceDescriptor`] à partir de laquelle la page construit les
//! prototypes, les constructeurs et les constantes de chaque interface.
//!
//! ÉTAT : le générateur produit la description des interfaces (chaîne
//! d'héritage, constantes, attributs, opérations et leur nombre d'arguments
//! requis). La conversion des arguments d'après leurs types IDL et l'appel
//! direct des implémentations ne sont pas encore générés : les opérations
//! sont reliées à des fonctions écrites à la main dans `oxyde_content`.
//!
//! LIMITES CONNUES de l'analyseur : les types sont validés puis gardés sous
//! forme de texte ; les valeurs par défaut des arguments sont sautées sans
//! être interprétées ; `iterable`, `maplike`, `setlike` et les opérations
//! spéciales sans nom (`getter`, `setter`, `deleter`) sont reconnus mais
//! n'engendrent aucun membre.

mod generate;
mod parse;

pub use generate::generate;
pub use parse::{Definition, IdlError, Interface, InterfaceKind, Member, parse, resolve};

/// Attribut d'une interface, tel que le décrit son IDL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttributeDescriptor {
    /// Nom de l'attribut.
    pub name: &'static str,
    /// Vrai si l'attribut est en lecture seule.
    pub readonly: bool,
    /// Vrai si l'attribut appartient à l'objet interface (`static`).
    pub is_static: bool,
}

/// Opération (méthode) d'une interface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationDescriptor {
    /// Nom de l'opération.
    pub name: &'static str,
    /// Nombre d'arguments requis (propriété `length` de la fonction) ; pour
    /// une opération surchargée, le plus petit.
    pub length: u32,
    /// Vrai si l'opération appartient à l'objet interface (`static`).
    pub is_static: bool,
}

/// Interface Web IDL, mixins et définitions partielles fusionnés.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InterfaceDescriptor {
    /// Nom de l'interface (`Node`, `Element`…).
    pub name: &'static str,
    /// Interface dont elle hérite.
    pub parent: Option<&'static str>,
    /// Nombre d'arguments requis du constructeur, si l'interface en déclare un.
    pub constructor: Option<u32>,
    /// Constantes numériques : (nom, valeur).
    pub constants: &'static [(&'static str, f64)],
    /// Attributs.
    pub attributes: &'static [AttributeDescriptor],
    /// Opérations.
    pub operations: &'static [OperationDescriptor],
}
