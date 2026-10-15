use crate::origins::transport::wire_shape;
use serde::{Deserialize, Deserializer};

#[derive(Debug)]
pub(crate) struct Table<T>(pub Vec<T>);

impl<T> Table<T> {
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Table<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        wire_shape::plain_table(deserializer)
    }
}

/// Required field whose JSON value may be null; omission is not the same thing.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub(crate) struct Nullable<T>(pub Option<T>);

fn required_nullable<'de, D, T>(deserializer: D) -> Result<Nullable<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Nullable)
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Document {
    pub schema_version: u64,
    #[serde(deserialize_with = "wire_shape::string_only")]
    pub coordinate_system: CoordinateSystem,
    pub end_exclusive: bool,
    #[serde(default, deserialize_with = "wire_shape::optional_object_table")]
    pub files: Option<Table<File>>,
    #[serde(default, deserialize_with = "wire_shape::optional_object_table")]
    pub origins: Option<Table<Origin>>,
    #[serde(default, deserialize_with = "wire_shape::optional_object_table")]
    pub addresses: Option<Table<Address>>,
    #[serde(default, deserialize_with = "wire_shape::optional_map")]
    pub error: Option<ProducerRefusal>,
}

#[derive(Debug, Deserialize)]
pub(crate) enum CoordinateSystem {
    #[serde(rename = "compact-decoded-source")]
    CompactDecodedSource,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct File {
    pub id: u64,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub(crate) struct Endpoint {
    pub line: u64,
    pub column: u64,
    pub offset: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Label {
    Provided,
    Derived,
    Inherited,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Category {
    Program,
    Declaration,
    Type,
    Expression,
    Argument,
    CounterAmount,
    StateAction,
    ReturnPlan,
    Identifier,
    Operation,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContextReason {
    ProgramVirtualEof,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Extent {
    PhysicalSource {},
    CompilerContext {
        #[serde(deserialize_with = "wire_shape::string_only")]
        reason: ContextReason,
        attachable: bool,
    },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub(crate) struct Origin {
    pub file_id: u64,
    #[serde(deserialize_with = "wire_shape::map_only")]
    pub start: Endpoint,
    #[serde(deserialize_with = "wire_shape::map_only")]
    pub end: Endpoint,
    #[serde(deserialize_with = "wire_shape::string_only")]
    pub label: Label,
    #[serde(deserialize_with = "wire_shape::string_only")]
    pub category: Category,
    #[serde(deserialize_with = "wire_shape::map_only")]
    pub extent: Extent,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootEdge {
    pub root: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub(crate) struct FieldEdge {
    pub field: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(deny_unknown_fields)]
pub(crate) struct IndexEdge {
    pub index: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub(crate) enum Edge {
    Root(RootEdge),
    Field(FieldEdge),
    Index(IndexEdge),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Address {
    pub id: u64,
    #[serde(deserialize_with = "required_nullable")]
    pub parent: Nullable<u64>,
    #[serde(deserialize_with = "wire_shape::map_only")]
    pub edge: Edge,
    #[serde(deserialize_with = "required_nullable")]
    pub kind: Nullable<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub origin: Nullable<u64>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum Resource {
    #[serde(rename = "origin_spans")]
    Spans,
    #[serde(rename = "origin_bytes")]
    Bytes,
    #[serde(rename = "origin_addresses")]
    Addresses,
    #[serde(rename = "origin_files")]
    Files,
    #[serde(rename = "origin_records")]
    Records,
    #[serde(rename = "origin_pending")]
    Pending,
    #[serde(rename = "origin_path_characters")]
    PathCharacters,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProducerRefusal {
    #[serde(deserialize_with = "wire_shape::string_only")]
    pub resource: Resource,
    pub observed: u64,
    pub limit: u64,
}

#[derive(Debug)]
pub(crate) struct Tables {
    pub files: Table<File>,
    pub origins: Table<Origin>,
    pub addresses: Table<Address>,
}
