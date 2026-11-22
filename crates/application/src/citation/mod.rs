//! 39 引用对齐门面：只读判定与唯一投影。

pub mod align;

pub use align::{check_citation_alignment, project_alignment, CitationDiagnostics};
