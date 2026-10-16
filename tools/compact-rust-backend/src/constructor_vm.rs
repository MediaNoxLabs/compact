//! Output state and the deliberately restricted pure constructor expression adapter.
//!
//! VM step recursion keeps its original source owners. Restricted operands retain
//! the caller's infallibility guard and an empty callee scope in both modes.
use crate::ir::{Expr, Type};
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};
use crate::{RenderError, expression_with_calls, native_fragment_with_refusal};
use std::collections::HashMap;

pub(crate) struct Output<'output> {
    pub(crate) next_loop: &'output mut usize,
    pub(crate) next_temp: &'output mut usize,
}

pub(crate) fn pure_expression(
    value: &Expr,
    parameters: &HashMap<&str, (&Type, syn::Ident)>,
    observer: Option<&dyn RefusalObserver>,
) -> Result<(syn::Expr, Type), RenderError> {
    let circuits = HashMap::new();
    let Some(observer) = observer else {
        return expression_with_calls(value, parameters, &circuits);
    };
    let mut callback = |value: &Expr, _: &RenderError| {
        observer.refused(RefusalSite::node(Owner::Expr(value)));
    };
    let (fragment, ty) = native_fragment_with_refusal(value, parameters, &circuits, &mut callback)?;
    Ok((fragment.into_expression(), ty))
}

pub(crate) fn refused(site: RefusalSite<'_>, observer: Option<&dyn RefusalObserver>) {
    if let Some(observer) = observer {
        observer.refused(site);
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod collection_guard_tests;
