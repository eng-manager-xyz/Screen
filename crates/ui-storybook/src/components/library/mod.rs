//! Library components — sidebar with storage meter, recording cards in a grid.
//! Filled in across UI-14 / UI-15.

pub mod library_sidebar;
pub mod recording_card;

pub use library_sidebar::{
    LibraryNavItemView, LibrarySectionView, LibrarySidebar, LibrarySidebarView, StorageMeter,
    StorageMeterView, storage_percent,
};
pub use recording_card::{
    LibraryGrid, LibraryGridView, LibraryLayoutMode, LibraryToolbar, LibraryToolbarView,
    RecordingCard, RecordingCardState, RecordingCardView, RecordingFilterView,
    RecordingMetricsView, ThumbnailView,
};
