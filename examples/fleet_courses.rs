//! Export routes from the exact city connector geometry used by production.
use car_game::city::curve;
use serde_json::json;
use sha2::{Digest, Sha256};
fn main() {
    let routes = json!({
        "curve_forward":curve::route(false), "curve_reverse":curve::route(true),
        "center":curve::CENTER.to_array(), "radius_m":curve::RADIUS,"width_m":curve::WIDTH,
        "source":"src/city/curve.rs"
        ,"source_sha256":format!("{:x}",Sha256::digest(include_bytes!("../src/city/curve.rs")))
    });
    println!("{}", serde_json::to_string_pretty(&routes).unwrap());
}
