use super::*;

#[test]
fn absent_metadata_real_recorder_keeps_same_gap_and_never_invents_parent_range() {
    // Private direct certifier entry, with the real Recorder and no transport.
    // This is not a genuine frontend or public native-render acceptance claim.
    let sources = [source("root", call("absent"))];
    let pure = HashMap::from([("root", &sources[0])]);
    let stateful = HashMap::new();
    let root = root();
    let recorder =
        crate::origins::recording::Recorder::new(None, None, None, std::path::Path::new("."));
    recorder.begin_recording_root(&root);
    let expected = Certifier::new(&pure, &stateful)
        .certify("root", "root.call")
        .unwrap_err();
    let actual = Certifier::new(&pure, &stateful)
        .certify_observed("root", "root.call", Some(&recorder))
        .unwrap_err();
    assert_eq!(actual, expected);
    recorder.refused(RefusalSite::node(Owner::Stateful(&root)));
    recorder.finish_recording_root(&root, Some(&actual));
    let diagnostics = recorder.take_completed();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].gap, expected);
    assert!(diagnostics[0].primary.is_none());
    assert!(diagnostics[0].related_call_sites.is_empty());
}

#[test]
fn successful_fresh_and_cached_certification_keep_source_work_depth_and_silent_events() {
    let sources = [source("root", call("leaf")), source("leaf", literal())];
    let pure = sources.iter().map(|s| (s.name.as_str(), s)).collect();
    let stateful = HashMap::new();
    let trace = Trace::default();
    let mut plain = Certifier::new(&pure, &stateful);
    let mut observed = Certifier::new(&pure, &stateful);
    plain.certify("root", "one").unwrap();
    observed
        .certify_observed("root", "one", Some(&trace))
        .unwrap();
    for name in ["leaf", "root"] {
        let expected = plain.get(name).unwrap();
        let actual = observed.get(name).unwrap();
        assert_eq!(actual.source(), expected.source());
        assert_eq!(actual.call_depth(), expected.call_depth());
        assert_eq!(actual.expanded_work(), expected.expanded_work());
    }
    let before = std::ptr::from_ref(observed.get("root").unwrap());
    observed
        .certify_observed("root", "two", Some(&trace))
        .unwrap();
    assert_eq!(std::ptr::from_ref(observed.get("root").unwrap()), before);
    assert!(trace.events.borrow().is_empty());
    assert!(trace.related.borrow().is_empty());
}
