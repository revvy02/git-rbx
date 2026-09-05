//! The Studio front ends ship inside the binary as darklua bundles built by
//! `mise run bundle` into studio-viewer/dist/. A bundle is only usable if
//! every require left in it is one rodeo resolves at runtime (its own API
//! and the lune/lute adapters); anything else means a module was missed.
const RESOLVER: &str = include_str!("../studio-viewer/dist/conflict-resolver.luau");
const VIEWER: &str = include_str!("../studio-viewer/dist/diff-viewer.luau");

fn requires(bundle: &str) -> Vec<&str> {
    bundle
        .match_indices("require(\"")
        .map(|(start, _)| {
            let rest = &bundle[start + 9..];
            &rest[..rest.find('"').unwrap()]
        })
        .collect()
}

fn check(name: &str, bundle: &str) {
    assert!(
        bundle.len() > 50_000,
        "{name} bundle is implausibly small ({} bytes)",
        bundle.len()
    );
    for spec in requires(bundle) {
        assert!(
            spec.starts_with("@rodeo") || spec.starts_with("@lune") || spec.starts_with("@lute"),
            "{name} bundle still requires `{spec}`; run `mise run bundle`"
        );
    }
    assert!(
        bundle.contains("GitRbxPackages") && bundle.contains("DeserializeInstancesAsync"),
        "{name} bundle must carry the embedded roblox packages"
    );
}

#[test]
fn conflict_resolver_bundle_is_self_contained() {
    check("conflict-resolver", RESOLVER);
}

#[test]
fn diff_viewer_bundle_is_self_contained() {
    check("diff-viewer", VIEWER);
}
