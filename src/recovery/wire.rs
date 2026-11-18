//! Streaming serde form of a recovery checkpoint.

use std::fmt::{
    self,
    Formatter,
};
use std::marker::PhantomData;

use serde::de::{
    self,
    DeserializeSeed,
    MapAccess,
    Visitor,
};
use serde::ser::SerializeStruct;
use serde::{
    Deserialize,
    Deserializer,
    Serialize,
    Serializer,
};

use super::{
    RecoveredTask,
    RecoveryCheckpoint,
    RecoveryCheckpointBuilder,
};
use crate::{
    ActivitySeq,
    TaskId,
    TerminalTailLimit,
    UrnScheme,
};

const NAME: &str = "RecoveryCheckpoint";
const CURSOR: &str = "cursor";
const TAIL_LIMIT: &str = "tail_limit";
const TASKS: &str = "tasks";
const FIELDS: &[&str] = &[CURSOR, TAIL_LIMIT, TASKS];

impl<S: UrnScheme, T: Serialize> Serialize for RecoveryCheckpoint<S, T> {
    fn serialize<Z: Serializer>(&self, serializer: Z) -> Result<Z::Ok, Z::Error> {
        let mut state = serializer.serialize_struct(NAME, FIELDS.len())?;
        state.serialize_field(CURSOR, &self.cursor())?;
        state.serialize_field(TAIL_LIMIT, &self.tail_limit())?;
        state.serialize_field(TASKS, self.entries())?;
        state.end()
    }
}

impl<'de, S, T> Deserialize<'de> for RecoveryCheckpoint<S, T>
where
    S: UrnScheme,
    T: RecoveredTask<S> + Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_struct(NAME, FIELDS, CheckpointVisitor(PhantomData))
    }
}

struct CheckpointVisitor<S, T>(PhantomData<(S, T)>);

impl<'de, S, T> Visitor<'de> for CheckpointVisitor<S, T>
where
    S: UrnScheme,
    T: RecoveredTask<S> + Deserialize<'de>,
{
    type Value = RecoveryCheckpoint<S, T>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("a recovery checkpoint with cursor, tail_limit, then tasks")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let cursor = Self::expect_field::<A, ActivitySeq>(&mut map, CURSOR)?;
        let tail_limit = Self::expect_field::<A, TerminalTailLimit>(&mut map, TAIL_LIMIT)?;
        Self::expect_key(&mut map, TASKS)?;
        let builder = RecoveryCheckpointBuilder::new(cursor, tail_limit);
        let checkpoint = map.next_value_seed(TasksSeed(builder))?;
        match map.next_key::<de::IgnoredAny>()? {
            Some(_) => Err(de::Error::custom("unexpected field after tasks")),
            None => Ok(checkpoint),
        }
    }
}

impl<'de, S, T> CheckpointVisitor<S, T>
where
    S: UrnScheme,
    T: RecoveredTask<S> + Deserialize<'de>,
{
    fn expect_key<A: MapAccess<'de>>(map: &mut A, field: &'static str) -> Result<(), A::Error> {
        match map.next_key::<FieldName>()? {
            Some(FieldName(name)) if name == field => Ok(()),
            Some(FieldName(name)) => Err(de::Error::custom(format_args!(
                "expected field `{field}`, found `{name}`"
            ))),
            None => Err(de::Error::missing_field(field)),
        }
    }

    fn expect_field<A, V>(map: &mut A, field: &'static str) -> Result<V, A::Error>
    where
        A: MapAccess<'de>,
        V: Deserialize<'de>,
    {
        Self::expect_key(map, field)?;
        map.next_value()
    }
}

/// One field name of the fixed checkpoint layout.
struct FieldName(String);

impl<'de> Deserialize<'de> for FieldName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self)
    }
}

struct TasksSeed<S: UrnScheme, T>(RecoveryCheckpointBuilder<S, T>);

impl<'de, S, T> DeserializeSeed<'de> for TasksSeed<S, T>
where
    S: UrnScheme,
    T: RecoveredTask<S> + Deserialize<'de>,
{
    type Value = RecoveryCheckpoint<S, T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de, S, T> Visitor<'de> for TasksSeed<S, T>
where
    S: UrnScheme,
    T: RecoveredTask<S> + Deserialize<'de>,
{
    type Value = RecoveryCheckpoint<S, T>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str("a map of task identities to retained tasks")
    }

    fn visit_map<A: MapAccess<'de>>(mut self, mut map: A) -> Result<Self::Value, A::Error> {
        while let Some(key) = map.next_key::<TaskId<S>>()? {
            let task = map.next_value::<T>()?;
            self.0.insert(key, task).map_err(de::Error::custom)?;
        }
        Ok(self.0.build())
    }
}
