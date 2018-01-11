extern crate config;
extern crate serde;
extern crate float_cmp;

#[macro_use]
extern crate serde_derive;

use std::collections::HashMap;
use float_cmp::ApproxEqUlps;
use config::*;

fn make() -> Config {
    let mut c = Config::default();
    c.merge(File::new("tests/Settings", FileFormat::Yaml))
        .unwrap();

    c
}

///
/// place:
///   name: Torre di Pisa
///   longitude: 43.7224985
///   latitude: 10.3970522
///   favorite: false
///   reviews: 3866
///   rating: 4.5
///   creators:
///     - name: John Smith
///       id: 12345
///     - name: Bob Dole
///       id: 67890
///
#[test]
fn test_get_tree() {
    let c = make();
    let mut place: Config = c.get_tree("place").unwrap();
    
    let mut creators_a = place.get_array("creators").unwrap();
    assert_eq!(creators_a.len(), 2);
    let creators1 = creators_a.remove(0).into_tree().unwrap();
    assert_eq!(creators1.get_str("name").unwrap(), "John Smith".to_string());

    place.refresh();
    
    let mut creators_b = place.get_array("creators").unwrap();
    assert_eq!(creators_b.len(), 2);
    let creators1 = creators_b.remove(0).into_tree().unwrap();
    assert_eq!(creators1.get_str("name").unwrap(), "John Smith".to_string());
}
