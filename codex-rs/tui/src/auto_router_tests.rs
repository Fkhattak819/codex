use super::*;
use codex_protocol::openai_models::InputModality;
use codex_protocol::openai_models::ReasoningEffortPreset;
use pretty_assertions::assert_eq;

#[test]
fn routes_small_mechanical_prompts_to_sol() {
    for prompt in [
        "rename Foo to Bar",
        "change the button padding to 12px",
        "fix typo",
    ] {
        let recommendation = recommend(prompt);
        assert_eq!(recommendation.route, AutoRoute::Sol, "prompt: {prompt}");
        assert_eq!(recommendation.effort, ReasoningEffort::Low);
    }
}

#[test]
fn routes_code_implementation_to_astra_low() {
    for prompt in [
        "implement pagination for the users API",
        "refactor this service to use the repository abstraction",
        "write code for the account settings screen",
        "fix this bug in the checkout flow",
    ] {
        let recommendation = recommend(prompt);
        assert_eq!(recommendation.route, AutoRoute::Astra, "prompt: {prompt}");
        assert_eq!(
            recommendation.effort,
            ReasoningEffort::Low,
            "prompt: {prompt}"
        );
    }
}

#[test]
fn routes_broad_code_implementation_to_astra_medium() {
    for prompt in [
        "implement this feature end to end across the codebase and update multiple files",
        "handle this tough implementation",
        "implement code for this difficult feature",
    ] {
        let recommendation = recommend(prompt);
        assert_eq!(recommendation.route, AutoRoute::Astra, "prompt: {prompt}");
        assert_eq!(
            recommendation.effort,
            ReasoningEffort::Medium,
            "prompt: {prompt}"
        );
    }
}

#[test]
fn routes_high_risk_ambiguous_work_to_astra_medium() {
    for prompt in [
        "there is an intermittent deadlock in production; find and fix it",
        "audit the authentication architecture and redesign anything unsafe",
    ] {
        let recommendation = recommend(prompt);
        assert_eq!(recommendation.route, AutoRoute::Astra, "prompt: {prompt}");
        assert_eq!(recommendation.effort, ReasoningEffort::Medium);
    }
}

#[test]
fn route_can_transition_without_retaining_classifier_state() {
    assert_eq!(recommend("fix typo").route, AutoRoute::Sol);
    assert_eq!(
        recommend("investigate this intermittent production race condition and fix it").route,
        AutoRoute::Astra
    );
    assert_eq!(recommend("rename Foo to Bar").route, AutoRoute::Sol);
}

#[test]
fn resolves_current_model_slugs_by_family_name() {
    let presets = all_router_presets();
    let decision = resolve(recommend("fix typo"), &presets).expect("Sol should resolve");
    assert_eq!(decision.model, "gpt-5.6-sol");

    let decision = resolve(
        recommend("implement pagination for the users API"),
        &presets,
    )
    .expect("Astra should resolve");
    assert_eq!(decision.model, "gpt-6-astra");

    let decision = resolve(
        recommend("there is an intermittent deadlock in production; find and fix it"),
        &presets,
    )
    .expect("Astra should resolve");
    assert_eq!(decision.model, "gpt-6-astra");
}

#[test]
fn falls_back_to_next_router_family_when_model_is_missing() {
    let presets = vec![preset(
        "gpt-5.6-terra",
        "GPT-5.6-Terra",
        &[ReasoningEffort::Low, ReasoningEffort::Medium],
    )];
    let decision = resolve(
        recommend("there is an intermittent deadlock in production; find and fix it"),
        &presets,
    )
    .expect("Terra fallback should resolve");
    assert_eq!(decision.route, AutoRoute::Terra);
    assert_eq!(decision.model, "gpt-5.6-terra");
    assert_eq!(
        decision.fallback,
        Some("Astra unavailable; using Terra".to_string())
    );
}

#[test]
fn unsupported_reasoning_uses_nearest_stronger_effort() {
    let presets = vec![preset(
        "gpt-6-astra",
        "GPT-6-Astra",
        &[ReasoningEffort::High, ReasoningEffort::XHigh],
    )];
    let decision = resolve(
        recommend("there is an intermittent deadlock in production; find and fix it"),
        &presets,
    )
    .expect("Astra should resolve");
    assert_eq!(decision.effort, ReasoningEffort::High);
    assert_eq!(
        decision.fallback,
        Some("medium reasoning unsupported; using high".to_string())
    );
}

#[test]
fn returns_none_when_no_router_family_is_available() {
    let presets = vec![preset(
        "gpt-5.6-luna",
        "GPT-5.6-Luna",
        &[ReasoningEffort::Medium],
    )];
    assert_eq!(resolve(recommend("fix typo"), &presets), None);
}

fn all_router_presets() -> Vec<ModelPreset> {
    vec![
        preset(
            "gpt-5.6-sol",
            "GPT-5.6-Sol",
            &[ReasoningEffort::Low, ReasoningEffort::Medium],
        ),
        preset(
            "gpt-5.6-terra",
            "GPT-5.6-Terra",
            &[ReasoningEffort::Low, ReasoningEffort::Medium],
        ),
        preset(
            "gpt-6-astra",
            "GPT-6-Astra",
            &[ReasoningEffort::Low, ReasoningEffort::Medium],
        ),
    ]
}

fn preset(model: &str, display_name: &str, efforts: &[ReasoningEffort]) -> ModelPreset {
    ModelPreset {
        id: model.to_string(),
        model: model.to_string(),
        display_name: display_name.to_string(),
        description: String::new(),
        model_specialty: None,
        default_reasoning_effort: ReasoningEffort::Medium,
        supported_reasoning_efforts: efforts
            .iter()
            .cloned()
            .map(|effort| ReasoningEffortPreset {
                effort,
                description: String::new(),
            })
            .collect(),
        supports_personality: false,
        additional_speed_tiers: Vec::new(),
        service_tiers: Vec::new(),
        default_service_tier: None,
        is_default: false,
        upgrade: None,
        show_in_picker: true,
        multi_agent_version: None,
        availability_nux: None,
        supported_in_api: true,
        input_modalities: vec![InputModality::Text],
    }
}
