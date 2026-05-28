macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(s: impl Into<String>) -> Self {
                Self(s.into())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(s: &str) -> Self {
                Self(s.to_owned())
            }
        }
        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }
    };
}

id_newtype!(SpecObjectId);
id_newtype!(SpecTypeId);
id_newtype!(DataTypeId);
id_newtype!(AttributeDefId);
id_newtype!(SpecificationId);
id_newtype!(SpecRelationId);
id_newtype!(RelationGroupId);
id_newtype!(EnumValueId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_distinct_types_but_share_string_eq() {
        let so = SpecObjectId::new("ID-001");
        let st = SpecTypeId::new("ID-001");
        assert_eq!(so.as_str(), st.as_str());
        // The following would not compile, which is the whole point:
        // assert_eq!(so, st);
    }

    #[test]
    fn display_round_trips_str() {
        let id = DataTypeId::from("DT-STRING");
        assert_eq!(id.to_string(), "DT-STRING");
    }
}
