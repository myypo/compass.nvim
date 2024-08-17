use nvim_oxi::api::create_namespace;
use serde::de;

#[derive(Clone, Copy, Debug)]
pub struct Namespace {
    id: u32,
}

static NAMESPACE: std::sync::OnceLock<Namespace> = std::sync::OnceLock::new();
pub fn get_namespace() -> Namespace {
    *NAMESPACE.get_or_init(Namespace::default)
}

impl Default for Namespace {
    fn default() -> Self {
        create_namespace("compass").into()
    }
}

impl<'de> de::Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        struct NamespaceVisitor;

        impl<'de> de::Visitor<'de> for NamespaceVisitor {
            type Value = Namespace;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("namespace name as a string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(create_namespace(v).into())
            }
        }

        deserializer.deserialize_str(NamespaceVisitor)
    }
}

impl From<u32> for Namespace {
    fn from(id: u32) -> Self {
        Self { id }
    }
}
impl From<Namespace> for u32 {
    fn from(val: Namespace) -> Self {
        val.id
    }
}
