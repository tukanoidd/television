use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    ops::Deref,
};

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct CableChannelPrototype {
    pub name: String,
    pub source_command: String,
    pub preview_command: Option<String>,
    #[serde(default = "default_delimiter")]
    pub preview_delimiter: Option<String>,
}

pub const DEFAULT_DELIMITER: &str = " ";

fn default_delimiter() -> Option<String> {
    Some(DEFAULT_DELIMITER.to_string())
}

impl Display for CableChannelPrototype {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, serde::Deserialize, Default)]
pub struct CableChannels(pub HashMap<String, CableChannelPrototype>);

impl Deref for CableChannels {
    type Target = HashMap<String, CableChannelPrototype>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
