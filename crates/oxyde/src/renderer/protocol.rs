//! Messages échangés entre le processus navigateur et un processus de rendu.

use oxyde_content::{FetchRequest, FetchResponse, FormPost, Navigation};
use oxyde_ipc::{Bytes, ipc_enum, ipc_struct};
use oxyde_paint::wire::WireList;

use crate::loader::LoadError;

/// Opération réseau qu'un processus de rendu demande au navigateur : lui
/// n'ouvre ni fichier ni connexion.
#[derive(Debug)]
pub(crate) enum NetworkRequest {
    /// Récupérer une ressource pour le document `initiator`.
    Get {
        url: String,
        initiator: Option<String>,
    },
    /// Demander une page par `POST` (soumission d'un formulaire).
    Post { url: String, form: FormPost },
    /// Requête `fetch` d'un script.
    Script(Box<FetchRequest>),
    /// Cookies que les scripts du document peuvent lire.
    Cookies { document: String },
    /// Cookie écrit par un script du document.
    SetCookie { document: String, text: String },
}
ipc_enum!(NetworkRequest {
    0 => Get { url, initiator },
    1 => Post { url, form },
    2 => Script(request),
    3 => Cookies { document },
    4 => SetCookie { document, text },
});

/// Réponse à une [`NetworkRequest`].
#[derive(Debug)]
pub(crate) enum NetworkReply {
    /// Adresse finale et octets d'une ressource, ou l'erreur de chargement.
    Resource(Result<(String, Bytes), LoadError>),
    /// Réponse à une requête `fetch`, ou le message d'échec pour le script.
    Script(Result<FetchResponse, String>),
    /// Valeur de `document.cookie`.
    Cookies(String),
    /// Opération sans résultat.
    Done,
}
ipc_enum!(NetworkReply {
    0 => Resource(result),
    1 => Script(result),
    2 => Cookies(text),
    3 => Done,
});

/// Touche transmise à la page.
#[derive(Debug)]
pub(crate) struct KeyMessage {
    pub(crate) down: bool,
    pub(crate) key: String,
    pub(crate) code: String,
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    pub(crate) alt: bool,
    pub(crate) meta: bool,
    pub(crate) repeat: bool,
}
ipc_struct!(KeyMessage {
    down,
    key,
    code,
    ctrl,
    shift,
    alt,
    meta,
    repeat
});

/// Du navigateur vers le processus de rendu.
#[derive(Debug)]
pub(crate) enum ToRenderer {
    /// Charger `url` dans une fenêtre de `width` × `height`.
    Load {
        url: String,
        post: Option<FormPost>,
        width: f32,
        height: f32,
    },
    /// La zone d'affichage a changé de taille.
    Resize { width: f32, height: f32 },
    /// Clic au point (`x`, `y`) de la page, défilement compris.
    Click { x: f32, y: f32 },
    /// Le pointeur est au point (`x`, `y`) de la page.
    Hover { x: f32, y: f32 },
    /// Touche enfoncée ou relâchée.
    Key(KeyMessage),
    /// Le compositeur a fait défiler la zone `id`.
    Scroll { id: u32, x: f32, y: f32 },
    /// Parcours de l'historique au sein du même document.
    Traverse { url: String, delta: i32 },
    /// Réponse à la requête réseau `id`.
    Network { id: u64, reply: NetworkReply },
    /// Chercher `query` dans le texte de la page ; vide : effacer la recherche.
    Find { query: String },
    /// Passer à l'occurrence suivante ou précédente.
    FindStep { forward: bool },
}
ipc_enum!(ToRenderer {
    0 => Load { url, post, width, height },
    1 => Resize { width, height },
    2 => Click { x, y },
    3 => Hover { x, y },
    4 => Key(key),
    5 => Scroll { id, x, y },
    6 => Traverse { url, delta },
    7 => Network { id, reply },
    8 => Find { query },
    9 => FindStep { forward },
});

/// Image de la page, prête à composer.
#[derive(Debug)]
pub(crate) struct Frame {
    pub(crate) list: WireList,
    /// Largeur et hauteur du contenu.
    pub(crate) content: [f32; 2],
    /// Couleur de fond du canevas.
    pub(crate) background: [u8; 4],
    /// Zones que les scripts ont fait défiler depuis l'image précédente.
    pub(crate) scroll: Vec<(u32, f32, f32)>,
    pub(crate) url: String,
    pub(crate) title: String,
}
ipc_struct!(Frame {
    list,
    content,
    background,
    scroll,
    url,
    title
});

/// Du processus de rendu vers le navigateur.
#[derive(Debug)]
pub(crate) enum FromRenderer {
    /// Requête réseau ; la réponse porte le même `id`.
    Network { id: u64, request: NetworkRequest },
    /// Le chargement demandé est terminé : adresse finale, et l'erreur s'il
    /// a échoué.
    Committed {
        url: String,
        error: Option<LoadError>,
    },
    /// Nouvelle image de la page.
    Frame(Box<Frame>),
    /// Navigation demandée par un script.
    Navigate(Navigation),
    /// Clic sur un lien, que les scripts n'ont pas annulé.
    Clicked { url: String },
    /// Le pointeur est, ou n'est pas, au-dessus d'un lien.
    Hover { link: bool },
    /// Avancement de la recherche : nombre d'occurrences, rang de la courante.
    Found { count: u32, current: u32 },
}
ipc_enum!(FromRenderer {
    0 => Network { id, request },
    1 => Committed { url, error },
    2 => Frame(frame),
    3 => Navigate(navigation),
    4 => Clicked { url },
    5 => Hover { link },
    6 => Found { count, current },
});
