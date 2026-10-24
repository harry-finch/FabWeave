//! Profils livrés, lus pour les tests du moteur.

use crate::profile::{parse_filament, parse_printer, FilamentProfile, PrinterProfile};

pub const MK4S: &str = include_str!("../../../profiles/printers/prusa_mk4s.toml");
pub const CORE_ONE: &str = include_str!("../../../profiles/printers/prusa_core_one.toml");
pub const PLA: &str = include_str!("../../../profiles/filaments/generic_pla.toml");
pub const PETG: &str = include_str!("../../../profiles/filaments/generic_petg.toml");

pub fn printer() -> PrinterProfile {
    parse_printer(MK4S).expect("profil MK4S")
}

pub fn core_one() -> PrinterProfile {
    parse_printer(CORE_ONE).expect("profil CORE One")
}

pub fn filament() -> FilamentProfile {
    parse_filament(PLA).expect("profil PLA")
}

pub fn petg() -> FilamentProfile {
    parse_filament(PETG).expect("profil PETG")
}
