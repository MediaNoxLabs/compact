use super::*;
use serde_json::json;

fn non_path_fields(map: &Value) -> Value {
    let mut value = map.clone();
    let fields = value.as_object_mut().unwrap();
    fields.remove("sourceRoot");
    fields.remove("sources");
    value
}

#[test]
fn real_combined_map_rebases_original_child_to_final_output_without_changing_mappings() {
    let mut map: Value = serde_json::from_slice(include_bytes!("combined-counter.json")).unwrap();
    let before = map.clone();
    // Exact directory relationships from the genuine combined-target finding.
    let original = Path::new("/evidence/implementation/.combined-stage/.invocation/contract");
    let published = Path::new("/evidence/implementation/publication-combined-70/contract");
    assert!(rebase_value(&mut map, original, published).unwrap());
    assert_eq!(map["sourceRoot"], "../../publication-genuine-69/");
    assert_eq!(map["sources"], before["sources"]);
    assert_eq!(non_path_fields(&map), non_path_fields(&before));
    assert_eq!(
        normalize(
            &published
                .join(map["sourceRoot"].as_str().unwrap())
                .join("counter.compact")
        ),
        PathBuf::from("/evidence/implementation/publication-genuine-69/counter.compact")
    );
}

#[test]
fn real_rustc_map_handles_nontrivial_directory_depth_and_relative_names() {
    let mut map: Value = serde_json::from_slice(include_bytes!("rustc-counter.json")).unwrap();
    let original = Path::new(
        "/evidence/implementation/publication-genuine-69/.small-stage/.invocation/contract",
    );
    let published =
        Path::new("/evidence/implementation/publication-genuine-69/compact-rustc-success/contract");
    let before = map.clone();
    assert!(rebase_value(&mut map, original, published).unwrap());
    assert_eq!(map["sourceRoot"], "../../");
    assert_eq!(map["sources"], before["sources"]);
    assert_eq!(non_path_fields(&map), non_path_fields(&before));
    assert_eq!(
        normalize(&published.join("../../counter.compact")),
        PathBuf::from("/evidence/implementation/publication-genuine-69/counter.compact")
    );
}

#[test]
fn no_root_rebases_each_relative_source_and_keeps_absolute_and_url_values() {
    let mut map = json!({"version":3,"file":"index.js","sources":["./parts/identity.compact","../shared/β.compact","/absolute/source.compact","https://example.test/source.compact","webpack:///entry.compact"],"sourcesContent":["first",null,"third",null,null],"names":["member"],"mappings":"AAAA;AACA","x_extra":{"opaque":true}});
    let before = map.clone();
    assert!(
        rebase_value(
            &mut map,
            Path::new("/project/work/a/b/contract"),
            Path::new("/project/release/contract")
        )
        .unwrap()
    );
    assert_eq!(
        map["sources"][0],
        "../../work/a/b/contract/parts/identity.compact"
    );
    assert_eq!(map["sources"][1], "../../work/a/b/shared/β.compact");
    assert_eq!(
        map["sources"].as_array().unwrap()[2..],
        before["sources"].as_array().unwrap()[2..]
    );
    assert_eq!(non_path_fields(&map), non_path_fields(&before));
    assert!(map.get("sourceRoot").is_none());
}

#[test]
fn absolute_url_and_drive_roots_remain_wholly_unchanged() {
    for root in [
        "/absolute/",
        "https://example.test/src/",
        "webpack:///src/",
        "file:///source/",
        "C:/source/",
        "\\\\server\\share\\",
    ] {
        let mut map =
            json!({"version":3,"sourceRoot":root,"sources":["relative.compact"],"mappings":"AAAA"});
        let before = map.clone();
        assert!(
            !rebase_value(
                &mut map,
                Path::new("/first/contract"),
                Path::new("/second/contract")
            )
            .unwrap()
        );
        assert_eq!(map, before);
    }
}

#[test]
fn empty_root_preserves_source_directory_semantics_and_content() {
    let mut map = json!({"version":3,"sourceRoot":"","sources":["source.compact"],"sourcesContent":["contract source"],"mappings":"AAAA"});
    let before = map.clone();
    assert!(
        rebase_value(
            &mut map,
            Path::new("/work/extra/child/contract"),
            Path::new("/release/contract")
        )
        .unwrap()
    );
    assert_eq!(map["sourceRoot"], "../../work/extra/child/contract");
    assert_eq!(map["sources"], before["sources"]);
    assert_eq!(non_path_fields(&map), non_path_fields(&before));
}
