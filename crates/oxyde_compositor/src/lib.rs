//! Compositeur (pendant de cc et Viz) : l'arbre de défilement.
//!
//! La peinture ne fige pas la position du contenu des zones défilantes : la
//! liste d'affichage désigne chaque zone par un identifiant, et son décalage
//! n'est appliqué qu'à la composition. Faire défiler la fenêtre ou un élément
//! ne redemande donc ni calcul de style, ni mise en page, ni peinture : seul
//! ce décalage change. L'[`ScrollTree`] le garde, le borne à l'étendue du
//! contenu et route la molette vers la zone située sous le pointeur.
//!
//! LIMITES CONNUES : tout se fait encore sur le fil principal ; les animations
//! de `transform` et d'`opacity` repassent par le calcul de style au lieu
//! d'être jouées par le compositeur ; pas de défilement fluide ni d'inertie ;
//! pas de mise en cache du rendu par calque.

use std::collections::HashMap;

pub use oxyde_paint::{ROOT_SCROLLER, Scroller};

/// Écart en deçà duquel deux décalages sont tenus pour égaux.
const EPSILON: f32 = 0.01;

/// Décalage de défilement de la fenêtre et de chaque zone défilante.
#[derive(Clone, Debug, Default)]
pub struct ScrollTree {
    /// Zones relevées à la dernière peinture, chacune après son parent.
    scrollers: Vec<Scroller>,
    /// Décalage de chaque zone, par identifiant ; absent : pas de défilement.
    offsets: HashMap<u32, (f32, f32)>,
    /// Taille de la fenêtre et du contenu de la page.
    viewport: (f32, f32),
    content: (f32, f32),
    /// Incrémenté à chaque changement de décalage.
    version: u64,
    /// Zones dont le décalage a changé depuis le dernier relevé.
    changed: Vec<u32>,
}

impl ScrollTree {
    /// Arbre sans aucun défilement.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Compteur qui change avec chaque décalage : ce qui dépend du
    /// défilement est à refaire quand il a bougé.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Décalage de la zone `id` ([`ROOT_SCROLLER`] pour la fenêtre).
    #[must_use]
    pub fn offset(&self, id: u32) -> (f32, f32) {
        self.offsets.get(&id).copied().unwrap_or((0.0, 0.0))
    }

    /// Plus grand décalage de la zone `id`, d'après la dernière peinture ;
    /// `None` si elle n'y figurait pas.
    #[must_use]
    pub fn limit(&self, id: u32) -> Option<(f32, f32)> {
        if id == ROOT_SCROLLER {
            return Some((
                (self.content.0 - self.viewport.0).max(0.0),
                (self.content.1 - self.viewport.1).max(0.0),
            ));
        }
        self.scrollers.iter().find(|s| s.id == id).map(|s| {
            (
                (s.content.0 - s.port.w).max(0.0),
                (s.content.1 - s.port.h).max(0.0),
            )
        })
    }

    /// Note le décalage de `id` ; renvoie `true` s'il a changé.
    fn store(&mut self, id: u32, x: f32, y: f32) -> bool {
        let (old_x, old_y) = self.offset(id);
        if (old_x - x).abs() < EPSILON && (old_y - y).abs() < EPSILON {
            return false;
        }
        if x == 0.0 && y == 0.0 {
            self.offsets.remove(&id);
        } else {
            self.offsets.insert(id, (x, y));
        }
        self.version = self.version.wrapping_add(1);
        if !self.changed.contains(&id) {
            self.changed.push(id);
        }
        true
    }

    /// Prend acte d'une nouvelle peinture : zones défilantes, taille de la
    /// fenêtre et du contenu. Les décalages sont ramenés dans leurs bornes
    /// et ceux des zones disparues sont oubliés.
    pub fn update(&mut self, scrollers: &[Scroller], viewport: (f32, f32), content: (f32, f32)) {
        self.scrollers = scrollers.to_vec();
        self.viewport = viewport;
        self.content = content;
        let ids: Vec<u32> = self.offsets.keys().copied().collect();
        for id in ids {
            match self.limit(id) {
                Some((max_x, max_y)) => {
                    let (x, y) = self.offset(id);
                    self.store(id, x.clamp(0.0, max_x), y.clamp(0.0, max_y));
                }
                None => {
                    self.offsets.remove(&id);
                    self.version = self.version.wrapping_add(1);
                }
            }
        }
    }

    /// Fait défiler la zone `id` jusqu'à (`x`, `y`), dans la limite de `max`
    /// (que l'appelant tient de sa propre mise en page). Renvoie `true` si le
    /// décalage a changé.
    pub fn scroll_to(&mut self, id: u32, x: f32, y: f32, max: (f32, f32)) -> bool {
        let clamp = |value: f32, max: f32| {
            if value.is_finite() {
                value.clamp(0.0, max.max(0.0))
            } else {
                0.0
            }
        };
        self.store(id, clamp(x, max.0), clamp(y, max.1))
    }

    /// Fait défiler la zone `id` jusqu'à (`x`, `y`), dans les bornes
    /// relevées à la dernière peinture ; sans effet si elle n'y figurait
    /// pas. Renvoie `true` si le décalage a changé.
    pub fn scroll_within_limit(&mut self, id: u32, x: f32, y: f32) -> bool {
        match self.limit(id) {
            Some(max) => self.scroll_to(id, x, y, max),
            None => false,
        }
    }

    /// Décalages non nuls : (identifiant, x, y).
    #[must_use]
    pub fn offsets(&self) -> Vec<(u32, f32, f32)> {
        self.offsets
            .iter()
            .map(|(id, (x, y))| (*id, *x, *y))
            .collect()
    }

    /// Rang de la zone la plus profonde située sous le point (`x`, `y`) de
    /// la page (défilement de la fenêtre compris).
    fn scroller_at(&self, x: f32, y: f32) -> Option<usize> {
        let mut found = None;
        // Point dans le repère du contenu de chaque zone qui le contient.
        let mut local: Vec<Option<(f32, f32)>> = Vec::with_capacity(self.scrollers.len());
        for (index, scroller) in self.scrollers.iter().enumerate() {
            // Une zone n'est atteinte qu'à travers celles qui la contiennent.
            let point = match scroller.parent {
                None => Some((x, y)),
                Some(parent) => local.get(parent).copied().flatten(),
            };
            let inside = point.filter(|(px, py)| scroller.port.contains(*px, *py));
            local.push(inside.map(|(px, py)| {
                let (ox, oy) = self.offset(scroller.id);
                (px + ox, py + oy)
            }));
            if inside.is_some() {
                found = Some(index);
            }
        }
        found
    }

    /// Molette de (`dx`, `dy`) au point (`x`, `y`) de la fenêtre : la zone la
    /// plus profonde sous le pointeur qui peut encore défiler dans ce sens
    /// l'emporte, la fenêtre en dernier recours. Renvoie l'identifiant de la
    /// zone qui a défilé.
    pub fn wheel(&mut self, x: f32, y: f32, dx: f32, dy: f32) -> Option<u32> {
        let (root_x, root_y) = self.offset(ROOT_SCROLLER);
        let mut current = self.scroller_at(x + root_x, y + root_y);
        while let Some(index) = current {
            let scroller = self.scrollers[index];
            let (ox, oy) = self.offset(scroller.id);
            let max = (
                (scroller.content.0 - scroller.port.w).max(0.0),
                (scroller.content.1 - scroller.port.h).max(0.0),
            );
            let target_x = if scroller.wheel.0 { ox + dx } else { ox };
            let target_y = if scroller.wheel.1 { oy + dy } else { oy };
            if self.scroll_to(scroller.id, target_x, target_y, max) {
                return Some(scroller.id);
            }
            current = scroller.parent;
        }
        let max = self.limit(ROOT_SCROLLER).unwrap_or((0.0, 0.0));
        self.scroll_to(ROOT_SCROLLER, root_x + dx, root_y + dy, max)
            .then_some(ROOT_SCROLLER)
    }

    /// Vrai si un décalage a changé depuis le dernier [`ScrollTree::take_changed`].
    #[must_use]
    pub fn has_changed(&self) -> bool {
        !self.changed.is_empty()
    }

    /// Zones dont le décalage a changé depuis le dernier appel : celles qui
    /// doivent un événement `scroll`.
    pub fn take_changed(&mut self) -> Vec<u32> {
        std::mem::take(&mut self.changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oxyde_paint::Rect;

    fn scroller(id: u32, parent: Option<usize>, port: Rect, content: (f32, f32)) -> Scroller {
        Scroller {
            id,
            parent,
            port,
            content,
            wheel: (true, true),
        }
    }

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    #[test]
    fn wheel_goes_to_the_deepest_scroller_then_bubbles() {
        let mut tree = ScrollTree::new();
        let outer = scroller(1, None, rect(0.0, 0.0, 200.0, 100.0), (200.0, 300.0));
        let inner = scroller(2, Some(0), rect(10.0, 10.0, 100.0, 50.0), (100.0, 80.0));
        tree.update(&[outer, inner], (400.0, 300.0), (400.0, 1000.0));
        // Sous le pointeur : la zone intérieure, qui défile de 30 au plus.
        assert_eq!(tree.wheel(20.0, 20.0, 0.0, 20.0), Some(2));
        assert_eq!(tree.wheel(20.0, 20.0, 0.0, 20.0), Some(2));
        assert_eq!(tree.offset(2), (0.0, 30.0));
        // À bout de course, la molette passe à la zone qui la contient.
        assert_eq!(tree.wheel(20.0, 20.0, 0.0, 20.0), Some(1));
        assert_eq!(tree.offset(1), (0.0, 20.0));
        // Le contenu extérieur a défilé : la zone intérieure n'est plus au
        // même endroit de la fenêtre.
        assert_eq!(tree.wheel(20.0, 45.0, 0.0, -5.0), Some(1));
        // Hors de toute zone : la fenêtre.
        assert_eq!(tree.wheel(300.0, 200.0, 0.0, 50.0), Some(ROOT_SCROLLER));
        assert_eq!(tree.offset(ROOT_SCROLLER), (0.0, 50.0));
        assert_eq!(tree.take_changed(), vec![2, 1, ROOT_SCROLLER]);
        assert!(!tree.has_changed());
    }

    #[test]
    fn offsets_are_clamped_and_forgotten() {
        let mut tree = ScrollTree::new();
        let zone = scroller(7, None, rect(0.0, 0.0, 100.0, 100.0), (100.0, 400.0));
        tree.update(&[zone], (200.0, 200.0), (200.0, 200.0));
        assert!(tree.scroll_to(7, 50.0, 1000.0, (0.0, 300.0)));
        assert_eq!(tree.offset(7), (0.0, 300.0));
        assert!(
            !tree.scroll_to(7, 0.0, 300.0, (0.0, 300.0)),
            "rien n'a changé"
        );
        // Le contenu rétrécit : le décalage suit.
        let smaller = scroller(7, None, rect(0.0, 0.0, 100.0, 100.0), (100.0, 150.0));
        let version = tree.version();
        tree.update(&[smaller], (200.0, 200.0), (200.0, 200.0));
        assert_eq!(tree.offset(7), (0.0, 50.0));
        assert_ne!(tree.version(), version);
        // La zone disparaît : son décalage aussi.
        tree.update(&[], (200.0, 200.0), (200.0, 200.0));
        assert_eq!(tree.offset(7), (0.0, 0.0));
        // La fenêtre ne défile pas au-delà du contenu.
        assert_eq!(tree.wheel(10.0, 10.0, 0.0, 30.0), None);
        assert!(!tree.scroll_to(ROOT_SCROLLER, f32::NAN, f32::NAN, (10.0, 10.0)));
    }
}
