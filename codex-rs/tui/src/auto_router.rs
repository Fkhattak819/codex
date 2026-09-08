//! Deterministic per-turn model routing for the interactive TUI.
//!
//! The policy is intentionally local and cheap: it classifies the finalized
//! user prompt, resolves one of the Sol/Terra/Astra families from the current
//! model catalog, and selects a supported reasoning effort. The existing turn
//! submission path remains responsible for every other setting.

use codex_protocol::openai_models::ModelPreset;
use codex_protocol::openai_models::ReasoningEffort;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AutoRoute {
    Sol,
    Terra,
    Astra,
}

impl AutoRoute {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Sol => "Sol",
            Self::Terra => "Terra",
            Self::Astra => "Astra",
        }
    }

    fn fallback_order(self) -> [Self; 3] {
        match self {
            Self::Sol => [Self::Sol, Self::Terra, Self::Astra],
            Self::Terra => [Self::Terra, Self::Sol, Self::Astra],
            Self::Astra => [Self::Astra, Self::Terra, Self::Sol],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RouteSignals {
    pub(crate) complexity: u8,
    pub(crate) scope: u8,
    pub(crate) ambiguity: u8,
    pub(crate) risk: u8,
}

impl RouteSignals {
    fn total(self) -> u8 {
        self.complexity + self.scope + self.ambiguity + self.risk
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RouteRecommendation {
    pub(crate) route: AutoRoute,
    pub(crate) effort: ReasoningEffort,
    pub(crate) signals: RouteSignals,
    pub(crate) reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RouteDecision {
    pub(crate) route: AutoRoute,
    pub(crate) model: String,
    pub(crate) effort: ReasoningEffort,
    pub(crate) reason: &'static str,
    pub(crate) fallback: Option<String>,
}

impl RouteDecision {
    pub(crate) fn concise_label(&self) -> String {
        format!(
            "Auto-routed: {} · {}",
            self.route.label(),
            self.effort.as_str()
        )
    }
}

pub(crate) fn recommend(prompt: &str) -> RouteRecommendation {
    let prompt = normalize(prompt);
    let word_count = prompt.split_whitespace().count();

    let trivial = contains_any(
        &prompt,
        &[
            "fix typo",
            "fix the typo",
            "rename ",
            "change the text",
            "change ui text",
            "adjust padding",
            "button padding",
            "update imports",
            "formatting cleanup",
            "locate the file",
            "find the function",
        ],
    );
    let implementation = contains_any(
        &prompt,
        &[
            "implement ",
            "add a feature",
            "add an endpoint",
            "api endpoint",
            "pagination",
            "integrate ",
            "refactor ",
            "database change",
            "repository abstraction",
            "existing design",
        ],
    );
    let broad_scope = contains_any(
        &prompt,
        &[
            "multi-file",
            "multiple files",
            "cross-cutting",
            "whole subsystem",
            "entire subsystem",
            "across the codebase",
            "end to end",
        ],
    );
    let ambiguous = contains_any(
        &prompt,
        &[
            "intermittent",
            "investigate",
            "root cause",
            "multiple plausible",
            "not sure why",
            "flaky",
            "sometimes fails",
        ],
    );
    let architecture = contains_any(
        &prompt,
        &[
            "architecture decision",
            "design the architecture",
            "redesign the architecture",
            "audit the architecture",
            "architectural redesign",
        ],
    ) || (prompt.contains("architecture")
        && contains_any(&prompt, &["audit ", "design ", "redesign "]));
    let concurrency = contains_any(
        &prompt,
        &[
            "race condition",
            "deadlock",
            "concurrency bug",
            "concurrent access",
            "thread safety",
        ],
    );
    let security = contains_any(
        &prompt,
        &[
            "security-sensitive",
            "security audit",
            "authentication architecture",
            "authorization architecture",
            "vulnerability",
            "exploit",
            "unsafe authentication",
            "anything unsafe",
        ],
    );
    let production = contains_any(
        &prompt,
        &["in production", "production failure", "production incident"],
    );
    let destructive = contains_any(
        &prompt,
        &[
            "data loss",
            "destructive migration",
            "irreversible migration",
            "drop the table",
            "corrupting data",
        ],
    );
    let repeated_failure = contains_any(
        &prompt,
        &[
            "still failing",
            "failed again",
            "previous attempt failed",
            "multiple failed attempts",
            "tried several times",
        ],
    );

    let mut signals = RouteSignals::default();
    if implementation {
        signals.complexity = 2;
        signals.scope = 1;
    }
    if broad_scope {
        signals.scope = 3;
        signals.complexity = signals.complexity.max(2);
    }
    if word_count >= 60 {
        signals.scope = signals.scope.max(2);
    }
    if word_count >= 140 {
        signals.complexity = signals.complexity.max(3);
        signals.scope = signals.scope.max(3);
    }
    if ambiguous {
        signals.ambiguity = 3;
    }
    if architecture {
        signals.complexity = 4;
        signals.scope = signals.scope.max(3);
        signals.ambiguity = signals.ambiguity.max(2);
    }
    if concurrency {
        signals.complexity = 4;
        signals.ambiguity = signals.ambiguity.max(2);
        signals.risk = signals.risk.max(3);
    }
    if security {
        signals.complexity = signals.complexity.max(3);
        signals.risk = 4;
    }
    if production {
        signals.risk = signals.risk.max(3);
    }
    if destructive {
        signals.risk = 4;
        signals.scope = signals.scope.max(2);
    }
    if repeated_failure {
        signals.ambiguity = 4;
        signals.complexity = signals.complexity.max(3);
    }

    let hard_astra_floor = architecture
        || concurrency
        || destructive
        || repeated_failure
        || (production && ambiguous)
        || (security && (architecture || ambiguous || broad_scope));
    if hard_astra_floor || signals.total() >= 11 {
        let effort = if signals.risk == 4 && signals.ambiguity == 4 {
            ReasoningEffort::High
        } else if signals.ambiguity >= 2 || signals.risk >= 3 {
            ReasoningEffort::Medium
        } else {
            ReasoningEffort::Low
        };
        return RouteRecommendation {
            route: AutoRoute::Astra,
            effort,
            signals,
            reason: "high-impact or ambiguous engineering work",
        };
    }

    if security || signals.total() >= 5 || (signals.complexity >= 2 && signals.scope >= 1) {
        let effort = if signals.ambiguity >= 2 || signals.scope >= 3 {
            ReasoningEffort::Medium
        } else {
            ReasoningEffort::Low
        };
        return RouteRecommendation {
            route: AutoRoute::Terra,
            effort,
            signals,
            reason: "normal implementation or scoped debugging",
        };
    }

    RouteRecommendation {
        route: AutoRoute::Sol,
        effort: if trivial || word_count <= 8 {
            ReasoningEffort::Low
        } else {
            ReasoningEffort::Medium
        },
        signals,
        reason: "small deterministic change",
    }
}

pub(crate) fn resolve(
    recommendation: RouteRecommendation,
    presets: &[ModelPreset],
) -> Option<RouteDecision> {
    let selected = recommendation
        .route
        .fallback_order()
        .into_iter()
        .find_map(|route| find_family_preset(presets, route).map(|preset| (route, preset)))?;
    let (resolved_route, preset) = selected;
    let supported: Vec<ReasoningEffort> = preset
        .supported_reasoning_efforts
        .iter()
        .map(|option| option.effort.clone())
        .collect();
    let effort = nearest_supported_effort(
        &recommendation.effort,
        &supported,
        &preset.default_reasoning_effort,
    );

    let mut fallback_parts = Vec::new();
    if resolved_route != recommendation.route {
        fallback_parts.push(format!(
            "{} unavailable; using {}",
            recommendation.route.label(),
            resolved_route.label()
        ));
    }
    if effort != recommendation.effort {
        fallback_parts.push(format!(
            "{} reasoning unsupported; using {}",
            recommendation.effort.as_str(),
            effort.as_str()
        ));
    }

    Some(RouteDecision {
        route: resolved_route,
        model: preset.model.clone(),
        effort,
        reason: recommendation.reason,
        fallback: (!fallback_parts.is_empty()).then(|| fallback_parts.join("; ")),
    })
}

fn find_family_preset(presets: &[ModelPreset], route: AutoRoute) -> Option<&ModelPreset> {
    presets.iter().find(|preset| {
        let slug = preset.model.to_ascii_lowercase();
        let display_name = preset.display_name.to_ascii_lowercase();
        let family = route.label().to_ascii_lowercase();
        slug == family
            || slug.ends_with(&format!("-{family}"))
            || display_name == family
            || display_name.ends_with(&format!("-{family}"))
            || display_name.ends_with(&format!(" {family}"))
    })
}

fn nearest_supported_effort(
    desired: &ReasoningEffort,
    supported: &[ReasoningEffort],
    default: &ReasoningEffort,
) -> ReasoningEffort {
    if supported.is_empty() {
        return default.clone();
    }
    if supported.contains(desired) {
        return desired.clone();
    }
    let desired_rank = effort_rank(desired);
    supported
        .iter()
        .min_by_key(|candidate| {
            let candidate_rank = effort_rank(candidate);
            (
                desired_rank.abs_diff(candidate_rank),
                u8::from(candidate_rank < desired_rank),
            )
        })
        .cloned()
        .unwrap_or_else(|| default.clone())
}

fn effort_rank(effort: &ReasoningEffort) -> u8 {
    match effort {
        ReasoningEffort::None => 0,
        ReasoningEffort::Minimal => 1,
        ReasoningEffort::Low => 2,
        ReasoningEffort::Medium => 3,
        ReasoningEffort::High => 4,
        ReasoningEffort::XHigh => 5,
        ReasoningEffort::Max => 6,
        ReasoningEffort::Ultra | ReasoningEffort::Persistent | ReasoningEffort::Custom(_) => 7,
    }
}

fn normalize(prompt: &str) -> String {
    prompt
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

#[cfg(test)]
#[path = "auto_router_tests.rs"]
mod tests;
