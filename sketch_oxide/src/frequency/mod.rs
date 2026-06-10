//! Frequency estimation algorithms

mod conservative_count_min;
mod count_min;
mod count_sketch;
pub mod elastic_sketch;
mod fcm_sketch;
pub mod frequent;
mod heavy_keeper;
mod nitrosketch;
pub mod removable_sketch;
pub mod salsa;
mod space_saving;
mod space_saving_pm;
mod spread_sketch;
mod tower_sketch;
mod waving_sketch;

pub use conservative_count_min::ConservativeCountMin;
pub use count_min::CountMinSketch;
pub use count_sketch::CountSketch;
pub use elastic_sketch::ElasticSketch;
pub use fcm_sketch::FcmSketch;
pub use frequent::{ErrorType, FrequentItems};
pub use heavy_keeper::HeavyKeeper;
pub use nitrosketch::{NitroSketch, NitroSketchStats};
pub use removable_sketch::RemovableUniversalSketch;
pub use salsa::SALSA;
pub use space_saving::SpaceSaving;
pub use space_saving_pm::SpaceSavingPlusMinus;
pub use spread_sketch::SpreadSketch;
pub use tower_sketch::TowerSketch;
pub use waving_sketch::WavingSketch;
