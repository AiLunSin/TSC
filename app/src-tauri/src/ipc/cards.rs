//! 卡片命令：只读快照投影与本地详情展开；`ClickState` 必须手工拆载。

use meeting_copilot_application::settings::{CardSnapshotEntry, ViewFactsSnapshot};
use meeting_copilot_domain::model::enums::TriggerKind;
use meeting_copilot_domain::model::gate_keyword::{
    CitationAlignment, ClickDisabledReason, ClickState, KeywordCardStatus, KeywordKind,
};
use meeting_copilot_ui_bridge::keyword_card::{
    open_keyword_card_detail, project_composed_card_view, project_keyword_card_view,
    ConclusionView, KeywordCardView, ViewFacts, ViewRejectReason,
};

use crate::ipc::dto::{
    CardBodyDto, CardDetailDto, ConclusionDto, IpcErrorDto, KeywordCardViewDto, OpenInputDto,
};
use crate::ipc::state::AppState;

/// 卡片快照：只读投影，撤回/契约违规直接过滤。
pub fn keyword_cards_snapshot_impl(
    state: &AppState,
) -> Result<Vec<KeywordCardViewDto>, IpcErrorDto> {
    let facts = to_view_facts(state.view_facts.view_facts());
    let mut cards = Vec::new();
    for entry in state.cards.cards() {
        match &entry {
            CardSnapshotEntry::Draft(draft) => {
                if let Some(view) = project_keyword_card_view(draft, &facts).view {
                    cards.push(to_card_dto(&view, false));
                }
            }
            CardSnapshotEntry::Preview(card) => {
                if card.card_status == KeywordCardStatus::Withdrawn {
                    continue;
                }
                if let Some(view) = project_composed_card_view(card, &facts).view {
                    cards.push(to_card_dto(&view, true));
                }
            }
        }
    }
    Ok(cards)
}

/// 卡片详情：仅在当前快照内定位草案卡，0 外发/0 存储/0 状态变更。
pub fn keyword_card_open_impl(
    state: &AppState,
    input: OpenInputDto,
) -> Result<CardDetailDto, IpcErrorDto> {
    if input.card_id.trim().is_empty() {
        return Err(IpcErrorDto::new(
            "input_contract_violation",
            "卡片标识无效。",
            false,
        ));
    }
    let facts = to_view_facts(state.view_facts.view_facts());
    for entry in state.cards.cards() {
        let CardSnapshotEntry::Draft(draft) = entry else {
            continue;
        };
        if draft.card_id.as_str() != input.card_id {
            continue;
        }
        let outcome = project_keyword_card_view(&draft, &facts);
        let Some(view) = outcome.view else {
            return Err(match outcome.reason {
                Some(ViewRejectReason::CardWithdrawn) => {
                    IpcErrorDto::new("card_withdrawn", "卡片已撤回。", false)
                }
                Some(ViewRejectReason::ReferenceContractViolation) => {
                    IpcErrorDto::new("reference_contract_violation", "引用不符合契约。", false)
                }
                _ => IpcErrorDto::new("input_contract_violation", "卡片内容不符合契约。", false),
            });
        };
        let click = open_keyword_card_detail(&view);
        return match click.detail {
            Some(detail) => Ok(to_detail_dto(&detail)),
            None => Err(match click.reason {
                Some(reason) => IpcErrorDto::new(
                    disabled_reason_str(reason),
                    disabled_reason_message(reason),
                    false,
                ),
                None => IpcErrorDto::new("card_not_found", "卡片不存在。", false),
            }),
        };
    }
    Err(IpcErrorDto::new("card_not_found", "卡片不存在。", false))
}

fn to_card_dto(view: &KeywordCardView, preview: bool) -> KeywordCardViewDto {
    let (click_state, disabled_reason) = if preview {
        ("disabled".to_owned(), Some("not_allowed".to_owned()))
    } else {
        match &view.click_state {
            ClickState::Enabled => ("enabled".to_owned(), None),
            ClickState::Disabled(reason) => (
                "disabled".to_owned(),
                Some(disabled_reason_str(*reason).to_owned()),
            ),
        }
    };
    KeywordCardViewDto {
        card_id: view.card_id.as_str().to_owned(),
        keyword_kind: keyword_kind_str(view.keyword_kind).to_owned(),
        trigger: trigger_str(view.trigger).to_owned(),
        triggered_at_ms: view.triggered_at_ms,
        card_status: card_status_str(view.card_status).to_owned(),
        body_visible: view.body_visible,
        body: view.body.as_ref().map(|body| CardBodyDto {
            title: body.title.as_str().to_owned(),
            conclusion: conclusion_dto(&body.conclusion),
            confidence: body.confidence.value(),
            reference_ids: body
                .references
                .iter()
                .map(|reference| reference.reference_id.as_str().to_owned())
                .collect(),
        }),
        click_state,
        disabled_reason,
        badge_count: view.badge_count,
    }
}

fn to_detail_dto(
    detail: &meeting_copilot_ui_bridge::keyword_card::CardDetailView,
) -> CardDetailDto {
    CardDetailDto {
        card_id: detail.card_id.as_str().to_owned(),
        title: detail.title.as_str().to_owned(),
        conclusion: conclusion_dto(&detail.conclusion),
        reference_ids: detail
            .reference_ids
            .iter()
            .map(|reference| reference.as_str().to_owned())
            .collect(),
        citation_alignment: citation_alignment_str(detail.citation_alignment).to_owned(),
    }
}

fn to_view_facts(snapshot: ViewFactsSnapshot) -> ViewFacts {
    ViewFacts {
        session_matches: snapshot.session_matches,
        authorization_revoked: snapshot.authorization_revoked,
        reference_stale: snapshot.reference_stale,
        screen_share: snapshot.screen_share,
        emergency_masked: snapshot.emergency_masked,
        visible_card_count: snapshot.visible_card_count,
    }
}

fn conclusion_dto(view: &ConclusionView) -> ConclusionDto {
    match view {
        ConclusionView::Available(text) => ConclusionDto::Available {
            text: text.as_str().to_owned(),
        },
        ConclusionView::NoReliableSource => ConclusionDto::NoReliableSource,
        ConclusionView::Absent => ConclusionDto::Absent,
    }
}

fn keyword_kind_str(kind: KeywordKind) -> &'static str {
    match kind {
        KeywordKind::ProperNoun => "proper_noun",
        KeywordKind::Acronym => "acronym",
        KeywordKind::InstitutionRole => "institution_role",
        KeywordKind::PolicyStandard => "policy_standard",
        KeywordKind::Metric => "metric",
    }
}

fn trigger_str(trigger: TriggerKind) -> &'static str {
    match trigger {
        TriggerKind::Auto => "auto",
        TriggerKind::OnDemand => "on_demand",
    }
}

fn card_status_str(status: KeywordCardStatus) -> &'static str {
    match status {
        KeywordCardStatus::Displayed => "displayed",
        KeywordCardStatus::Collapsed => "collapsed",
        KeywordCardStatus::Withdrawn => "withdrawn",
    }
}

fn citation_alignment_str(alignment: CitationAlignment) -> &'static str {
    match alignment {
        CitationAlignment::Passed => "passed",
        CitationAlignment::Failed => "failed",
        CitationAlignment::Unknown => "unknown",
    }
}

fn disabled_reason_str(reason: ClickDisabledReason) -> &'static str {
    match reason {
        ClickDisabledReason::StaleReference => "stale_reference",
        ClickDisabledReason::AuthorizationRevoked => "authorization_revoked",
        ClickDisabledReason::NotAllowed => "not_allowed",
        ClickDisabledReason::EmergencyMasked => "emergency_masked",
        ClickDisabledReason::ScreenShareProtection => "screen_share_protection",
        ClickDisabledReason::ReferenceUnverified => "reference_unverified",
    }
}

fn disabled_reason_message(reason: ClickDisabledReason) -> &'static str {
    match reason {
        ClickDisabledReason::StaleReference => "引用已过期，请稍后重试。",
        ClickDisabledReason::AuthorizationRevoked => "授权已撤回。",
        ClickDisabledReason::NotAllowed => "当前卡片不可点击。",
        ClickDisabledReason::EmergencyMasked => "紧急遮蔽中。",
        ClickDisabledReason::ScreenShareProtection => "投屏保护中。",
        ClickDisabledReason::ReferenceUnverified => "引用未经核实。",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use meeting_copilot_application::cards::ComposedCard;
    use meeting_copilot_application::settings::{
        CardSnapshotSource, FactsSnapshot, PortError, SecureStore, SettingsConfig,
        SettingsConfigStore, SettingsFactsSource, ViewFactsSource,
    };
    use meeting_copilot_domain::vo::{CardRef, Confidence, NormalizedTerm, TitleText};

    use super::*;
    use crate::ipc::catalog::SettingsCatalog;
    use crate::ipc::state::CommandCounters;

    struct MemorySecure;

    impl SecureStore for MemorySecure {
        fn write(
            &self,
            _secret: &meeting_copilot_application::settings::SecretBytes,
        ) -> Result<(), PortError> {
            Ok(())
        }

        fn read(
            &self,
        ) -> Result<Option<meeting_copilot_application::settings::SecretBytes>, PortError> {
            Ok(None)
        }

        fn delete(&self) -> Result<(), PortError> {
            Ok(())
        }
    }

    struct MemoryConfig;

    impl SettingsConfigStore for MemoryConfig {
        fn load(&self) -> Result<SettingsConfig, PortError> {
            Ok(SettingsConfig::default())
        }

        fn save(&self, _config: &SettingsConfig) -> Result<(), PortError> {
            Ok(())
        }

        fn reset(&self) -> Result<(), PortError> {
            Ok(())
        }
    }

    struct FixedFacts(FactsSnapshot);

    impl SettingsFactsSource for FixedFacts {
        fn facts(&self) -> FactsSnapshot {
            self.0
        }
    }

    struct NoProbe;

    impl meeting_copilot_application::settings::ProbeExecutor for NoProbe {
        fn execute(
            &self,
            _call: &meeting_copilot_application::settings::ProbeCall,
        ) -> Result<meeting_copilot_application::settings::ProbeAttempt, PortError> {
            Err(PortError::Unsupported)
        }
    }

    struct FixedView(ViewFactsSnapshot);

    impl ViewFactsSource for FixedView {
        fn view_facts(&self) -> ViewFactsSnapshot {
            self.0
        }
    }

    struct CountingCards {
        cards: Vec<CardSnapshotEntry>,
        calls: AtomicUsize,
    }

    impl CardSnapshotSource for CountingCards {
        fn cards(&self) -> Vec<CardSnapshotEntry> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.cards.clone()
        }
    }

    fn normal_view() -> ViewFactsSnapshot {
        ViewFactsSnapshot {
            session_matches: true,
            authorization_revoked: false,
            reference_stale: false,
            screen_share: false,
            emergency_masked: false,
            visible_card_count: 2,
        }
    }

    fn preview(status: KeywordCardStatus) -> ComposedCard {
        ComposedCard {
            card_id: CardRef::new("card-1").expect("id"),
            keyword_text: TitleText::new("测试关键词").expect("title"),
            normalized_term: NormalizedTerm::new("测试关键词").expect("norm"),
            keyword_kind: KeywordKind::Metric,
            trigger: TriggerKind::Auto,
            triggered_at_ms: 1_700_000_000_000,
            confidence: Confidence::new(0.8).expect("conf"),
            summary: None,
            references: Vec::new(),
            citation_alignment: CitationAlignment::Unknown,
            card_status: status,
        }
    }

    fn state(cards: Arc<dyn CardSnapshotSource>, view: ViewFactsSnapshot) -> AppState {
        AppState {
            secure: Arc::new(MemorySecure),
            config: Arc::new(MemoryConfig),
            probe: Arc::new(NoProbe),
            facts: Arc::new(FixedFacts(FactsSnapshot {
                has_stored_key: false,
                model_count: 1,
                endpoint_count: 1,
                now_ms: 1_000,
                cooldown_until_ms: None,
                probe_in_flight: false,
            })),
            cards,
            view_facts: Arc::new(FixedView(view)),
            catalog: SettingsCatalog::from_injected(vec!["preset-a".to_owned()]),
            memory_key: Mutex::new(None),
            form_state: Mutex::new(
                meeting_copilot_ui_bridge::settings::SettingsFormState::NotConfigured,
            ),
            probe_in_flight: AtomicBool::new(false),
            cooldown_until_ms: Mutex::new(None),
            last_probe_report: Mutex::new(None),
            counters: Mutex::new(CommandCounters::default()),
        }
    }

    #[test]
    fn ipc_card_snapshot_is_closed_and_read_only() {
        let cards = Arc::new(CountingCards {
            cards: vec![CardSnapshotEntry::Preview(preview(
                KeywordCardStatus::Displayed,
            ))],
            calls: AtomicUsize::new(0),
        });
        let state = state(cards, normal_view());
        let snapshot = keyword_cards_snapshot_impl(&state).expect("snapshot");
        assert_eq!(snapshot.len(), 1);
        let json = serde_json::to_value(&snapshot[0]).expect("json");
        assert_eq!(json.as_object().expect("object").len(), 10);
        let text = json.to_string();
        for banned in [
            "normalizedTerm",
            "gateDecisionRef",
            "traceId",
            "url",
            "model",
            "requestId",
            "apiKey",
        ] {
            assert!(!text.contains(banned), "banned {banned}");
        }
        assert_eq!(snapshot[0].click_state, "disabled");
        assert_eq!(snapshot[0].disabled_reason.as_deref(), Some("not_allowed"));
        assert_eq!(snapshot[0].badge_count, 2);
    }

    #[test]
    fn ipc_card_detail_is_closed() {
        let detail = CardDetailDto {
            card_id: "card-1".to_owned(),
            title: "标题".to_owned(),
            conclusion: ConclusionDto::Absent,
            reference_ids: Vec::new(),
            citation_alignment: "unknown".to_owned(),
        };
        let json = serde_json::to_value(&detail).expect("json");
        assert_eq!(json.as_object().expect("object").len(), 5);

        let cards = Arc::new(CountingCards {
            cards: vec![CardSnapshotEntry::Preview(preview(
                KeywordCardStatus::Displayed,
            ))],
            calls: AtomicUsize::new(0),
        });
        let state = state(cards, normal_view());
        let error = keyword_card_open_impl(
            &state,
            OpenInputDto {
                card_id: "card-1".to_owned(),
            },
        )
        .expect_err("preview not openable");
        assert_eq!(error.code, "card_not_found");
    }

    #[test]
    fn ipc_card_snapshot_hides_body_under_screen_share() {
        let cards = Arc::new(CountingCards {
            cards: vec![CardSnapshotEntry::Preview(preview(
                KeywordCardStatus::Displayed,
            ))],
            calls: AtomicUsize::new(0),
        });
        let mut view = normal_view();
        view.screen_share = true;
        let state = state(cards, view);
        let snapshot = keyword_cards_snapshot_impl(&state).expect("snapshot");
        assert_eq!(snapshot.len(), 1);
        assert!(!snapshot[0].body_visible);
        assert!(snapshot[0].body.is_none());
        assert_eq!(snapshot[0].disabled_reason.as_deref(), Some("not_allowed"));
    }

    #[test]
    fn card_snapshot_does_not_revive_withdrawn() {
        let cards = Arc::new(CountingCards {
            cards: vec![CardSnapshotEntry::Preview(preview(
                KeywordCardStatus::Withdrawn,
            ))],
            calls: AtomicUsize::new(0),
        });
        let state = state(cards, normal_view());
        assert!(keyword_cards_snapshot_impl(&state)
            .expect("snapshot")
            .is_empty());
    }

    #[test]
    fn ipc_card_open_has_zero_side_effects() {
        let cards = Arc::new(CountingCards {
            cards: vec![CardSnapshotEntry::Preview(preview(
                KeywordCardStatus::Displayed,
            ))],
            calls: AtomicUsize::new(0),
        });
        let counter = Arc::clone(&cards);
        let state = state(cards, normal_view());
        let _ = keyword_card_open_impl(
            &state,
            OpenInputDto {
                card_id: "missing".to_owned(),
            },
        );
        assert_eq!(counter.calls.load(Ordering::SeqCst), 1);
        assert!(state.last_probe_report.lock().expect("lock").is_none());
        assert!(state.cooldown_until_ms.lock().expect("lock").is_none());
        assert!(
            meeting_copilot_ui_bridge::settings::SettingsFormState::NotConfigured
                == *state.form_state.lock().expect("lock")
        );
    }
}
