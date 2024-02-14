use std::{fmt::Display, str::FromStr};

use anyhow::anyhow;

#[derive(Clone, Debug, PartialEq)]
pub enum SudoPayAsset {
    Usdb,
    Eth,
    Weth,
}

impl FromStr for SudoPayAsset {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "USDB" => Ok(SudoPayAsset::Usdb),
            "ETH" => Ok(SudoPayAsset::Eth),
            "WETH" => Ok(SudoPayAsset::Weth),
            _ => Err(anyhow!("Invalid SudoPayAsset")),
        }
    }
}

impl Display for SudoPayAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SudoPayAsset::Usdb => write!(f, "USDB"),
            SudoPayAsset::Eth => write!(f, "ETH"),
            SudoPayAsset::Weth => write!(f, "WETH"),
        }
    }
}
