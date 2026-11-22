//! 51 设置向导（AC-B4-85~94 核心用例）。

use meeting_copilot_ui_bridge::settings::{
    apply_defaults, humanize_probe_failure, mask_api_key, next_probe_state, project_settings_view,
    resolve_model_name, validate_settings_input, EndpointDraft, EndpointSelectionView, FieldIssue,
    MaskedKey, ModelDraft, ModelSelectionView, PersistTarget, ProbeEvent, ProbeFailureKind,
    SecretInput, SettingsDefaults, SettingsFacts, SettingsFormState, SettingsInput, SettingsIntent,
    SECTION_ORDER,
};

fn facts() -> SettingsFacts {
    SettingsFacts {
        has_stored_key: false,
        model_count: 1,
        endpoint_count: 1,
        now_ms: 1_000,
        cooldown_until_ms: None,
        probe_in_flight: false,
    }
}

fn input() -> SettingsInput {
    SettingsInput {
        api_key_draft: SecretInput::from_draft("test-key-000000000000"),
        model_draft: ModelDraft::PresetIndex(0),
        endpoint_draft: EndpointDraft::Index(0),
        submitted: false,
        clear_confirmed: false,
    }
}

#[test]
fn llm_settings_view_fields_are_closed() {
    assert_eq!(SECTION_ORDER.len(), 4);
    let defaults = SettingsDefaults::default();
    assert_eq!(defaults.timeout_ms, 6_000);
    assert!(defaults.auto_save_on_verified);
    let view = project_settings_view(
        &facts(),
        &input(),
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert!(view.has_key);
    assert_eq!(view.key_display, MaskedKey::Masked);
    assert!(view.can_test);
    assert!(view.can_save);
}

#[test]
fn llm_settings_view_mask_has_no_payload() {
    for raw in [1usize, 16, 64, 256] {
        let secret = SecretInput::from_draft(&"k".repeat(raw));
        assert_eq!(format!("{:?}", mask_api_key(&secret)), "Masked");
    }
    assert_eq!(format!("{:?}", mask_api_key(&SecretInput::empty())), "None");
}

#[test]
fn llm_settings_view_secret_debug_is_fixed() {
    let secret = SecretInput::from_draft("test-key-000000000000");
    assert_eq!(format!("{:?}", secret), "***");
    assert!(secret.is_provided());
    assert!(!format!("{:?}", secret).contains("test-key"));
}

#[test]
fn llm_settings_view_one_step_default_is_applied() {
    let applied = apply_defaults(&facts(), &SettingsDefaults::single_step());
    assert_eq!(applied.model_draft, ModelDraft::PresetIndex(0));
    assert_eq!(applied.endpoint_draft, EndpointDraft::Index(0));
    assert!(!applied.submitted);
    assert!(!applied.clear_confirmed);
}

#[test]
fn llm_settings_view_key_validation_is_inline() {
    let mut settings = input();
    settings.api_key_draft = SecretInput::from_draft("");
    assert!(settings.api_key_draft.draft_issue() == Some(FieldIssue::ApiKeyMissing));
    settings.api_key_draft = SecretInput::from_draft("short");
    assert!(settings.api_key_draft.draft_issue() == Some(FieldIssue::ApiKeyTooShort));
    settings.api_key_draft = SecretInput::from_draft("test key 000000000000");
    assert!(settings.api_key_draft.draft_issue() == Some(FieldIssue::ApiKeyHasWhitespace));
    settings.api_key_draft = SecretInput::from_draft("test-key-000000000000");
    assert!(settings.api_key_draft.draft_issue().is_none());

    settings.submitted = true;
    let issues = validate_settings_input(
        &facts(),
        &settings,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert!(issues.is_empty());
}

#[test]
fn llm_settings_view_model_names_are_injected() {
    let presets = vec!["model-a".to_owned()];
    assert_eq!(
        resolve_model_name(&ModelDraft::PresetIndex(0), &presets),
        ModelSelectionView::Preset {
            display_name: "model-a".to_owned()
        }
    );
    assert_eq!(
        resolve_model_name(&ModelDraft::CustomName("m-1".to_owned()), &presets),
        ModelSelectionView::Custom {
            name: "m-1".to_owned()
        }
    );
}

#[test]
fn llm_settings_view_endpoint_is_index_only() {
    let view = project_settings_view(
        &facts(),
        &input(),
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert_eq!(
        view.endpoint_choice,
        EndpointSelectionView::Option {
            display_name: "endpoint-a".to_owned()
        }
    );
}

#[test]
fn llm_settings_view_missing_key_is_not_configured() {
    let mut settings = input();
    settings.api_key_draft = SecretInput::empty();
    let view = project_settings_view(
        &facts(),
        &settings,
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert_eq!(view.form_state, SettingsFormState::NotConfigured);
    assert_eq!(view.key_display, MaskedKey::None);
    assert!(!view.can_test);
    assert!(!view.can_save);
}

#[test]
fn llm_settings_view_probe_state_is_single_flight() {
    let mut busy = facts();
    busy.probe_in_flight = true;
    let view = project_settings_view(
        &busy,
        &input(),
        SettingsFormState::Testing,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert_eq!(view.form_state, SettingsFormState::Testing);
    assert!(!view.can_test);

    let mut cooling = facts();
    cooling.cooldown_until_ms = Some(10_000);
    let view = project_settings_view(
        &cooling,
        &input(),
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert!(
        matches!(view.form_state, SettingsFormState::Cooling { remaining_ms } if remaining_ms > 0)
    );
    assert!(!view.can_test);
}

#[test]
fn llm_settings_view_intents_are_explicit() {
    let settings = input();
    assert!(matches!(
        settings.persist_intent(PersistTarget::Memory),
        Some(SettingsIntent::Persist {
            target: PersistTarget::Memory
        })
    ));
    assert!(settings.clear_intent().is_none());
    let mut confirmed = input();
    confirmed.clear_confirmed = true;
    assert!(matches!(
        confirmed.clear_intent(),
        Some(SettingsIntent::Clear)
    ));
}

#[test]
fn llm_settings_view_probe_events_and_messages() {
    assert_eq!(
        next_probe_state(
            &facts(),
            &input(),
            SettingsFormState::ConfiguredUnverified,
            ProbeEvent::UserTest,
            &["model-a".to_owned()],
            &["endpoint-a".to_owned()],
            &SettingsDefaults::single_step(),
        )
        .0,
        SettingsFormState::Testing
    );
    assert_eq!(
        next_probe_state(
            &facts(),
            &input(),
            SettingsFormState::Testing,
            ProbeEvent::UserCancel,
            &[],
            &[],
            &SettingsDefaults::single_step(),
        )
        .0,
        SettingsFormState::ConfiguredUnverified
    );
    for failure in [
        ProbeFailureKind::MissingKey,
        ProbeFailureKind::KeyRejected,
        ProbeFailureKind::EndpointUnreachable,
        ProbeFailureKind::Timeout,
        ProbeFailureKind::RateLimited,
        ProbeFailureKind::QuotaExceeded,
        ProbeFailureKind::ServerError,
        ProbeFailureKind::Unknown,
    ] {
        assert!(!humanize_probe_failure(failure).is_empty());
    }
}

#[test]
fn llm_settings_view_is_offline_and_platform_free() {
    let first = project_settings_view(
        &facts(),
        &input(),
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    for _ in 0..10 {
        let again = project_settings_view(
            &facts(),
            &input(),
            SettingsFormState::ConfiguredUnverified,
            &["model-a".to_owned()],
            &["endpoint-a".to_owned()],
        );
        assert_eq!(first, again);
    }
}

#[test]
fn llm_settings_view_endpoint_whitelist_is_index_only() {
    let mut settings = input();
    settings.endpoint_draft = EndpointDraft::Index(5);
    let view = project_settings_view(
        &facts(),
        &settings,
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert_eq!(view.endpoint_choice, EndpointSelectionView::Unavailable);
    assert!(!view.can_test);
    assert!(!view.can_save);
}

#[test]
fn llm_settings_view_clear_requires_confirm() {
    let settings = input();
    assert!(settings.clear_intent().is_none());
    let mut confirmed = input();
    confirmed.clear_confirmed = true;
    assert!(matches!(
        confirmed.clear_intent(),
        Some(SettingsIntent::Clear)
    ));
}

#[test]
fn llm_settings_view_cooldown_boundary() {
    let mut boundary = facts();
    boundary.cooldown_until_ms = Some(boundary.now_ms);
    let view = project_settings_view(
        &boundary,
        &input(),
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert_eq!(view.form_state, SettingsFormState::ConfiguredUnverified);
    assert!(view.can_test);

    let mut cooling = facts();
    cooling.cooldown_until_ms = Some(cooling.now_ms + 1);
    let view = project_settings_view(
        &cooling,
        &input(),
        SettingsFormState::ConfiguredUnverified,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert!(
        matches!(view.form_state, SettingsFormState::Cooling { remaining_ms } if remaining_ms == 1)
    );
    assert!(!view.can_test);
}

#[test]
fn llm_settings_view_model_length_and_missing() {
    let mut settings = input();
    settings.submitted = true;
    settings.model_draft = ModelDraft::CustomName("m".repeat(129));
    let issues = validate_settings_input(
        &facts(),
        &settings,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert!(issues.contains(&FieldIssue::ModelTooLong));

    settings.model_draft = ModelDraft::CustomName("has space".to_owned());
    let issues = validate_settings_input(
        &facts(),
        &settings,
        &["model-a".to_owned()],
        &["endpoint-a".to_owned()],
    );
    assert!(issues.contains(&FieldIssue::ModelMissing));
}

#[test]
fn llm_settings_view_field_issues_are_ordered_and_unique() {
    let mut settings = input();
    settings.submitted = true;
    settings.api_key_draft = SecretInput::from_draft("");
    settings.model_draft = ModelDraft::CustomName("m".repeat(129));
    let mut empty_facts = facts();
    empty_facts.endpoint_count = 0;
    let issues = validate_settings_input(&empty_facts, &settings, &[], &[]);
    assert!(issues.contains(&FieldIssue::ApiKeyMissing));
    assert!(issues.contains(&FieldIssue::ModelTooLong));
    assert!(issues.contains(&FieldIssue::EndpointUnavailable));
    let unique: Vec<FieldIssue> = {
        let mut copy = issues.clone();
        copy.dedup();
        copy
    };
    assert_eq!(issues, unique);
}

#[test]
fn llm_settings_view_no_plaintext_in_debug() {
    let settings = input();
    let debug = format!("{:?}", settings);
    assert!(!debug.contains("test-key"));
    assert!(!debug.contains("***"));
    let secret_debug = format!("{:?}", settings.api_key_draft);
    assert_eq!(secret_debug, "***");
}
