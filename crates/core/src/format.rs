//! The JSON files `advent seal` and `advent keeper seal` write. Every file
//! carries `schema` and `sealed_by`: an engine reads only its own schema and
//! refuses anything else with the version that sealed it, so an event repo
//! is upgraded by re-sealing from sources rather than by compatibility code.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::primitives::{Iv, Salt, WordLock};

/// The sealed-file schema this engine reads and writes.
pub const SCHEMA: u32 = 1;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FormatError {
    #[error(
        "sealed with {sealed_by} (schema {found}); this engine reads schema {SCHEMA}. \
         Install {sealed_by} to work with this event, or re-seal it with `advent reseal --all`."
    )]
    UnsupportedSchema { found: u32, sealed_by: String },
    #[error("not a valid sealed file: {0}")]
    Parse(String),
}

/// A sealed day: everything committed for it apart from `source.age`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedDay {
    pub schema: u32,
    pub sealed_by: String,
    pub day: u32,
    /// The puzzle bundle, encrypted with the day key.
    pub bundle: Encrypted,
    /// The day key, age-wrapped to the release recipients.
    #[serde(with = "b64")]
    pub day_key: Vec<u8>,
    /// One lock per accepted answer, each opening to the day's word.
    pub word_locks: Vec<WordLock>,
    pub hints: Vec<SealedHint>,
}

/// AES-GCM ciphertext with the IV it was sealed under.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encrypted {
    #[serde(with = "b64_array")]
    pub iv: Iv,
    #[serde(with = "b64")]
    pub ct: Vec<u8>,
}

/// A hint, encrypted with its own key so it can be released after the day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedHint {
    #[serde(with = "b64_array")]
    pub iv: Iv,
    #[serde(with = "b64")]
    pub ct: Vec<u8>,
    /// The hint key, age-wrapped to the release recipients.
    #[serde(with = "b64")]
    pub key: Vec<u8>,
    /// Days after the door opens when this hint is due, if the author set
    /// it; otherwise the event's `hint_after` applies. Plain on purpose: the
    /// build has only the release key, and the timing reveals nothing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<u32>,
}

/// The sealed certificate payload of an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedFinal {
    pub schema: u32,
    pub sealed_by: String,
    #[serde(with = "b64_array")]
    pub salt: Salt,
    #[serde(with = "b64_array")]
    pub iv: Iv,
    #[serde(with = "b64")]
    pub ct: Vec<u8>,
}

/// What the day key decrypts to: everything a player sees when the day opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub title: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub markdown: String,
    /// Image files by name, already converted to WebP.
    #[serde(with = "b64_map")]
    pub assets: BTreeMap<String, Vec<u8>>,
}

/// What the final key decrypts to: the certificate's content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Certificate {
    pub title: String,
    pub message: String,
    /// WebP image, when the keeper provided one.
    #[serde(default, with = "b64_opt", skip_serializing_if = "Option::is_none")]
    pub image: Option<Vec<u8>>,
}

impl Serialize for WordLock {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Repr<'a> {
            #[serde(with = "b64_array")]
            salt: &'a Salt,
            #[serde(with = "b64_array")]
            iv: &'a Iv,
            #[serde(with = "b64")]
            ct: &'a Vec<u8>,
        }
        Repr {
            salt: &self.salt,
            iv: &self.iv,
            ct: &self.ct,
        }
        .serialize(s)
    }
}

impl<'de> Deserialize<'de> for WordLock {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Repr {
            #[serde(with = "b64_array")]
            salt: Salt,
            #[serde(with = "b64_array")]
            iv: Iv,
            #[serde(with = "b64")]
            ct: Vec<u8>,
        }
        let r = Repr::deserialize(d)?;
        Ok(WordLock {
            salt: r.salt,
            iv: r.iv,
            ct: r.ct,
        })
    }
}

/// Just enough of any sealed file to decide whether this engine can read it,
/// so a future layout that no longer parses still gets the friendly error.
#[derive(Deserialize)]
struct SchemaProbe {
    schema: Option<u32>,
    sealed_by: Option<String>,
}

fn check_schema(bytes: &[u8]) -> Result<(), FormatError> {
    let probe: SchemaProbe =
        serde_json::from_slice(bytes).map_err(|e| FormatError::Parse(e.to_string()))?;
    match probe.schema {
        None => Err(FormatError::Parse("missing `schema` field".into())),
        Some(SCHEMA) => Ok(()),
        Some(found) => Err(FormatError::UnsupportedSchema {
            found,
            sealed_by: probe
                .sealed_by
                .unwrap_or_else(|| "an unknown engine".into()),
        }),
    }
}

fn read<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, FormatError> {
    check_schema(bytes)?;
    serde_json::from_slice(bytes).map_err(|e| FormatError::Parse(e.to_string()))
}

pub fn read_sealed_day(bytes: &[u8]) -> Result<SealedDay, FormatError> {
    read(bytes)
}

pub fn read_sealed_final(bytes: &[u8]) -> Result<SealedFinal, FormatError> {
    read(bytes)
}

/// Standard base64 with padding, matching the browser's `btoa` / `atob`.
mod b64 {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let text = String::deserialize(d)?;
        STANDARD.decode(text).map_err(serde::de::Error::custom)
    }
}

mod b64_array {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer, const N: usize>(
        bytes: &[u8; N],
        s: S,
    ) -> Result<S::Ok, S::Error> {
        super::b64::serialize(bytes, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>, const N: usize>(
        d: D,
    ) -> Result<[u8; N], D::Error> {
        let bytes = super::b64::deserialize(d)?;
        let len = bytes.len();
        bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom(format!("expected {N} bytes, got {len}")))
    }
}

mod b64_opt {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bytes: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match bytes {
            Some(b) => super::b64::serialize(b, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        use base64::Engine;
        Option::<String>::deserialize(d)?
            .map(|text| {
                base64::engine::general_purpose::STANDARD
                    .decode(text)
                    .map_err(serde::de::Error::custom)
            })
            .transpose()
    }
}

mod b64_map {
    use std::collections::BTreeMap;

    use base64::Engine;
    use base64::engine::general_purpose::STANDARD;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        map: &BTreeMap<String, Vec<u8>>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter()
            .map(|(k, v)| (k, STANDARD.encode(v)))
            .collect::<BTreeMap<_, _>>()
            .serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<BTreeMap<String, Vec<u8>>, D::Error> {
        BTreeMap::<String, String>::deserialize(d)?
            .into_iter()
            .map(|(k, v)| {
                STANDARD
                    .decode(v)
                    .map(|bytes| (k, bytes))
                    .map_err(serde::de::Error::custom)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_day() -> SealedDay {
        SealedDay {
            schema: SCHEMA,
            sealed_by: "advent 0.1.0".into(),
            day: 5,
            bundle: Encrypted {
                iv: [1; 12],
                ct: vec![2, 3, 4],
            },
            day_key: vec![5, 6],
            word_locks: vec![WordLock {
                salt: [7; 16],
                iv: [8; 12],
                ct: vec![9],
            }],
            hints: vec![SealedHint {
                iv: [10; 12],
                ct: vec![11],
                key: vec![12],
                after: None,
            }],
        }
    }

    #[test]
    fn format_sealed_day_round_trips_with_expected_keys() {
        let day = sample_day();
        let json = serde_json::to_vec(&day).unwrap();
        assert_eq!(read_sealed_day(&json).unwrap(), day);

        let value: serde_json::Value = serde_json::from_slice(&json).unwrap();
        let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "bundle",
                "day",
                "day_key",
                "hints",
                "schema",
                "sealed_by",
                "word_locks"
            ]
        );
    }

    #[test]
    fn format_unknown_schema_names_the_sealing_engine() {
        let raw = include_bytes!("../../../fixtures/schema/unknown-schema.json");
        let err = read_sealed_day(raw).unwrap_err();
        assert_eq!(
            err,
            FormatError::UnsupportedSchema {
                found: 2,
                sealed_by: "advent 9.0.0".into()
            }
        );
        assert!(err.to_string().contains("advent 9.0.0"));
    }

    #[test]
    fn format_missing_schema_is_a_parse_error() {
        let err = read_sealed_final(br#"{"sealed_by":"advent 0.1.0"}"#).unwrap_err();
        assert!(matches!(err, FormatError::Parse(_)));
    }

    #[test]
    fn format_malformed_base64_is_a_parse_error() {
        let mut value = serde_json::to_value(sample_day()).unwrap();
        value["day_key"] = "not base64!".into();
        let err = read_sealed_day(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(matches!(err, FormatError::Parse(_)));
    }

    #[test]
    fn format_wrong_iv_length_is_a_parse_error() {
        let mut value = serde_json::to_value(sample_day()).unwrap();
        value["bundle"]["iv"] = "AAAA".into();
        let err = read_sealed_day(&serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert!(matches!(err, FormatError::Parse(_)));
    }

    #[test]
    fn format_bundle_round_trips_assets() {
        let bundle = Bundle {
            title: "Door five".into(),
            kind: "image".into(),
            markdown: "![map](map.webp)".into(),
            assets: BTreeMap::from([("map.webp".into(), vec![0, 255, 1])]),
        };
        let json = serde_json::to_string(&bundle).unwrap();
        assert!(json.contains(r#""type":"image""#));
        assert_eq!(serde_json::from_str::<Bundle>(&json).unwrap(), bundle);
    }
}
