//! Frozen H197 assertion/query helpers; imports/visibility/name alone adapted.
//! Other expression child owners remain unchanged semantic dependencies.
use super::*;
mod assertions;
mod queries;
impl Lowerer<'_> {
    pub(super) fn old_synthetic_expression(
        &mut self,
        expression: &Expr,
        scope: &Scope,
        bindings: &mut Vec<Binding>,
        path: &str,
    ) -> Result<Value, RecordingGap> {
        self.spend(path)?;
        let (operation, ty) = match expression {
            Expr::Assert { .. } => {
                self.old_assertion_operation(expression, scope, bindings, path)?
            }
            _ => self.old_query_operation(expression, scope, bindings, path)?,
        };
        self.declarations.types().validate_at(&ty, path)?;
        Ok(self.emit(operation, ty, bindings, path))
    }
}
