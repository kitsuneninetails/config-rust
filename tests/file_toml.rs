extern crate config;
extern crate serde;
extern crate float_cmp;

#[macro_use]
extern crate serde_derive;

use std::vec::Vec;
use float_cmp::ApproxEqUlps;
use config::*;

#[derive(Debug, Deserialize)]
struct Creator {
    name: Value,
    id: Value,
}

#[derive(Debug, Deserialize)]
struct Place {
    name: String,
    longitude: f64,
    latitude: f64,
    favorite: bool,
    telephone: Option<String>,
    reviews: u64,
    creators: Vec<Creator>,
    rating: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct Settings {
    debug: f64,
    production: Option<String>,
    place: Place,
    #[serde(rename = "arr")]
    elements: Vec<String>,
}

fn make() -> Config {
    let mut c = Config::default();
    c.merge(File::new("tests/Settings", FileFormat::Toml))
        .unwrap();

    c
}

#[test]
fn test_file() {
    let c = make();

    // Deserialize the entire file as single struct
    let s: Settings = c.deserialize().unwrap();

    assert!(s.debug.approx_eq_ulps(&1.0, 2));
    assert_eq!(s.production, Some("false".to_string()));
    assert_eq!(s.place.name, "Torre di Pisa");
    assert!(s.place.longitude.approx_eq_ulps(&43.7224985, 2));
    assert!(s.place.latitude.approx_eq_ulps(&10.3970522, 2));
    assert_eq!(s.place.favorite, false);
    assert_eq!(s.place.reviews, 3866);
    assert_eq!(s.place.rating, Some(4.5));
    assert_eq!(s.place.telephone, None);
    assert_eq!(s.elements.len(), 10);
    assert_eq!(s.elements[3], "4".to_string());
    assert_eq!(s.place.creators[0].name.clone().into_str().unwrap(), "John Smith".to_string());
    assert_eq!(s.place.creators[0].id.clone().into_str().unwrap(), "12345".to_string());
    assert_eq!(s.place.creators[1].name.clone().into_str().unwrap(), "Bob Dole".to_string());
}

#[test]
fn test_error_parse() {
    let mut c = Config::default();
    let res = c.merge(File::new("tests/Settings-invalid", FileFormat::Toml));

    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(),
               "invalid number at line 2 in tests/Settings-invalid.toml"
                   .to_string());
}
