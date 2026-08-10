//! Content-authoring subcommands that operate on a project that already
//! exists: `page new`, `image add`, `page add-image`, `social
//! add`/`update`/`remove`, `footer set`, `menu add-item`. Unlike
//! `generate`/`check`, which read a whole source and image tree, and unlike
//! `init`, which creates one from nothing, each of these touches one file
//! for one reason -- see `vados.allium`'s comment introducing
//! `PageCreationRun` and its five siblings.

pub mod image_registry;
pub mod menu;
pub mod page;
pub mod social;
