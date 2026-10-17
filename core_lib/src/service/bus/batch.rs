//! Bounded recipient-list decoding; all authorization and receipt ownership stay in the bus/core.
use std::fmt;

use serde::de::{self, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

#[derive(zbus::zvariant::Type)]
#[zvariant(signature = "as")]
pub(super) struct Recipients(pub Vec<String>);
impl<'de> Deserialize<'de> for Recipients {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Bounded;
        impl<'de> Visitor<'de> for Bounded {
            type Value = Recipients;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bounded unique recipient IDs")
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut sequence: A,
            ) -> Result<Self::Value, A::Error> {
                let mut ids = Vec::new();
                while let Some(id) = sequence.next_element::<String>()? {
                    if ids.len() == crate::service::batch::RECIPIENTS {
                        return Err(de::Error::custom("Too many recipients"));
                    }
                    crate::service::identifier(&id, 256)
                        .map_err(|_| de::Error::custom("Invalid recipient"))?;
                    if ids.contains(&id) {
                        return Err(de::Error::custom("Duplicate recipient"));
                    }
                    ids.push(id);
                }
                if ids.is_empty() {
                    return Err(de::Error::custom("Missing recipients"));
                }
                Ok(Recipients(ids))
            }
        }
        deserializer.deserialize_seq(Bounded)
    }
}
