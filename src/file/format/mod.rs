use source::Source;
use value::Value;
use std::error::Error;
use std::collections::HashMap;

#[cfg(feature = "toml")]
mod toml;

#[cfg(feature = "yaml")]
mod yaml;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum FileFormat {
    /// TOML (parsed with toml)
    #[cfg(feature = "toml")]
    Toml,

    /// YAML (parsed with yaml_rust)
    #[cfg(feature = "yaml")]
    Yaml,
}

lazy_static! {
    #[doc(hidden)]
    pub static ref ALL_EXTENSIONS: HashMap<FileFormat, Vec<&'static str>> = {
        let mut formats: HashMap<FileFormat, Vec<_>> = HashMap::new();

        #[cfg(feature = "toml")]
        formats.insert(FileFormat::Toml, vec!["toml"]);

        #[cfg(feature = "json")]
        formats.insert(FileFormat::Json, vec!["json"]);

        #[cfg(feature = "yaml")]
        formats.insert(FileFormat::Yaml, vec!["yaml", "yml"]);

        formats
    };
}

impl FileFormat {
    // TODO: pub(crate)
    #[doc(hidden)]
    pub fn extensions(&self) -> &'static Vec<&'static str> {
        ALL_EXTENSIONS.get(self).unwrap()
    }

    // TODO: pub(crate)
    #[doc(hidden)]
    #[allow(unused_variables)]
    pub fn parse(&self,
                 uri: Option<&String>,
                 text: &str)
                 -> Result<HashMap<String, Value>, Box<Error>> {
        match *self {
            #[cfg(feature = "toml")]
            FileFormat::Toml => toml::parse(uri, text),

            #[cfg(feature = "yaml")]
            FileFormat::Yaml => yaml::parse(uri, text),
        }
    }
}
