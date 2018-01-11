use std::collections::HashMap;
use std::ops::Deref;
use std::str::FromStr;
use std::fmt::{Display, Debug, Formatter, Result as FmtResult};
use serde::de::Deserialize;

use error::*;
use source::Source;

use value::{Value, ValueKind, ValueWithKey};
use path;

#[derive(Clone, Debug)]
enum ConfigKind {
    // A mutable configuration. This is the default.
    Mutable {
        defaults: HashMap<path::Expression, Value>,
        overrides: HashMap<path::Expression, Value>,
        sources: Vec<Box<Source + Send + Sync>>,
    },

    // A frozen configuration.
    // Configuration can no longer be mutated.
    Frozen,
}

impl Default for ConfigKind {
    fn default() -> Self {
        ConfigKind::Mutable {
            defaults: HashMap::new(),
            overrides: HashMap::new(),
            sources: Vec::new(),
        }
    }
}

/// A prioritized configuration repository. It maintains a set of
/// configuration sources, fetches values to populate those, and provides
/// them according to the source's priority.
#[derive(Default, Clone, Debug)]
pub struct Config {
    kind: ConfigKind,

    /// Root of the cached configuration.
    pub cache: Value,
}

impl From<HashMap<String, Value>> for Config {
    fn from(map: HashMap<String, Value>) -> Config {
        fn build_path(root: String, valmap: &HashMap<String, Value>,
                      retmap: &mut HashMap<path::Expression, Value>) {
            for (k, v) in valmap {
                match v.kind {
                    ValueKind::Table(ref t) => {
                        let root_path = if root == "".to_string() {
                            format!("")
                        } else {
                            format!("{}.", root)
                        };
                        build_path(format!("{}{}", root_path, k), &t, retmap)
                    },
                    ValueKind::Array(ref a) => {
                        let mut i = 0;
                        for elem in a {
                            retmap.insert(path::Expression::from_str(
                                format!("{}[{}]", k, i).as_ref()).unwrap(), elem.clone());
                            i += 1;
                        }
                    },
                    _ => {
                        let root_path = if root == "".to_string() {
                            format!("")
                        } else {
                            format!("{}.", root)
                        };
                        retmap.insert(path::Expression::from_str(
                            format!("{}{}", root_path, k).as_ref()).unwrap(), v.clone());
                    },
                }
            }
        }
        let mut retmap = HashMap::new();
        build_path("".to_string(), &map, &mut retmap);
        Config {
            kind: ConfigKind::Mutable {
                defaults: HashMap::new(),
                overrides: retmap,
                sources: Vec::new(),
            },
            cache: map.into(),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Config::default()
    }

    /// Merge in a configuration property source.
    pub fn merge<T>(&mut self, source: T) -> ConfigResult
        where T: 'static,
              T: Source + Send + Sync
    {
        match self.kind {
            ConfigKind::Mutable { ref mut sources, .. } => {
                sources.push(Box::new(source));
            }

            ConfigKind::Frozen => {
                return ConfigResult(Err(ConfigError::Frozen));
            }
        }

        self.refresh()
    }

    /// Refresh the configuration cache with fresh
    /// data from added sources.
    ///
    /// Configuration is automatically refreshed after a mutation
    /// operation (`set`, `merge`, `set_default`, etc.).
    pub fn refresh(&mut self) -> ConfigResult {
        self.cache = match self.kind {
            // TODO: We need to actually merge in all the stuff
            ConfigKind::Mutable {
                ref overrides,
                ref sources,
                ref defaults,
            } => {
                let mut cache: Value = HashMap::<String, Value>::new().into();

                // Add defaults
                for (key, val) in defaults {
                    key.set(&mut cache, val.clone());
                }

                // Add sources
                if let Err(error) = sources.collect_to(&mut cache) {
                    return ConfigResult(Err(error));
                }

                // Add overrides
                for (key, val) in overrides {
                    key.set(&mut cache, val.clone());
                }

                cache
            }

            ConfigKind::Frozen => {
                return ConfigResult(Err(ConfigError::Frozen));
            }
        };

        ConfigResult(Ok(self))
    }

    /// Deserialize the entire configuration.
    pub fn deserialize<'de, T: Deserialize<'de>>(&self) -> Result<T> {
        T::deserialize(self.cache.clone())
    }

    pub fn set_default<T>(&mut self, key: &str, value: T) -> ConfigResult
        where T: Into<Value>
    {
        match self.kind {
            ConfigKind::Mutable { ref mut defaults, .. } => {
                defaults.insert(match key.to_lowercase().parse() {
                                    Ok(expr) => expr,
                                    Err(error) => {
                                        return ConfigResult(Err(error));
                                    }
                                },
                                value.into());
            }

            ConfigKind::Frozen => return ConfigResult(Err(ConfigError::Frozen)),
        };

        self.refresh()
    }

    pub fn set<T>(&mut self, key: &str, value: T) -> ConfigResult
        where T: Into<Value>
    {
        match self.kind {
            ConfigKind::Mutable { ref mut overrides, .. } => {
                overrides.insert(match key.to_lowercase().parse() {
                                     Ok(expr) => expr,
                                     Err(error) => {
                                         return ConfigResult(Err(error));
                                     }
                                 },
                                 value.into());
            }

            ConfigKind::Frozen => return ConfigResult(Err(ConfigError::Frozen)),
        };

        self.refresh()
    }

    pub fn get<'de, T: Deserialize<'de>>(&self, key: &'de str) -> Result<T> {
        // Parse the key into a path expression
        let expr: path::Expression = key.to_lowercase().parse()?;

        // Traverse the cache using the path to (possibly) retrieve a value
        let value = expr.get(&self.cache).cloned();

        match value {
            Some(value) => {
                // Deserialize the received value into the requested type
                T::deserialize(ValueWithKey::new(value, key))
            }

            None => Err(ConfigError::NotFound(key.into())),
        }
    }

    pub fn get_str(&self, key: &str) -> Result<String> {
        self.get(key).and_then(Value::into_str)
    }

    pub fn get_int(&self, key: &str) -> Result<i64> {
        self.get(key).and_then(Value::into_int)
    }

    pub fn get_float(&self, key: &str) -> Result<f64> {
        self.get(key).and_then(Value::into_float)
    }

    pub fn get_bool(&self, key: &str) -> Result<bool> {
        self.get(key).and_then(Value::into_bool)
    }

    pub fn get_tree(&self, key: &str) -> Result<Config> {
        self.get(key).and_then(Value::into_tree)
    }

    pub fn get_array(&self, key: &str) -> Result<Vec<Value>> {
        self.get(key).and_then(Value::into_array)
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.write_fmt(format_args!("{}", self.cache.as_string()))
    }
}

pub struct ConfigResult<'a>(Result<&'a mut Config>);

#[inline]
fn unwrap_failed<E: Debug>(msg: &str, error: E) -> ! {
    panic!("{}: {:?}", msg, error)
}

impl<'a> ConfigResult<'a> {
    pub fn merge<T>(self, source: T) -> ConfigResult<'a>
        where T: 'static,
              T: Source + Send + Sync
    {
        match self.0 {
            // If OK, Proceed to nested method
            Ok(instance) => instance.merge(source),

            // Else, Forward the error
            error => ConfigResult(error),
        }
    }

    pub fn set_default<T>(self, key: &str, value: T) -> ConfigResult<'a>
        where T: Into<Value>,
              T: 'static
    {
        match self.0 {
            // If OK, Proceed to nested method
            Ok(instance) => instance.set_default(key, value),

            // Else, Forward the error
            error => ConfigResult(error),
        }
    }

    pub fn set<T>(self, key: &str, value: T) -> ConfigResult<'a>
        where T: Into<Value>,
              T: 'static
    {
        match self.0 {
            // If OK, Proceed to nested method
            Ok(instance) => instance.set(key, value),

            // Else, Forward the error
            error => ConfigResult(error),
        }
    }

    /// Forwards `Result::is_ok`
    #[inline]
    pub fn is_ok(&self) -> bool {
        match self.0 {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    /// Forwards `Result::is_err`
    #[inline]
    pub fn is_err(&self) -> bool {
        !self.is_ok()
    }

    /// Forwards `Result::ok`
    #[inline]
    pub fn ok(self) -> Option<Config> {
        match self.0 {
            Ok(x) => Some(x.clone()),
            Err(_) => None,
        }
    }

    /// Forwards `Result::err`
    #[inline]
    pub fn err(self) -> Option<ConfigError> {
        match self.0 {
            Ok(_) => None,
            Err(x) => Some(x),
        }
    }

    /// Forwards `Result::unwrap`
    #[inline]
    pub fn unwrap(self) -> Config {
        match self.0 {
            Ok(instance) => instance.clone(),
            Err(error) => unwrap_failed("called `Result::unwrap()` on an `Err` value", error),
        }
    }

    /// Forwards `Result::expect`
    #[inline]
    pub fn expect(self, msg: &str) -> Config {
        match self.0 {
            Ok(instance) => instance.clone(),
            Err(error) => unwrap_failed(msg, error),
        }
    }

    /// Forwards `Result::unwrap_err`
    #[inline]
    pub fn unwrap_err(self) -> ConfigError {
        match self.0 {
            Ok(t) => unwrap_failed("called `Result::unwrap_err()` on an `Ok` value", t),
            Err(e) => e,
        }
    }

    /// Forwards `Result::expect_err`
    #[inline]
    pub fn expect_err(self, msg: &str) -> ConfigError {
        match self.0 {
            Ok(t) => unwrap_failed(msg, t),
            Err(e) => e,
        }
    }
}
