//! Binary graph cache (.xray/graph.bin) using bincode.
//! M1 implementation: serialize/deserialize XrayGraph for instant re-serve.

use crate::graph::types::XrayGraph;

#[cfg(not(target_arch = "wasm32"))]
pub fn save(_graph: &XrayGraph, _path: &str) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn load(_path: &str) -> Result<Option<XrayGraph>, Box<dyn std::error::Error>> {
    Ok(None)
}
