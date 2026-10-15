# Synthetic nested point layout

This separate fixture crate mutates retained IR to the native-valid layout for
16 point Cells: first `[0,0]`, second `[1,0]`, padding `[1,1]` through `[1,14]`.
Its constructor/source annotations are removed; tests provide exact runtime
state. It has no source/proof qualification claim. Tests compare generated native
and recorded execution with an official-slot program, transcript, state/effects,
execution gas, independent replay gas, neighbor preservation and invalid flat
runtime layout refusal. The source-bound proof fixture remains separate.
