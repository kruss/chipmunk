use serde::{Deserialize, Serialize};

/// A generic, hierarchical representation of a parsed log message.
///
/// The tree models the semantic structure of a message independently of the
/// underlying protocol. A message node may contain nested message nodes,
/// allowing protocol stacks (e.g. DLT → SOME/IP → application payload) to be
/// represented naturally.
///
/// Each node may reference the corresponding byte and bit range of the original
/// message, enabling features such as structured inspection, byte selection,
/// and future protocol reinterpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredLogMessage {
    pub root: DetailNode,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailNode {
    pub name: String,
    pub role: NodeRole,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub byte_range: Option<ByteRange>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_range: Option<BitRange>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<DetailNode>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum NodeRole {
    /// A decoded protocol layer.
    Frame,
    /// Protocol header.
    Header,
    /// Protocol payload.
    Payload,
    /// A named field or value.
    Field,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    Bool(bool),
    UInt(u64),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Enum { raw: u64, name: String },
    BitMask { raw: u64, flags: Vec<Flag> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flag {
    pub name: String,
    pub set: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ByteRange {
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BitRange {
    pub offset: u8,
    pub length: u8,
}

impl StructuredLogMessage {
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str::<Self>(json).map_err(|err| err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_structured_log_message_from_json() {
        let json = r#"{
            "root": {
                "name": "DLT Message",
                "role": "Frame"
            }
        }"#;

        let message = StructuredLogMessage::from_json(json).unwrap();

        assert_eq!(message.root.name, "DLT Message");
        assert!(matches!(message.root.role, NodeRole::Frame));
        assert!(message.root.children.is_empty());
    }
}
