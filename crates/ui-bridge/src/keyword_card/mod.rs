//! 51 点击 UI：只读视图投影 + 本地点击展开（0 外发/0 LLM/0 审计）。

pub mod view;

pub use view::{
    open_keyword_card_detail, project_composed_card_view, project_keyword_card_view, CardBodyView,
    CardDetailView, CardViewOutcome, ClickOutcome, ConclusionView, KeywordCardView, ViewFacts,
    ViewRejectReason,
};
