//! Messages échangés entre le processus navigateur et un processus de rendu.

use oxyde_content::{FetchRequest, FetchResponse, FormPost, MouseInput, Navigation};
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

/// Geste de sélection de texte à la souris.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectGesture {
    /// Bouton enfoncé : la sélection commence ici, l'ancienne s'efface.
    Start,
    /// Pointeur déplacé, bouton enfoncé : la sélection va jusqu'ici.
    Extend,
    /// Double clic : le mot sous le pointeur.
    Word,
}

impl SelectGesture {
    /// Code du geste dans un message.
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Start => 0,
            Self::Extend => 1,
            Self::Word => 2,
        }
    }

    /// Geste de code `code` ; `None` pour un code inconnu.
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Start),
            1 => Some(Self::Extend),
            2 => Some(Self::Word),
            _ => None,
        }
    }

    /// Applique le geste à `page` ; renvoie `true` si l'affichage a changé.
    pub(crate) fn apply(self, page: &mut oxyde_content::Page, x: f32, y: f32) -> bool {
        match self {
            Self::Start => page.select_start(x, y),
            Self::Extend => page.select_to(x, y),
            Self::Word => page.select_word(x, y),
        }
    }
}

/// Événement de souris transmis à une page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MouseKind {
    /// Le pointeur s'est déplacé.
    Move,
    /// Un bouton est enfoncé.
    Down,
    /// Un bouton est relâché.
    Up,
}

impl MouseKind {
    /// Code de l'événement dans un message.
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Move => 0,
            Self::Down => 1,
            Self::Up => 2,
        }
    }

    /// Événement de code `code` ; `None` pour un code inconnu.
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Move),
            1 => Some(Self::Down),
            2 => Some(Self::Up),
            _ => None,
        }
    }

    /// Transmet l'événement à `page`, `input` étant dans le repère de la
    /// page ; renvoie `true` si elle doit être redessinée.
    pub(crate) fn apply(self, page: &mut oxyde_content::Page, input: &MouseInput) -> bool {
        match self {
            Self::Move => page.mouse_move(input),
            Self::Down => page.mouse_button(true, input),
            Self::Up => page.mouse_button(false, input),
        }
    }
}

/// Touches tenues pendant un événement de souris, sur un octet : Ctrl 1,
/// Maj 2, Alt 4, Méta 8.
pub(crate) fn pack_modifiers(input: &MouseInput) -> u8 {
    u8::from(input.ctrl)
        | u8::from(input.shift) << 1
        | u8::from(input.alt) << 2
        | u8::from(input.meta) << 3
}

/// Pointeur d'un message : l'inverse de [`pack_modifiers`].
pub(crate) fn mouse_input(x: f32, y: f32, button: u8, detail: u8, modifiers: u8) -> MouseInput {
    MouseInput {
        x,
        y,
        button,
        buttons: 0,
        detail,
        ctrl: modifiers & 1 != 0,
        shift: modifiers & 2 != 0,
        alt: modifiers & 4 != 0,
        meta: modifiers & 8 != 0,
    }
}

/// Action d'édition choisie dans un menu contextuel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditAction {
    /// Copier le texte sélectionné.
    Copy,
    /// Tout sélectionner.
    SelectAll,
}

impl EditAction {
    /// Code de l'action dans un message.
    pub(crate) fn code(self) -> u8 {
        match self {
            Self::Copy => 0,
            Self::SelectAll => 1,
        }
    }

    /// Action de code `code` ; `None` pour un code inconnu.
    pub(crate) fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::Copy),
            1 => Some(Self::SelectAll),
            _ => None,
        }
    }

    /// Applique l'action à `page` ; renvoie `true` si l'affichage a changé.
    pub(crate) fn apply(self, page: &mut oxyde_content::Page) -> bool {
        match self {
            Self::Copy => {
                page.copy();
                false
            }
            Self::SelectAll => page.select_everything(),
        }
    }
}

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
    /// Geste de sélection de texte au point (`x`, `y`) de la page :
    /// `gesture` est un [`SelectGesture`] (0 commencer, 1 étendre, 2 mot).
    Select { gesture: u8, x: f32, y: f32 },
    /// Coller `text` (Ctrl V) dans le champ qui a le focus.
    Paste { text: String },
    /// Demander ce qu'un menu contextuel ouvert au point (`x`, `y`) de la
    /// page doit proposer ; la réponse est [`FromRenderer::Context`].
    Context { x: f32, y: f32 },
    /// Action d'édition choisie dans un menu : `action` est un
    /// [`EditAction`] (0 copier, 1 tout sélectionner).
    Edit { action: u8 },
    /// Événement de souris au point (`x`, `y`) de la page : `kind` est un
    /// [`MouseKind`] (0 déplacement, 1 bouton enfoncé, 2 relâché),
    /// `modifiers` les touches tenues (voir [`pack_modifiers`]).
    Mouse {
        kind: u8,
        x: f32,
        y: f32,
        button: u8,
        detail: u8,
        modifiers: u8,
    },
    /// Molette de (`dx`, `dy`) au point (`x`, `y`) de la fenêtre. `scroll` :
    /// le processus de rendu fait défiler lui-même si aucun script
    /// n'annule ; sinon le navigateur a déjà défilé.
    Wheel {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        modifiers: u8,
        scroll: bool,
    },
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
    10 => Select { gesture, x, y },
    11 => Paste { text },
    12 => Context { x, y },
    13 => Edit { action },
    14 => Mouse { kind, x, y, button, detail, modifiers },
    15 => Wheel { x, y, dx, dy, modifiers, scroll },
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
    /// Écouteurs de molette de la page : 0 aucun, 1 des passifs seulement,
    /// 2 au moins un qui peut annuler le défilement.
    pub(crate) wheel: u8,
}
ipc_struct!(Frame {
    list,
    content,
    background,
    scroll,
    url,
    title,
    wheel
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
    /// Texte que l'utilisateur a copié dans la page (Ctrl C).
    Copied { text: String },
    /// Réponse à [`ToRenderer::Context`] : lien sous le pointeur, texte
    /// sélectionné, champ de saisie qui a le focus.
    /// `prevented` : un script a annulé `contextmenu`.
    Context {
        link: Option<String>,
        selection: bool,
        editable: bool,
        prevented: bool,
    },
}
ipc_enum!(FromRenderer {
    0 => Network { id, request },
    1 => Committed { url, error },
    2 => Frame(frame),
    3 => Navigate(navigation),
    4 => Clicked { url },
    5 => Hover { link },
    6 => Found { count, current },
    7 => Copied { text },
    8 => Context { link, selection, editable, prevented },
});
