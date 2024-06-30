use std::slice::Iter;

use serde::{de, Deserialize};

#[derive(Debug, Deserialize)]
pub struct JumpKeymapList(Vec<JumpKeymap>);

#[derive(Debug, PartialEq, Eq)]
pub struct JumpKeymap {
    pub follow: String,
    pub immediate: String,
}

impl<'de> de::Deserialize<'de> for JumpKeymap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct JumpKeymapVisitor;

        impl<'de> de::Visitor<'de> for JumpKeymapVisitor {
            type Value = JumpKeymap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter
                    .write_str("a tuple consisting of two strings where the first one is a keymap to follow the preview to see more options and the second is to jump straight away to the place highlighted in the preview")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let follow = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let immediate = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                Ok(JumpKeymap { immediate, follow })
            }
        }

        deserializer.deserialize_tuple(2, JumpKeymapVisitor)
    }
}

impl JumpKeymapList {
    pub fn get(&self, idx: usize) -> Option<&JumpKeymap> {
        self.0.get(idx)
    }

    pub fn iter(&self) -> Iter<JumpKeymap> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Default for JumpKeymapList {
    fn default() -> Self {
        let follow = ["j", "f", "k", "d", "l", "s", ";", "a", "h", "g"];

        Self(
            follow
                .iter()
                .map(|&ik| JumpKeymap {
                    follow: ik.to_owned(),
                    immediate: ik.to_owned().to_uppercase(),
                })
                .collect::<Vec<JumpKeymap>>(),
        )
    }
}
