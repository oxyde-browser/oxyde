//! Jetons de design repris de la maquette « Oxyde — navigateur Rust ».

use oxyde_css::Color;

/// Hauteur de la barre de titre.
pub const TITLE_BAR: f32 = 40.0;
/// Largeur de la barre latérale.
pub const SIDEBAR: f32 = 264.0;

/// Fond de la fenêtre.
pub const WINDOW: Color = Color::rgb(0x0E, 0x11, 0x16);
/// Surface (barre d'adresse, menus).
pub const SURFACE: Color = Color::rgb(0x17, 0x1C, 0x23);
/// Fond d'un groupe d'onglets (vue scindée).
pub const GROUP: Color = Color::rgb(0x15, 0x1A, 0x21);
/// Fond de l'onglet actif.
pub const ACTIVE: Color = Color::rgb(0x22, 0x2A, 0x34);
/// Fond d'un élément survolé.
pub const HOVER: Color = Color::rgb(0x1A, 0x20, 0x28);
/// Fond d'un bouton survolé posé sur un élément déjà mis en avant.
pub const HOVER_STRONG: Color = Color::rgb(0x2E, 0x37, 0x43);
/// Fond d'une pastille (espace actif, bascule activée).
pub const PILL: Color = Color::rgb(0x1C, 0x22, 0x2B);
/// Bordure d'un champ ou d'un panneau flottant.
pub const BORDER: Color = Color::rgb(0x2A, 0x31, 0x3B);
/// Bordure discrète.
pub const BORDER_SOFT: Color = Color::rgb(0x26, 0x2D, 0x37);
/// Bordure d'une indication de raccourci.
pub const BORDER_HINT: Color = Color::rgb(0x32, 0x3A, 0x45);
/// Texte principal.
pub const TEXT: Color = Color::rgb(0xE6, 0xE9, 0xED);
/// Titre d'un onglet inactif.
pub const TEXT_TAB: Color = Color::rgb(0xD5, 0xDA, 0xE1);
/// Texte secondaire.
pub const TEXT_MUTED: Color = Color::rgb(0x9A, 0xA3, 0xAE);
/// Explication sous le titre d'un réglage.
pub const TEXT_SECONDARY: Color = Color::rgb(0xA3, 0xAD, 0xB9);
/// Fond d'un interrupteur éteint, et du choix retenu dans un groupe.
pub const SWITCH_OFF: Color = Color::rgb(0x2A, 0x32, 0x3D);
/// Icône.
pub const ICON: Color = Color::rgb(0xC3, 0xCA, 0xD3);
/// Icône désactivée.
pub const ICON_DISABLED: Color = Color::rgb(0x7F, 0x89, 0x96);
/// Blanc.
pub const WHITE: Color = Color::rgb(0xFF, 0xFF, 0xFF);
/// Couleur d'accent par défaut.
pub const ACCENT: Color = Color::rgb(0x4C, 0xC9, 0xA8);
/// Second accent (point du logo).
pub const AMBER: Color = Color::rgb(0xE0, 0xA4, 0x58);
/// Sélection de texte dans la barre d'adresse.
pub const SELECTION: Color = Color::rgb(0x1F, 0x4F, 0x45);
/// Survol du bouton de fermeture de la fenêtre.
pub const DANGER: Color = Color::rgb(0xC4, 0x2B, 0x1C);
/// En-tête d'un volet de la vue scindée.
pub const PANE_HEADER: Color = Color::rgb(0xE7, 0xEA, 0xEE);
/// Texte de l'en-tête d'un volet.
pub const PANE_HEADER_TEXT: Color = Color::rgb(0x41, 0x49, 0x55);
/// Survol dans l'en-tête d'un volet.
pub const PANE_HEADER_HOVER: Color = Color::rgb(0xD3, 0xD8, 0xDF);
/// Couleurs des pastilles d'onglets.
pub const FAVICONS: [Color; 4] = [
    Color::rgb(0x2F, 0x6F, 0x60),
    Color::rgb(0x7A, 0x5A, 0x1E),
    Color::rgb(0x2B, 0x4E, 0x86),
    Color::rgb(0x5B, 0x3F, 0x78),
];

/// Troisième couleur d'espace.
pub const BLUE: Color = Color::rgb(0x8F, 0xB4, 0xF2);
/// Couleurs des initiales des onglets épinglés.
pub const PINNED_LETTERS: [Color; 4] = [BLUE, AMBER, ACCENT, Color::rgb(0xD9, 0xA0, 0xE8)];
/// Fond d'une page interne (nouvel onglet, erreur).
pub const PANEL: Color = Color::rgb(0x12, 0x16, 0x1C);
/// Bordure d'une page interne.
pub const PANEL_BORDER: Color = Color::rgb(0x22, 0x28, 0x31);
/// Anneau de focus du champ de recherche (accent à 18 %).
pub const FOCUS_RING: Color = Color {
    r: 0x4C,
    g: 0xC9,
    b: 0xA8,
    a: 46,
};
/// Texte posé sur la couleur d'accent.
pub const ON_ACCENT: Color = Color::rgb(0x06, 0x23, 0x1C);
/// Fond de la page d'erreur de certificat.
pub const DANGER_PANEL: Color = Color::rgb(0x1A, 0x12, 0x13);
/// Bordure de la page d'erreur de certificat.
pub const DANGER_BORDER: Color = Color::rgb(0x4A, 0x2A, 0x2A);
/// Rouge clair : icône et bouton principal de l'erreur de certificat.
pub const DANGER_SOFT: Color = Color::rgb(0xF0, 0xA7, 0xA0);
/// Texte posé sur le rouge clair.
pub const DANGER_ON_SOFT: Color = Color::rgb(0x2A, 0x0F, 0x0D);
/// Texte courant de l'erreur de certificat.
pub const DANGER_TEXT: Color = Color::rgb(0xE0, 0xCF, 0xCD);
/// Texte secondaire de l'erreur de certificat.
pub const DANGER_MUTED: Color = Color::rgb(0xC9, 0xA9, 0xA6);
/// Bouton secondaire de l'erreur de certificat.
pub const DANGER_BUTTON: Color = Color::rgb(0x2E, 0x1D, 0x1E);
