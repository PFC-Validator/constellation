/// Convert a f64 number (which is sent as a string) into a f64 rust structure
pub mod b64_format {
    use serde::{self, Deserialize, Deserializer, Serializer};

    // convert a number in string format into a regular u64
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    #[allow(missing_docs)]
    pub fn serialize<S>(val: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        //  let s = format!("{}", val);
        let encoded = base64::encode(val);
        serializer.serialize_str(&encoded)
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    #[allow(missing_docs)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = String::deserialize(deserializer)?;

        match base64::decode(&s) {
            Err(_e) => {
                eprintln!("f64 Fail {} {:#?}", s, _e);
                Err(serde::de::Error::custom(_e))
            }
            Ok(val) => Ok(String::from_utf8_lossy(&val).into()),
        }
    }
}
/// Convert a f64 number (which is sent as a string) into a f64 rust structure
pub mod b64_o_format {
    use serde::{self, Deserialize, Deserializer, Serializer};

    // convert a number in string format into a regular u64
    //
    //    fn serialize<S>(&T, S) -> Result<S::Ok, S::Error>
    //    where
    //        S: Serializer
    //
    // although it may also be generic over the input types T.
    #[allow(missing_docs)]
    pub fn serialize<S>(v: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        //  let s = format!("{}", val);
        if let Some(val) = v {
            let encoded = base64::encode(val);
            serializer.serialize_str(&encoded)
        } else {
            serializer.serialize_none()
        }
    }

    // The signature of a deserialize_with function must follow the pattern:
    //
    //    fn deserialize<'de, D>(D) -> Result<T, D::Error>
    //    where
    //        D: Deserializer<'de>
    //
    // although it may also be generic over the output types T.
    #[allow(missing_docs)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // let s: String = String::deserialize(deserializer)?;
        match String::deserialize(deserializer) {
            Ok(s) => {
                if s.is_empty() {
                    Ok(None)
                } else {
                    match base64::decode(&s) {
                        Err(e) => {
                            log::error!("Base64-opt Fail {} {:#?}", s, e);
                            Err(serde::de::Error::custom(e))
                        }
                        Ok(val) => Ok(Some(String::from_utf8_lossy(&val).into())),
                    }
                }
            }
            Err(e) => {
                log::error!("Deserializer {}", e);
                Ok(None)
            }
        }
    }
}
