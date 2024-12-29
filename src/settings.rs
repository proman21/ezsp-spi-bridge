use anyhow::Result;
use config::{builder::DefaultState, ConfigBuilder, Environment, File};
use gpiod::LineId;
use serde::{de::Visitor, Deserialize, Deserializer};
use spidev::Spidev;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    str::FromStr,
};
use tracing::Level;

const LOG_LEVELS: [&'static str; 5] = ["DEBUG", "ERROR", "INFO", "TRACE", "WARN"];

struct LevelVistor;

impl<'de> Visitor<'de> for LevelVistor {
    type Value = Level;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .write_str("Expecting a number 1-5 or ")
            .and(formatter.write_str(&LOG_LEVELS.join(",")))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        FromStr::from_str(v).map_err(|_| E::unknown_variant(v, &LOG_LEVELS))
    }
}

pub fn deserialize_level<'de, D>(de: D) -> Result<Level, D::Error>
where
    D: Deserializer<'de>,
{
    de.deserialize_string(LevelVistor)
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Spi {
    pub device: PathBuf,
    pub gpiochip: PathBuf,
    pub cs_line: LineId,
    pub int_line: LineId,
    pub reset_line: LineId,
    pub wake_line: LineId,
}

#[derive(Debug, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub address: IpAddr,
    pub port: u16,
    pub spi: Spi,
    #[serde(deserialize_with = "deserialize_level")]
    pub loglevel: Level,
}

impl Settings {
    pub fn new() -> Result<Settings> {
        let reader = ConfigBuilder::<DefaultState>::default()
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::default())
            .build()?;

        Ok(reader.try_deserialize()?)
    }

    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }

    pub async fn spi_device(&self) -> Result<Spidev> {
        Ok(Spidev::open(&self.spi.device)?)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            address: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: 5555,
            spi: Default::default(),
            loglevel: Level::INFO,
        }
    }
}

impl Default for Spi {
    fn default() -> Self {
        Spi {
            device: PathBuf::from("/dev/spidev1.0"),
            gpiochip: PathBuf::from("/dev/gpiochip0"),
            cs_line: 45,
            int_line: 2,
            reset_line: 43,
            wake_line: 48,
        }
    }
}
