use std::fs::read_to_string;
use std::{
    fs::{DirBuilder, File},
    path::Path,
};

use miette::{Context, IntoDiagnostic};

use node_provider::*;

fn main() -> miette::Result<()> {
    for zone in ["zone", "zone-postgres"] {
        convert_zone_to_yaml(zone)?;
    }

    Ok(())
}

fn convert_zone_to_yaml(name: &str) -> miette::Result<()> {
    let zone = parse_zone(&format!("sample_data/{name}.kdl"))?;

    if !Path::new("sample_data").exists() {
        DirBuilder::new().create("sample_data").into_diagnostic()?;
    }

    let mut f = File::create(format!("sample_data/{name}.yaml")).into_diagnostic()?;

    serde_yaml::to_writer(&mut f, &zone).into_diagnostic()
}

fn parse_zone(path: &str) -> miette::Result<Zone> {
    let text = read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot read {:?}", path))?;
    Ok(knuffel::parse(path, &text)?)
}
