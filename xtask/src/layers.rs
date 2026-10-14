//! Dependency layering rules (research/08 §1–2).
//!
//! The rule engine works on a small, metadata-independent model so it can be
//! unit-tested; `from_metadata` builds that model from `cargo metadata`.

use serde_json::Value;

/// Where a workspace crate sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// Regular layered crate.
    Layer(u8),
    /// Test tooling: may depend on anything up to L5 (it sits at L6 for rule
    /// purposes); other crates may use it only as a dev-dependency.
    Testkit,
    /// Binaries and build tooling: exempt from the rules.
    Exempt,
}

impl Class {
    fn layer(self) -> Option<u8> {
        match self {
            Class::Layer(l) => Some(l),
            Class::Testkit => Some(6),
            Class::Exempt => None,
        }
    }
}

/// The layering table (research/08 §1–2). Names are package names without the
/// `anicraft-` prefix. Planned crates are registered here before they exist so
/// the layer contract is fixed; `check` validates whatever members are present.
pub const TABLE: &[(&str, Class)] = &[
    // L0 foundations
    ("geom", Class::Layer(0)),
    ("color", Class::Layer(0)),
    ("time", Class::Layer(0)),
    // L1 document model
    ("doc", Class::Layer(1)),
    ("keyframe", Class::Layer(1)),
    // L2 authoring primitives
    ("shapes", Class::Layer(2)),
    ("text", Class::Layer(2)),
    // L3 evaluation, pixels, format I/O
    ("timeline", Class::Layer(3)),
    ("render", Class::Layer(3)),
    ("gpu", Class::Layer(3)),
    ("swf", Class::Layer(3)),
    ("xfl", Class::Layer(3)),
    ("svg", Class::Layer(3)),
    ("export", Class::Layer(3)),
    // L4 tools
    ("tools", Class::Layer(4)),
    // L5 engine and scripting
    ("engine", Class::Layer(5)),
    ("script", Class::Layer(5)),
    // L6 frontend and agent surfaces
    ("ui-egui", Class::Layer(6)),
    ("mcp", Class::Layer(6)),
    // Test tooling (dev-dependencies only)
    ("testkit", Class::Testkit),
    // L7 apps and build tooling
    ("anicraft", Class::Exempt),
    ("cli", Class::Exempt),
    ("web", Class::Exempt),
    ("xtask", Class::Exempt),
];

/// Explicit orderings *within* a layer (earlier may be used by later)
/// (research/08 §2, "Allowed same-layer edges").
pub const INTRA_LAYER_ORDER: &[&[&str]] = &[
    // The CPU renderer consumes the evaluator's display list; the evaluator may in turn use
    // the renderer's bounds/viewport types.
    &["timeline", "render"],
    &["render", "timeline"],
    // Export rasterizes frames through the renderer.
    &["render", "export"],
    // The GPU compositor (later) implements the CPU reference's steps and is parity-tested.
    &["render", "gpu"],
    // Scripts mutate the document through engine commands, never memory.
    &["engine", "script"],
];

fn intra_layer_allowed(from: &str, to: &str) -> bool {
    let (from, to) = (short_name(from), short_name(to));
    INTRA_LAYER_ORDER.iter().any(|chain| match (chain.iter().position(|n| *n == from), chain.iter().position(|n| *n == to)) {
        (Some(f), Some(t)) => t < f,
        _ => false,
    })
}

/// External crates that constitute a UI toolkit / windowing dependency.
/// Entries ending in `*` are prefixes.
pub const UI_CRATES: &[&str] = &["egui", "eframe", "winit", "egui_kittest", "craft-ui", "rfd", "cpal", "muda"];

/// First layer allowed to use UI crates.
pub const UI_MIN_LAYER: u8 = 6;

pub fn short_name(pkg: &str) -> &str {
    pkg.strip_prefix("anicraft-").unwrap_or(pkg)
}

pub fn classify(pkg: &str) -> Option<Class> {
    let s = short_name(pkg);
    TABLE.iter().find(|(n, _)| *n == s).map(|(_, c)| *c)
}

fn is_ui_crate(name: &str) -> bool {
    UI_CRATES.iter().any(|p| match p.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *p,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepKind {
    Normal,
    Dev,
    Build,
}

#[derive(Debug, Clone)]
pub struct Dep {
    pub name: String,
    pub kind: DepKind,
    /// `true` if the dependency is a workspace member.
    pub workspace: bool,
}

#[derive(Debug, Clone)]
pub struct Crate {
    pub name: String,
    pub deps: Vec<Dep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Violation {
    Unregistered { krate: String },
    Upward { krate: String, dep: String, from: u8, to: u8, kind: DepKind },
    TestkitAsNormalDep { krate: String },
    UiBelowL6 { krate: String, dep: String, layer: u8 },
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Violation::Unregistered { krate } => {
                write!(f, "{krate}: unknown workspace crate; register it in xtask/src/layers.rs TABLE (see research/08 §2)")
            }
            Violation::Upward { krate, dep, from, to, kind } => {
                write!(f, "{krate} (L{from}) -> {dep} (L{to}) [{kind:?}]: may only depend on strictly lower layers")
            }
            Violation::TestkitAsNormalDep { krate } => {
                write!(f, "{krate}: anicraft-testkit may only be a dev-dependency")
            }
            Violation::UiBelowL6 { krate, dep, layer } => {
                write!(f, "{krate} (L{layer}) depends on UI crate `{dep}`; UI toolkits are only allowed in L6+")
            }
        }
    }
}

/// Check all rules. Returns violations sorted for stable output.
pub fn check(crates: &[Crate]) -> Vec<Violation> {
    let mut out = Vec::new();
    for c in crates {
        let Some(class) = classify(&c.name) else {
            out.push(Violation::Unregistered { krate: c.name.clone() });
            continue;
        };
        if class == Class::Exempt {
            continue;
        }
        let layer = class.layer().unwrap_or(0);
        for d in &c.deps {
            // Self dev-dependencies (e.g. to enable features in tests) are fine.
            if d.name == c.name {
                continue;
            }
            if d.workspace {
                match classify(&d.name) {
                    // Unregistered deps are reported on their own entry.
                    None => {}
                    Some(Class::Testkit) if d.kind != DepKind::Dev => {
                        out.push(Violation::TestkitAsNormalDep { krate: c.name.clone() });
                    }
                    // The testkit as a dev-dependency is how tests use it.
                    Some(Class::Testkit) => {}
                    Some(dc) => {
                        let to = dc.layer().unwrap_or(u8::MAX);
                        if to >= layer && !(to == layer && intra_layer_allowed(&c.name, &d.name)) {
                            out.push(Violation::Upward {
                                krate: c.name.clone(),
                                dep: d.name.clone(),
                                from: layer,
                                to: if to == u8::MAX { 7 } else { to },
                                kind: d.kind,
                            });
                        }
                    }
                }
            } else if layer < UI_MIN_LAYER && is_ui_crate(&d.name) {
                out.push(Violation::UiBelowL6 { krate: c.name.clone(), dep: d.name.clone(), layer });
            }
        }
    }
    out.sort_by_key(|v| v.to_string());
    out.dedup();
    out
}

/// Build the model from `cargo metadata --format-version 1 --no-deps`.
pub fn from_metadata(meta: &Value) -> Result<Vec<Crate>, String> {
    let pkgs = meta["packages"].as_array().ok_or("metadata: no packages array")?;
    let members: Vec<&str> = pkgs.iter().filter_map(|p| p["name"].as_str()).collect();
    let mut out = Vec::new();
    for p in pkgs {
        let name = p["name"].as_str().ok_or("package without name")?.to_owned();
        let mut deps = Vec::new();
        for d in p["dependencies"].as_array().into_iter().flatten() {
            let dname = d["name"].as_str().unwrap_or_default().to_owned();
            let kind = match d["kind"].as_str() {
                Some("dev") => DepKind::Dev,
                Some("build") => DepKind::Build,
                _ => DepKind::Normal,
            };
            let workspace = members.contains(&dname.as_str()) || d["path"].is_string();
            deps.push(Dep { name: dname, kind, workspace });
        }
        out.push(Crate { name, deps });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn describe(class: Option<Class>) -> String {
    match class {
        Some(Class::Layer(l)) => format!("L{l}"),
        Some(Class::Testkit) => "testkit".into(),
        Some(Class::Exempt) => "exempt".into(),
        None => "UNREGISTERED".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DepKind::*;

    fn c(name: &str, deps: &[(&str, DepKind, bool)]) -> Crate {
        Crate { name: name.into(), deps: deps.iter().map(|(n, k, w)| Dep { name: (*n).into(), kind: *k, workspace: *w }).collect() }
    }

    #[test]
    fn clean_downward_graph_passes() {
        let g = [
            c("anicraft-geom", &[("kurbo", Normal, false)]),
            c("anicraft-doc", &[("anicraft-geom", Normal, true)]),
            c("anicraft-engine", &[("anicraft-doc", Normal, true), ("anicraft-testkit", Dev, true)]),
            c("anicraft-ui-egui", &[("anicraft-engine", Normal, true), ("egui", Normal, false)]),
            c("anicraft-cli", &[("anicraft-ui-egui", Normal, true)]),
        ];
        assert!(check(&g).is_empty(), "{:?}", check(&g));
    }

    #[test]
    fn upward_dependency_flagged() {
        let v = check(&[c("anicraft-doc", &[("anicraft-engine", Normal, true)])]);
        assert!(matches!(v[..], [Violation::Upward { from: 1, to: 5, .. }]));
    }

    #[test]
    fn sideways_dependency_flagged() {
        let v = check(&[c("anicraft-text", &[("anicraft-shapes", Normal, true)])]);
        assert!(matches!(v[..], [Violation::Upward { from: 2, to: 2, .. }]));
    }

    #[test]
    fn l0_foundations_are_independent() {
        // No intra-layer chain is declared for L0, so sideways L0 deps are flagged.
        let v = check(&[c("anicraft-color", &[("anicraft-geom", Normal, true)])]);
        assert!(matches!(v[..], [Violation::Upward { from: 0, to: 0, .. }]));
        // time and color (both L0, unrelated) pass the downward-only rule trivially.
        assert!(check(&[c("anicraft-time", &[("kurbo", Normal, false)])]).is_empty());
    }

    #[test]
    fn intra_layer_edges_allowed() {
        // render -> timeline, timeline -> render (bounds), export -> render, gpu -> render,
        // script -> engine (research/08 §2).
        assert!(check(&[c("anicraft-render", &[("anicraft-timeline", Normal, true)])]).is_empty());
        assert!(check(&[c("anicraft-timeline", &[("anicraft-render", Normal, true)])]).is_empty());
        assert!(check(&[c("anicraft-export", &[("anicraft-render", Normal, true)])]).is_empty());
        assert!(check(&[c("anicraft-gpu", &[("anicraft-render", Normal, true)])]).is_empty());
        assert!(check(&[c("anicraft-script", &[("anicraft-engine", Normal, true)])]).is_empty());
        // The reverse of a one-way edge is not allowed.
        let v = check(&[c("anicraft-render", &[("anicraft-export", Normal, true)])]);
        assert!(matches!(v[..], [Violation::Upward { from: 3, to: 3, .. }]));
    }

    #[test]
    fn self_dev_dependency_ignored() {
        assert!(check(&[c("anicraft-doc", &[("anicraft-doc", Dev, true)])]).is_empty());
    }

    #[test]
    fn upward_dev_dependency_flagged() {
        let v = check(&[c("anicraft-geom", &[("anicraft-doc", Dev, true)])]);
        assert!(matches!(v[..], [Violation::Upward { kind: Dev, .. }]));
    }

    #[test]
    fn ui_crates_below_l6_flagged() {
        for dep in ["egui", "eframe", "winit", "egui_kittest", "craft-ui", "rfd", "cpal", "muda"] {
            let v = check(&[c("anicraft-engine", &[(dep, Normal, false)])]);
            assert!(matches!(v[..], [Violation::UiBelowL6 { layer: 5, .. }]), "{dep}");
        }
        assert!(check(&[c("anicraft-engine", &[("egui_extras_something", Normal, false)])]).is_empty());
        assert!(check(&[c("anicraft-ui-egui", &[("craft-ui", Normal, false)])]).is_empty());
        assert!(check(&[c("anicraft-mcp", &[("winit", Normal, false)])]).is_empty());
    }

    #[test]
    fn unregistered_crate_is_error() {
        let v = check(&[c("anicraft-mystery", &[])]);
        assert!(matches!(&v[..], [Violation::Unregistered { krate }] if krate == "anicraft-mystery"));
        assert!(v[0].to_string().contains("register"));
    }

    #[test]
    fn testkit_only_as_dev_dependency() {
        let v = check(&[c("anicraft-render", &[("anicraft-testkit", Normal, true)])]);
        assert!(matches!(v[..], [Violation::TestkitAsNormalDep { .. }]));
        assert!(check(&[c("anicraft-render", &[("anicraft-testkit", Dev, true)])]).is_empty());
        // testkit itself may use anything up to L5 but not L6 crates.
        assert!(check(&[c("anicraft-testkit", &[("anicraft-engine", Normal, true)])]).is_empty());
        assert!(!check(&[c("anicraft-testkit", &[("anicraft-ui-egui", Normal, true)])]).is_empty());
    }

    #[test]
    fn apps_and_xtask_exempt() {
        for app in ["anicraft", "anicraft-cli", "anicraft-web", "xtask"] {
            assert!(check(&[c(app, &[("egui", Normal, false), ("anicraft-ui-egui", Normal, true)])]).is_empty());
        }
    }

    #[test]
    fn metadata_parsing() {
        let meta: Value = serde_json::from_str(
            r#"{"packages":[
                {"name":"anicraft-doc","dependencies":[
                    {"name":"anicraft-geom","kind":null,"path":"/x/crates/geom"},
                    {"name":"serde","kind":null},
                    {"name":"proptest","kind":"dev"}]},
                {"name":"anicraft-geom","dependencies":[]}
            ]}"#,
        )
        .unwrap();
        let g = from_metadata(&meta).unwrap();
        assert_eq!(g.len(), 2);
        let doc = g.iter().find(|c| c.name == "anicraft-doc").unwrap();
        assert!(doc.deps[0].workspace && !doc.deps[1].workspace);
        assert_eq!(doc.deps[2].kind, Dev);
        assert!(check(&g).is_empty());
    }
}
