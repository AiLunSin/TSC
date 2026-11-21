//! 双通道去重合并的纯函数内核（B2.24）。

pub mod equivalence;
pub mod hash;
pub mod merger;
pub mod normalize;
pub mod policy;

pub use equivalence::{classify_equivalence, ngram_cosine_milli, PairBasis};
pub use hash::{fnv1a64, hamming_distance, simhash64};
pub use merger::{build_merged, MergedUtterance, Pairing};
pub use normalize::normalize_text;
pub use policy::{MergePolicy, MergePolicyError};
