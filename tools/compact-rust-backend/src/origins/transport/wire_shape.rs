//! Strict JSON representation boundaries without a Value allocation/prepass.
use crate::origins::transport::MAX_ENTRIES;
use crate::origins::transport::model::Table;
use serde::de::value::{MapAccessDeserializer, StrDeserializer};
use serde::de::{self, DeserializeSeed, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use std::marker::PhantomData;

/// Forward a JSON object to the typed derive, preserving duplicate-field checks.
pub(crate) fn map_only<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct MapVisitor<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for MapVisitor<T> {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON object")
        }
        fn visit_map<A: MapAccess<'de>>(self, access: A) -> Result<T, A::Error> {
            T::deserialize(MapAccessDeserializer::new(access))
        }
    }
    deserializer.deserialize_map(MapVisitor(PhantomData))
}

/// Unit enums are strings in this wire format, never {"variant":null} maps.
pub(crate) fn string_only<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct StringVisitor<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for StringVisitor<T> {
        type Value = T;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON string")
        }
        fn visit_str<E: de::Error>(self, value: &str) -> Result<T, E> {
            T::deserialize(StrDeserializer::<E>::new(value))
        }
    }
    deserializer.deserialize_str(StringVisitor(PhantomData))
}

struct PlainSeed<T>(PhantomData<T>);
impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for PlainSeed<T> {
    type Value = T;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
        T::deserialize(deserializer)
    }
}

struct ObjectSeed<T>(PhantomData<T>);
impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for ObjectSeed<T> {
    type Value = T;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<T, D::Error> {
        map_only(deserializer)
    }
}

/// One bounded table collector shared by ordinary and object-only row seeds.
/// It does not allocate a second table or trust the caller's length hint.
fn table_with_seed<'de, D, T, S, F>(deserializer: D, make_seed: F) -> Result<Table<T>, D::Error>
where
    D: Deserializer<'de>,
    S: DeserializeSeed<'de, Value = T>,
    F: Fn() -> S,
{
    struct TableVisitor<T, F> {
        make_seed: F,
        marker: PhantomData<T>,
    }
    impl<'de, T, S, F> Visitor<'de> for TableVisitor<T, F>
    where
        S: DeserializeSeed<'de, Value = T>,
        F: Fn() -> S,
    {
        type Value = Table<T>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON array with at most 131072 entries")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Table<T>, A::Error> {
            let mut values = Vec::new();
            loop {
                if values.len() == MAX_ENTRIES {
                    if sequence.next_element::<IgnoredAny>()?.is_some() {
                        return Err(de::Error::custom("origin table entry limit exceeded"));
                    }
                    break;
                }
                match sequence.next_element_seed((self.make_seed)())? {
                    Some(value) => values.push(value),
                    None => break,
                }
            }
            Ok(Table(values))
        }
    }
    deserializer.deserialize_seq(TableVisitor {
        make_seed,
        marker: PhantomData,
    })
}

pub(crate) fn plain_table<'de, D, T>(deserializer: D) -> Result<Table<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    table_with_seed(deserializer, || PlainSeed::<T>(PhantomData))
}

/// Missing and null tables stay None; present rows must be JSON objects.
pub(crate) fn optional_object_table<'de, D, T>(
    deserializer: D,
) -> Result<Option<Table<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct OptionalTable<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for OptionalTable<T> {
        type Value = Option<Table<T>>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("null or a JSON array of object rows")
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            table_with_seed(deserializer, || ObjectSeed::<T>(PhantomData)).map(Some)
        }
    }
    deserializer.deserialize_option(OptionalTable(PhantomData))
}

pub(crate) fn optional_table<'de, D, T>(deserializer: D) -> Result<Option<Table<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct OptionalTable<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for OptionalTable<T> {
        type Value = Option<Table<T>>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("null or a JSON array of typed rows")
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            plain_table(deserializer).map(Some)
        }
    }
    deserializer.deserialize_option(OptionalTable(PhantomData))
}

pub(crate) fn optional_map<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct OptionalMap<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de>> Visitor<'de> for OptionalMap<T> {
        type Value = Option<T>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("null or a JSON object")
        }
        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }
        fn visit_some<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            map_only(deserializer).map(Some)
        }
    }
    deserializer.deserialize_option(OptionalMap(PhantomData))
}
