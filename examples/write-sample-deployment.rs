use std::{
    fs::{DirBuilder, File},
    path::Path,
};

use chrono::Utc;
use cloud::{Deployment, ResourceIdentifier};
use miette::IntoDiagnostic;
use semver::Version;

fn main() -> miette::Result<()> {
    let identifier = ResourceIdentifier::new(
        Some("wegmueller.it".to_owned()),
        "zones/testzone".to_owned(),
        Version::new(0, 1, 0),
    );
    let depl = Deployment {
        resource_identifier: identifier,
        created_at: Utc::now().naive_utc(),
        updated_at: None,
        resources: vec![],
        files: vec![],
        selector: cloud::Selector::Node {
            name: "testnode".to_owned(),
        },
    };

    if !Path::new("sample_data").exists() {
        DirBuilder::new().create("sample_data").into_diagnostic()?;
    }

    let mut f = File::create("sample_data/empty-deployment.yaml").into_diagnostic()?;

    serde_yaml::to_writer(&mut f, &depl).into_diagnostic()?;

    Ok(())
}
