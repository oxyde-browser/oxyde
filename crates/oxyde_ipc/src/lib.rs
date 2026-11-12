//! Communication entre processus (pendant de Mojo).
//!
//! Messages typés entre les processus d'Oxyde, sur la seule bibliothèque
//! standard :
//!
//! - [`codec`] : codage binaire des messages ([`Encode`], [`Decode`], et les
//!   macros [`ipc_struct!`] et [`ipc_enum!`] pour les types composés) ;
//! - [`channel`] : trames, [`Sender`], [`Receiver`], lancement d'un processus
//!   enfant relié par un canal ([`spawn`]) et, côté enfant,
//!   [`connect_to_parent`].
//!
//! Un message reçu vient d'un processus qui a pu être compromis : le
//! décodage vérifie tout, ne panique jamais et ne réserve pas plus de
//! mémoire que le correspondant n'a envoyé d'octets.
//!
//! LIMITES CONNUES : pas de mémoire partagée (un gros message est copié
//! dans le tube) ; pas de transfert de descripteurs entre processus.

pub mod channel;
pub mod codec;

pub use channel::{
    ChannelError, ChildProcess, MAX_FRAME, Receiver, Sender, connect_to_parent, read_frame, spawn,
    write_frame,
};
pub use codec::{Bytes, Decode, DecodeError, Encode, Reader, Writer, from_bytes, to_bytes};
