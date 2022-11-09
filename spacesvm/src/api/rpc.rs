use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
struct Request {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    id: serde_json::Value,
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_request() {}
}
