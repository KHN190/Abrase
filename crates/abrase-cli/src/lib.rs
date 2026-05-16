// Integration layer: glues the abrase compiler/frontend to the myriad runtime.
// Anything that needs to drive parse → typeck → compile → run lives here.

pub mod host;
