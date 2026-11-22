//! 38 卡片合成门面：只读输入、确定性输出、不判定点击。

pub mod compose;

pub use compose::{
    compose_keyword_card, next_card_status, CardComposeDiagnostics, CardComposeInput,
    CardComposeOutcome, CardStatusSignal, ComposeRejectReason, ComposedCard,
};
