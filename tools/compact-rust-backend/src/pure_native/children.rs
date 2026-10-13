// This file is part of Compact.
// Copyright (C) 2026 Midnight Foundation
// SPDX-License-Identifier: Apache-2.0

//! Transitional native traversal environment, outside shared semantic core.
//! It borrows scopes/declarations without rebuilding them per child. The complete
//! generic Expr visitor will replace this adapter before body CheckSink cutover.
use super::leaf::NativeLeafSink;
use crate::{
    RenderError,
    ir::{Expr, PureCircuit, Type},
    pure_semantics::{children::Children, sink::Typed},
};
use std::collections::HashMap;

pub(crate) struct NativeChildren<'call, 'ir> {
    bindings: &'call HashMap<&'ir str, (&'ir Type, syn::Ident)>,
    declarations: &'call HashMap<&'ir str, &'ir PureCircuit>,
}

impl<'call, 'ir> NativeChildren<'call, 'ir> {
    pub(crate) fn new(
        bindings: &'call HashMap<&'ir str, (&'ir Type, syn::Ident)>,
        declarations: &'call HashMap<&'ir str, &'ir PureCircuit>,
    ) -> Self {
        Self {
            bindings,
            declarations,
        }
    }
}

impl Children<NativeLeafSink> for NativeChildren<'_, '_> {
    fn visit(
        &mut self,
        source: &Expr,
        _: &mut NativeLeafSink,
    ) -> Result<Typed<syn::Expr>, RenderError> {
        let (output, ty) = crate::expression_with_calls(source, self.bindings, self.declarations)?;
        Ok(Typed { output, ty })
    }
}
