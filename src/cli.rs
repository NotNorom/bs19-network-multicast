use std::num::NonZeroU16;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct Cli {
    /// universes
    pub universes: Vec<NonZeroU16>,
}
