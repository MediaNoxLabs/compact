//! Borrowed declarations and optional source observation for one native fold.
//! No additional semantic pass, ownership mapping or source-address invention.
use super::*;
use crate::source_diagnostics::{Owner, RefusalObserver, RefusalSite};

pub(crate) struct StateExpressionContext<'scope, 'ir> {
    pub(crate) witnesses: &'scope HashMap<&'ir str, &'ir WitnessDeclaration>,
    pub(crate) circuits: &'scope HashMap<&'ir str, &'ir PureCircuit>,
    pub(crate) stateful_circuits: &'scope HashMap<&'ir str, &'ir StatefulCircuit>,
    pub(crate) ledger_fields: &'scope HashMap<&'ir str, &'ir LedgerField>,
    pub(crate) observer: Option<&'scope dyn RefusalObserver>,
}
impl StateExpressionContext<'_, '_> {
    pub(super) fn refused(&self, source: &Expr) {
        if let Some(observer) = self.observer {
            observer.refused(RefusalSite::node(Owner::Expr(source)));
        }
    }
    pub(crate) fn pure_expression(
        &self,
        source: &Expr,
        parameters: &HashMap<&str, (&Type, syn::Ident)>,
    ) -> Result<(syn::Expr, Type), RenderError> {
        let Some(observer) = self.observer else {
            return expression_with_calls(source, parameters, self.circuits);
        };
        let mut callback = |source: &Expr, _: &RenderError| {
            observer.refused(RefusalSite::node(Owner::Expr(source)))
        };
        let (fragment, ty) =
            crate::native_fragment_with_refusal(source, parameters, self.circuits, &mut callback)?;
        Ok((fragment.into_expression(), ty))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod workers;
