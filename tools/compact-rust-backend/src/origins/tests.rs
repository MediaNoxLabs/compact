//! Only composition/lifecycle tests are registered here. Qualified codec/binder
//! suites are preserved as preimages and remain separate independent gates.
mod fixture;
mod invocation;
mod native_capture;
mod precedence;
mod resources;
mod source_text;

mod recording_capture;

mod recorded_stateful_relations;

mod recorded_borrowed_operations;

mod recorded_type_environment;
