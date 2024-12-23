//! Contains abstractions representing image artifacts referred by a Project.
use super::ArtifactVendor;
use crate::docker::ImageUri;
use anyhow::{ensure, Result};
use semver::Version;
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::hash::Hash;
use std::str::FromStr;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct ProjectImage {
    pub(super) image: Image,
    pub(super) vendor: ArtifactVendor,
}

impl Display for ProjectImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.vendor {
            ArtifactVendor::Overridden(_) => write!(
                f,
                "{}-{}@{} (overridden-to: {})",
                self.name(),
                self.version(),
                self.original_source_uri(),
                self.project_image_uri(),
            ),
            ArtifactVendor::Verbatim(_) => write!(
                f,
                "{}-{}@{}",
                self.name(),
                self.version(),
                self.original_source_uri()
            ),
        }
    }
}

impl ProjectImage {
    pub(crate) fn name(&self) -> &ValidIdentifier {
        &self.image.name
    }

    pub(crate) fn version(&self) -> &Version {
        self.image.version()
    }

    pub(crate) fn vendor_name(&self) -> &ValidIdentifier {
        self.vendor.vendor_name()
    }

    /// Returns the URI for the original vendor.
    pub(crate) fn original_source_uri(&self) -> ImageUri {
        match &self.vendor {
            ArtifactVendor::Overridden(overridden) => {
                let original = ArtifactVendor::Verbatim(overridden.original_vendor());
                original.image_uri_for(&self.image)
            }
            ArtifactVendor::Verbatim(_) => self.vendor.image_uri_for(&self.image),
        }
    }

    /// Returns the image URI that the project will use for this image
    ///
    /// This could be different than the source_uri if overridden.
    pub(crate) fn project_image_uri(&self) -> ImageUri {
        ImageUri {
            registry: Some(self.vendor.registry().to_string()),
            repo: self.vendor.repo_for(&self.image).to_string(),
            tag: format!("v{}", self.image.version()),
        }
    }
}

/// An artifact/vendor name combination used to identify an artifact resolved by Twoliter.
///
/// This is intended for use in [`Project::vendor_for`] lookups.
pub(crate) trait VendedArtifact: std::fmt::Debug {
    fn artifact_name(&self) -> &ValidIdentifier;
    fn vendor_name(&self) -> &ValidIdentifier;
    fn version(&self) -> &Version;
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub(crate) struct ValidIdentifier(pub(crate) String);

impl Serialize for ValidIdentifier {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.as_str())
    }
}

impl FromStr for ValidIdentifier {
    type Err = anyhow::Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        ensure!(
            !input.is_empty(),
            "cannot define an identifier as an empty string",
        );

        // Check if the input contains any invalid characters
        for c in input.chars() {
            ensure!(
                is_valid_id_char(c),
                "invalid character '{}' found in identifier name",
                c
            );
        }

        Ok(Self(input.to_string()))
    }
}

impl<'de> Deserialize<'de> for ValidIdentifier {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let input = String::deserialize(deserializer)?;
        input.parse().map_err(D::Error::custom)
    }
}

impl AsRef<str> for ValidIdentifier {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl Display for ValidIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

fn is_valid_id_char(c: char) -> bool {
    match c {
        // Allow alphanumeric characters, underscores, and hyphens
        'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' => true,
        // Disallow other characters
        _ => false,
    }
}

/// This represents a container registry vendor that is used in resolving the kits and also
/// now the bottlerocket sdk
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Vendor {
    pub registry: String,
}

/// This represents a dependency on a container, primarily used for kits
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[serde(rename_all = "kebab-case")]
pub(crate) struct Image {
    pub name: ValidIdentifier,
    pub version: Version,
    pub vendor: ValidIdentifier,
}

impl Image {
    pub(super) fn from_vended_artifact(artifact: &impl VendedArtifact) -> Self {
        Self {
            name: artifact.artifact_name().clone(),
            vendor: artifact.vendor_name().clone(),
            version: artifact.version().clone(),
        }
    }
}

impl Display for Image {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}@{}", self.name, self.version, self.vendor)
    }
}

impl VendedArtifact for Image {
    fn artifact_name(&self) -> &ValidIdentifier {
        &self.name
    }

    fn vendor_name(&self) -> &ValidIdentifier {
        &self.vendor
    }

    fn version(&self) -> &Version {
        &self.version
    }
}
