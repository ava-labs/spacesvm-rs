use std::{fmt, marker::PhantomData};

use serde::de::{Deserialize, Deserializer, SeqAccess, Visitor};

/// Custom Serde deserializer for lists of boxed traits. Activate
/// by adding the following above your structs field.
/// #[serde(deserialize_with = "from_boxed_seq")]
pub fn from_boxed_seq<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct GenericObjects<T> {
        _type: PhantomData<T>,
    }

    impl<'de, T> Visitor<'de> for GenericObjects<T>
    where
        T: Deserialize<'de>,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("list of dyn traits")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let len = seq.size_hint().unwrap_or(0);
            let mut values = Vec::with_capacity(len);
            while let Some(value) = seq.next_element()? {
                values.push(value);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(GenericObjects { _type: PhantomData })
}
