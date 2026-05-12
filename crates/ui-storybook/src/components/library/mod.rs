//! Library components — sidebar with storage meter, recording cards in a grid.
//! Filled in across UI-14 / UI-15.

pub mod library_sidebar;

pub use library_sidebar::{
    LibraryNavItemView, LibrarySectionView, LibrarySidebar, LibrarySidebarView, StorageMeter,
    StorageMeterView, storage_percent,
};
