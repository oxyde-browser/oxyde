//! Canal de messages entre deux processus.
//!
//! Chaque message voyage dans une trame : sa longueur sur 32 bits, puis ses
//! octets. Un processus enfant parle à son parent par son entrée et sa sortie
//! standard, que le parent a remplacées par des tubes ; il ne lui faut donc
//! aucun droit d'accès au système pour communiquer, ce qui convient à un bac
//! à sable.

use std::io::{self, Read, Write};
use std::marker::PhantomData;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use crate::codec::{Decode, DecodeError, Encode, from_bytes, to_bytes};

/// Taille maximale d'une trame : au-delà, le correspondant est tenu pour
/// défaillant.
pub const MAX_FRAME: usize = 256 * 1024 * 1024;

/// Erreur de communication.
#[derive(Debug)]
pub enum ChannelError {
    /// Le correspondant a fermé le canal (ou son processus s'est arrêté).
    Closed,
    /// Erreur d'entrée-sortie.
    Io(io::Error),
    /// Trame plus grande que [`MAX_FRAME`].
    Oversized(usize),
    /// Message illisible.
    Decode(DecodeError),
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => f.write_str("canal fermé"),
            Self::Io(error) => write!(f, "erreur d'entrée-sortie : {error}"),
            Self::Oversized(len) => write!(f, "trame trop grande ({len} octets)"),
            Self::Decode(error) => write!(f, "message illisible : {error}"),
        }
    }
}

impl std::error::Error for ChannelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Decode(error) => Some(error),
            Self::Closed | Self::Oversized(_) => None,
        }
    }
}

/// Écrit une trame.
///
/// # Errors
///
/// [`ChannelError::Oversized`] si `payload` dépasse [`MAX_FRAME`], ou
/// l'erreur d'écriture.
pub fn write_frame(writer: &mut impl Write, payload: &[u8]) -> Result<(), ChannelError> {
    if payload.len() > MAX_FRAME {
        return Err(ChannelError::Oversized(payload.len()));
    }
    let len = u32::try_from(payload.len()).map_err(|_| ChannelError::Oversized(payload.len()))?;
    let closed = |error: io::Error| {
        if error.kind() == io::ErrorKind::BrokenPipe {
            ChannelError::Closed
        } else {
            ChannelError::Io(error)
        }
    };
    writer.write_all(&len.to_le_bytes()).map_err(closed)?;
    writer.write_all(payload).map_err(closed)?;
    writer.flush().map_err(closed)
}

/// Lit une trame.
///
/// # Errors
///
/// [`ChannelError::Closed`] si le flux s'arrête, [`ChannelError::Oversized`]
/// si la trame annoncée dépasse [`MAX_FRAME`], ou l'erreur de lecture.
pub fn read_frame(reader: &mut impl Read) -> Result<Vec<u8>, ChannelError> {
    let ended = |error: io::Error| match error.kind() {
        io::ErrorKind::UnexpectedEof | io::ErrorKind::BrokenPipe => ChannelError::Closed,
        _ => ChannelError::Io(error),
    };
    let mut header = [0u8; 4];
    reader.read_exact(&mut header).map_err(ended)?;
    let len = usize::try_from(u32::from_le_bytes(header)).unwrap_or(usize::MAX);
    if len > MAX_FRAME {
        return Err(ChannelError::Oversized(len));
    }
    // La trame est lue par morceaux : une longueur mensongère ne réserve pas
    // plus de mémoire que le correspondant n'envoie d'octets.
    let mut payload = Vec::new();
    let mut limited = reader.take(len as u64);
    limited.read_to_end(&mut payload).map_err(ended)?;
    if payload.len() != len {
        return Err(ChannelError::Closed);
    }
    Ok(payload)
}

/// Extrémité d'émission d'un canal ; se clone pour être partagée entre fils.
pub struct Sender<T> {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    message: PhantomData<fn(&T)>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            writer: Arc::clone(&self.writer),
            message: PhantomData,
        }
    }
}

impl<T> std::fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sender").finish_non_exhaustive()
    }
}

impl<T: Encode> Sender<T> {
    /// Émetteur qui écrit dans `writer`.
    pub fn new(writer: impl Write + Send + 'static) -> Self {
        Self {
            writer: Arc::new(Mutex::new(Box::new(writer))),
            message: PhantomData,
        }
    }

    /// Envoie `message`.
    ///
    /// # Errors
    ///
    /// [`ChannelError::Closed`] si le correspondant n'écoute plus, ou une
    /// autre erreur d'écriture.
    pub fn send(&self, message: &T) -> Result<(), ChannelError> {
        let payload = to_bytes(message);
        // Un verrou empoisonné ne laisse rien d'incohérent : au pire une
        // trame interrompue, que le correspondant refusera.
        let mut writer = self.writer.lock().unwrap_or_else(PoisonError::into_inner);
        write_frame(&mut *writer, &payload)
    }
}

/// Extrémité de réception d'un canal. Un fil dédié lit et décode les
/// messages ; ils attendent ici dans l'ordre d'arrivée.
pub struct Receiver<T> {
    queue: mpsc::Receiver<Result<T, ChannelError>>,
    /// Vrai une fois la fin du canal constatée.
    closed: bool,
}

impl<T> std::fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Receiver")
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl<T: Decode + Send + 'static> Receiver<T> {
    /// Récepteur qui lit dans `reader`. `wake` est appelé, depuis le fil de
    /// lecture, chaque fois qu'un message (ou la fin du canal) est en
    /// attente : de quoi réveiller une boucle d'événements.
    pub fn new(reader: impl Read + Send + 'static, wake: impl Fn() + Send + 'static) -> Self {
        let (queue_in, queue) = mpsc::channel();
        let mut reader = reader;
        // Le fil s'arrête avec le flux, ou quand le récepteur disparaît.
        let spawned = std::thread::Builder::new()
            .name("oxyde-ipc".to_owned())
            .spawn(move || {
                loop {
                    let message = read_frame(&mut reader).and_then(|payload| {
                        from_bytes::<T>(&payload).map_err(ChannelError::Decode)
                    });
                    let last = message.is_err();
                    let delivered = queue_in.send(message).is_ok();
                    wake();
                    if last || !delivered {
                        break;
                    }
                }
            });
        if spawned.is_err() {
            // Pas de fil de lecture : le canal se présente comme fermé.
            return Self {
                queue: mpsc::channel().1,
                closed: true,
            };
        }
        Self {
            queue,
            closed: false,
        }
    }

    fn note(&mut self, message: Result<T, ChannelError>) -> Result<T, ChannelError> {
        if message.is_err() {
            self.closed = true;
        }
        message
    }

    /// Message en attente, s'il y en a un ; `Ok(None)` sinon.
    ///
    /// # Errors
    ///
    /// L'erreur qui a mis fin au canal ; [`ChannelError::Closed`] ensuite.
    pub fn try_recv(&mut self) -> Result<Option<T>, ChannelError> {
        if self.closed {
            return Err(ChannelError::Closed);
        }
        match self.queue.try_recv() {
            Ok(message) => self.note(message).map(Some),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                self.closed = true;
                Err(ChannelError::Closed)
            }
        }
    }

    /// Attend le prochain message.
    ///
    /// # Errors
    ///
    /// L'erreur qui a mis fin au canal ; [`ChannelError::Closed`] ensuite.
    pub fn recv(&mut self) -> Result<T, ChannelError> {
        if self.closed {
            return Err(ChannelError::Closed);
        }
        match self.queue.recv() {
            Ok(message) => self.note(message),
            Err(mpsc::RecvError) => {
                self.closed = true;
                Err(ChannelError::Closed)
            }
        }
    }

    /// Attend le prochain message, `timeout` au plus ; `Ok(None)` si rien
    /// n'est arrivé.
    ///
    /// # Errors
    ///
    /// L'erreur qui a mis fin au canal ; [`ChannelError::Closed`] ensuite.
    pub fn recv_timeout(&mut self, timeout: Duration) -> Result<Option<T>, ChannelError> {
        if self.closed {
            return Err(ChannelError::Closed);
        }
        match self.queue.recv_timeout(timeout) {
            Ok(message) => self.note(message).map(Some),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.closed = true;
                Err(ChannelError::Closed)
            }
        }
    }

    /// Vrai une fois la fin du canal constatée.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.closed
    }
}

/// Processus enfant relié à son parent par un canal.
#[derive(Debug)]
pub struct ChildProcess<S, R> {
    /// Le processus lui-même.
    pub process: Child,
    /// Messages vers l'enfant.
    pub sender: Sender<S>,
    /// Messages venus de l'enfant.
    pub receiver: Receiver<R>,
}

/// Lance `command` et le relie par son entrée et sa sortie standard. Sa
/// sortie d'erreur reste celle du parent : c'est là qu'il journalise.
///
/// # Errors
///
/// L'erreur de lancement du processus.
pub fn spawn<S: Encode, R: Decode + Send + 'static>(
    command: &mut Command,
    wake: impl Fn() + Send + 'static,
) -> io::Result<ChildProcess<S, R>> {
    let mut process = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let missing = || io::Error::other("tube du processus enfant indisponible");
    let (Some(stdin), Some(stdout)) = (process.stdin.take(), process.stdout.take()) else {
        // Sans ses tubes, l'enfant est inutilisable : on ne le laisse pas traîner.
        let _ = process.kill();
        let _ = process.wait();
        return Err(missing());
    };
    Ok(ChildProcess {
        process,
        sender: Sender::new(stdin),
        receiver: Receiver::new(stdout, wake),
    })
}

/// Côté enfant : canal vers le parent, par l'entrée et la sortie standard.
/// Le processus ne doit plus rien écrire d'autre sur sa sortie standard.
#[must_use]
pub fn connect_to_parent<S: Encode, R: Decode + Send + 'static>() -> (Sender<S>, Receiver<R>) {
    (Sender::new(io::stdout()), Receiver::new(io::stdin(), || {}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc_enum;

    #[derive(Debug, PartialEq)]
    enum Note {
        Hello(String),
        Count(u32),
    }
    ipc_enum!(Note {
        0 => Hello(text),
        1 => Count(value),
    });

    #[test]
    fn frames_round_trip_and_reject_lies() {
        let mut wire = Vec::new();
        write_frame(&mut wire, b"abc").expect("écriture");
        write_frame(&mut wire, b"").expect("écriture");
        let mut reader = wire.as_slice();
        assert_eq!(read_frame(&mut reader).expect("trame"), b"abc");
        assert_eq!(read_frame(&mut reader).expect("trame"), b"");
        assert!(matches!(read_frame(&mut reader), Err(ChannelError::Closed)));
        // Trame annoncée plus longue que le flux.
        let mut short = [5u8, 0, 0, 0, 1, 2].as_slice();
        assert!(matches!(read_frame(&mut short), Err(ChannelError::Closed)));
        // Trame annoncée au-delà de la limite : refusée sans rien lire.
        let mut huge = [0xff_u8, 0xff, 0xff, 0xff].as_slice();
        assert!(matches!(
            read_frame(&mut huge),
            Err(ChannelError::Oversized(_))
        ));
    }

    #[test]
    fn channel_delivers_in_order_then_reports_the_end() {
        let (reader, writer) = io::pipe().expect("tube");
        let woken = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let counter = Arc::clone(&woken);
        let sender: Sender<Note> = Sender::new(writer);
        let mut receiver: Receiver<Note> = Receiver::new(reader, move || {
            counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });
        sender
            .send(&Note::Hello("bonjour".to_owned()))
            .expect("envoi");
        sender.clone().send(&Note::Count(7)).expect("envoi");
        assert_eq!(
            receiver.recv().expect("message"),
            Note::Hello("bonjour".to_owned())
        );
        assert_eq!(
            receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("message"),
            Some(Note::Count(7))
        );
        assert!(matches!(receiver.try_recv(), Ok(None)));
        // L'émetteur disparaît : le récepteur voit la fin du canal.
        drop(sender);
        assert!(matches!(receiver.recv(), Err(ChannelError::Closed)));
        assert!(receiver.is_closed());
        assert!(matches!(receiver.try_recv(), Err(ChannelError::Closed)));
        assert_eq!(woken.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[test]
    fn garbage_ends_the_channel_with_a_decode_error() {
        let (reader, mut writer) = io::pipe().expect("tube");
        let mut receiver: Receiver<Note> = Receiver::new(reader, || {});
        write_frame(&mut writer, &[42, 1, 2, 3]).expect("écriture");
        assert!(matches!(
            receiver.recv(),
            Err(ChannelError::Decode(DecodeError::UnknownVariant(42)))
        ));
        assert!(matches!(receiver.recv(), Err(ChannelError::Closed)));
    }
}
