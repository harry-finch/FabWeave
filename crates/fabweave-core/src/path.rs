//! Parcours produit par un modèle : une suite de points enrichis (position,
//! section du boudin, nature du mouvement, vitesse demandée, ventilateur).
//!
//! Repère du modèle : millimètres, centré sur (0, 0), Z vers le haut, Z = 0
//! au plateau. Le premier point est rejoint par un déplacement ; chaque point
//! suivant décrit le mouvement depuis le point précédent.

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum MoveKind {
    /// Déplacement sans extrusion (rétraction et saut en Z selon le filament).
    Travel = 0,
    /// Impression d'un boudin de section W × H.
    Print = 1,
    /// Impression dans le vide (pont tendu) : débit corrigé, ventilateur forcé.
    Bridge = 2,
}

/// Vitesse demandée par le script.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Speed {
    /// Multiple de la vitesse nominale du filament.
    Factor(f64),
    /// Vitesse en mm/s.
    Absolute(f64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WayPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Largeur du boudin (mm).
    pub w: f64,
    /// Hauteur du boudin (mm).
    pub h: f64,
    pub kind: MoveKind,
    pub speed: Speed,
    /// Ventilateur imposé par le script (%), sinon celui du filament.
    pub fan: Option<u8>,
    /// Multiplicateur de débit propre au mouvement (ponts), 1 sinon.
    pub flow: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Bounds {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl Bounds {
    pub fn size(&self) -> [f64; 3] {
        [
            self.max[0] - self.min[0],
            self.max[1] - self.min[1],
            self.max[2] - self.min[2],
        ]
    }

    pub fn center(&self) -> [f64; 3] {
        [
            (self.min[0] + self.max[0]) / 2.0,
            (self.min[1] + self.max[1]) / 2.0,
            (self.min[2] + self.max[2]) / 2.0,
        ]
    }
}

/// Annotation affichée dans la vue 3D (repère du modèle).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Note {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub text: crate::model::Text,
}

#[derive(Clone, Debug, Default)]
pub struct Toolpath {
    pub points: Vec<WayPoint>,
    pub notes: Vec<Note>,
}

impl Toolpath {
    /// Encombrement de tous les points (déplacements compris).
    pub fn bounds(&self) -> Option<Bounds> {
        let mut it = self.points.iter();
        let first = it.next()?;
        let mut b = Bounds {
            min: [first.x, first.y, first.z],
            max: [first.x, first.y, first.z],
        };
        for p in it {
            for (i, v) in [p.x, p.y, p.z].into_iter().enumerate() {
                b.min[i] = b.min[i].min(v);
                b.max[i] = b.max[i].max(v);
            }
        }
        Some(b)
    }

    /// Nombre de mouvements qui déposent de la matière.
    pub fn extruding_moves(&self) -> usize {
        self.points
            .iter()
            .skip(1)
            .filter(|p| p.kind != MoveKind::Travel)
            .count()
    }
}
