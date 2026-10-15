use super::*;

#[derive(Debug, PartialEq, Eq)]
struct Key(Option<usize>, Slot);
#[derive(Default)]
pub(super) struct Trace(RefCell<Vec<Key>>);
fn key(site: RefusalSite<'_>) -> Key {
    let owner = site.owner.map(|owner| match owner {
        Owner::Type(ty) => std::ptr::from_ref(ty) as usize,
        Owner::StructField(field) => std::ptr::from_ref(field) as usize,
        _ => panic!("unexpected source kind in typed environment"),
    });
    Key(owner, site.slot)
}
impl RefusalObserver for Trace {
    fn refused(&self, site: RefusalSite<'_>) {
        self.0.borrow_mut().push(key(site));
    }
}
pub(super) fn one(trace: &Trace, site: RefusalSite<'_>) {
    assert_eq!(trace.0.borrow().as_slice(), &[key(site)]);
}
pub(super) fn none(trace: &Trace) {
    assert!(trace.0.borrow().is_empty());
}
pub(super) fn enumeration(name: &str, variants: &[&str]) -> Type {
    Type::Enum {
        name: name.into(),
        variants: variants.iter().map(|v| (*v).into()).collect(),
    }
}
pub(super) fn member(name: &str, ty: Type) -> StructField {
    StructField {
        name: name.into(),
        ty,
    }
}
pub(super) fn structure(name: &str, fields: Vec<StructField>) -> Type {
    Type::Struct {
        name: name.into(),
        fields,
    }
}
pub(super) fn source(ty: Type) -> StatefulCircuit {
    StatefulCircuit {
        source: None,
        name: "root".into(),
        internal: false,
        parameters: vec![Parameter {
            name: "unused".into(),
            ty,
        }],
        actions: vec![],
        result: Type::Unit,
        return_value: StateReturn::Unit,
    }
}
pub(super) fn env(
    root: &StatefulCircuit,
    observer: Option<&dyn RefusalObserver>,
) -> Result<TypeEnvironment, RecordingGap> {
    TypeEnvironment::from_sources_observed(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        observer,
    )
}
pub(super) fn old_env(
    root: &StatefulCircuit,
) -> Result<old_reference::TypeEnvironment, RecordingGap> {
    old_reference::TypeEnvironment::from_sources(
        root,
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
        &HashMap::new(),
    )
}
pub(super) fn validation(ty: &Type, site: RefusalSite<'_>) -> RecordingGap {
    let env = TypeEnvironment::default();
    let old = old_reference::TypeEnvironment::default();
    let trace = Trace::default();
    let observed = env.validate_at_observed(ty, "actual.path", Some(&trace));
    assert_eq!(observed, env.validate_at(ty, "actual.path"));
    assert_eq!(observed, old.validate_at(ty, "actual.path"));
    one(&trace, site);
    observed.expect_err("existing typed failure")
}
