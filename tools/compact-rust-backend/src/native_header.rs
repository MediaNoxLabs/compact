//! Fatal header validation adapters; allocator probes stay originless.
use crate::{
    RenderError,
    ir::Type,
    naming::ident,
    source_diagnostics::{RefusalObserver, RefusalSite},
};
pub(crate) fn identifier(
    name: &str,
    site: RefusalSite<'_>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<syn::Ident, RenderError> {
    ident(name).inspect_err(|_| {
        if let Some(observer) = observer {
            observer.refused(site);
        }
    })
}
pub(crate) fn carrier(
    ty: &Type,
    observer: Option<&dyn RefusalObserver>,
) -> Result<syn::Type, RenderError> {
    crate::value_lowering::rust_type_observed(ty, observer)
}
