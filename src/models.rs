use std::fs::read_to_string;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use thiserror::Error;

pub(crate) type Result<T> = std::result::Result<T, ScancodeError>;

/// License from [ScanCode](https://github.com/nexB/scancode-toolkit).
#[derive(Serialize, Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ScancodeLicense {
    /// Mandatory unique key: lower case ASCII characters, digits, underscore and dots.
    pub key: String,

    /// Commonly used short name, often abbreviated.
    pub short_name: String,

    /// Full name.
    pub name: String,

    /// Permissive, Copyleft, etc.
    pub category: Category,

    pub owner: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub homepage_url: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    #[serde(
        deserialize_with = "bool_from_yes",
        skip_serializing_if = "is_false",
        serialize_with = "yes_from_bool"
    )]
    #[serde(default)]
    pub is_deprecated: bool,

    /// SPDX key for SPDX licenses
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spdx_license_key: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_urls: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub osi_url: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub osi_license_key: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faq_url: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_urls: Vec<String>,

    #[serde(
        deserialize_with = "bool_from_yes",
        skip_serializing_if = "is_false",
        serialize_with = "yes_from_bool"
    )]
    #[serde(default)]
    pub is_exception: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub other_spdx_license_keys: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignorable_copyrights: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignorable_holders: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignorable_authors: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignorable_urls: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ignorable_emails: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_coverage: Option<i32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard_notice: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,

    #[serde(skip)]
    pub text: String,
}

impl ScancodeLicense {
    /// Deserialize [Self] from YAML files.
    ///
    /// Scancode stores the license information in two files, one for the metadata and one for the
    /// license text. This expects the file for the license text to exist in the same directory as
    /// the metadata file with same filename but `LICENSE` extension.
    pub(crate) fn from_yaml_file<P: AsRef<Path>>(yaml_path: P) -> Result<Self> {
        let license_string =
            read_to_string(yaml_path.as_ref()).map_err(|err| ScancodeError::Io {
                source: err,
                path: yaml_path.as_ref().to_path_buf(),
            })?;
        let mut license = serde_yaml::from_str::<Self>(&license_string).map_err(|err| {
            ScancodeError::SerdeYaml {
                source: err,
                path: yaml_path.as_ref().to_path_buf(),
            }
        })?;
        let text_path = yaml_path.as_ref().with_extension("LICENSE");
        let license_text = read_to_string(&text_path).map_err(|err| ScancodeError::Io {
            source: err,
            path: text_path,
        });
        license.text = match license_text {
            Ok(value) => value,
            Err(_) => "".into(),
        };

        Ok(license)
    }
}

/// Different license cateogires from ScanCode.
#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Category {
    Copyleft,
    #[serde(rename = "Copyleft Limited")]
    CopyleftLimited,
    #[serde(rename = "Patent License")]
    PatentLicense,
    Permissive,
    #[serde(rename = "Public Domain")]
    PublicDomain,
    Commercial,
    #[serde(rename = "Proprietary Free")]
    ProprietaryFree,
    #[serde(rename = "Free Restricted")]
    FreeRestricted,
    #[serde(rename = "Source-available")]
    SourceAvailable,
    #[serde(rename = "Unstated License")]
    UnstatedLicense,
}

/// Deserialize string "yes" as boolean true.
fn bool_from_yes<'de, D>(deserializer: D) -> std::result::Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "yes" => Ok(true),
        _ => Ok(false),
    }
}

/// Serialize a boolean true as "yes".
#[allow(clippy::trivially_copy_pass_by_ref)]
fn yes_from_bool<S>(boolean: &bool, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match boolean {
        true => s.serialize_str("yes"),
        false => s.serialize_str("no"),
    }
}

/// Check if false.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(boolean: &bool) -> bool {
    boolean == &false
}

#[cfg(test)]
mod tests {
    use std::{fs::read_to_string, path::PathBuf};

    use pretty_assertions::assert_eq;
    use yaml_rust::YamlLoader;

    use crate::import::{from_git_database, from_scancode_database};

    use super::*;

    fn test_data_directory() -> PathBuf {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        directory.join("tests/data")
    }

    #[test]
    fn deseralize_test_licenses() -> Result<()> {
        let paths = std::fs::read_dir(test_data_directory().join("licenses")).unwrap();

        for path in paths {
            let path = path.unwrap();
            if path.path().extension().unwrap() == "yml" {
                ScancodeLicense::from_yaml_file(&path.path())?;
            }
        }
        Ok(())
    }

    #[test]
    fn serialize_test_licenses_immutably() -> Result<()> {
        let paths = std::fs::read_dir(test_data_directory().join("licenses")).unwrap();

        for path in paths {
            let path = path.unwrap();
            if path.path().extension().unwrap() == "yml" {
                let license_string = read_to_string(path.path()).unwrap();
                let license_yaml = YamlLoader::load_from_str(&license_string).unwrap();
                let license = ScancodeLicense::from_yaml_file(&path.path())?;
                let serialized_yaml =
                    YamlLoader::load_from_str(&serde_yaml::to_string(&license).unwrap()).unwrap();
                assert_eq!(license_yaml, serialized_yaml);
            }
        }

        Ok(())
    }

    #[test]
    fn deserialize_correct_values() {
        let test_licenses_dir = test_data_directory().join("licenses");
        let actual_license: ScancodeLicense =
            serde_yaml::from_str(&read_to_string(test_licenses_dir.join("gpl-3.0.yml")).unwrap())
                .unwrap();

        let expected_license = ScancodeLicense {
            key: "gpl-3.0".into(),
            short_name: "GPL 3.0".into(),
            name: "GNU General Public License 3.0".into(),
            category: Category::Copyleft,
            owner: "Free Software Foundation (FSF)".into(),
            homepage_url: Some("http://www.gnu.org/licenses/gpl-3.0.html".into()),
            notes: Some("notes from SPDX:\nThis license was released: 29 June 2007 This license is OSI certified.".into()),
            spdx_license_key: Some("GPL-3.0".into()),
            text_urls: vec![
                "http://www.gnu.org/licenses/gpl-3.0.txt".into(),
                "http://www.gnu.org/licenses/gpl-3.0-standalone.html".into()],
            osi_url: Some("http://opensource.org/licenses/gpl-3.0.html".into()),
            faq_url: Some("http://www.gnu.org/licenses/gpl-faq.html".into()),
            other_urls: vec![
                "http://www.gnu.org/licenses/quick-guide-gplv3.html".into()],
            is_exception: false,
            is_deprecated: false,
            minimum_coverage: None,
            ignorable_copyrights: Vec::new(),
            ignorable_holders: Vec::new(),
            ignorable_authors: Vec::new(),
            ignorable_urls: Vec::new(),
            ignorable_emails: Vec::new(),
            other_spdx_license_keys: Vec::new(),
            standard_notice: None,
            osi_license_key: None,
            language: None,
            text: "".to_string()
        };

        assert_eq!(actual_license, expected_license);

        let actual_exception: ScancodeLicense = serde_yaml::from_str(
            &read_to_string(test_licenses_dir.join("gpl-2.0-library.yml")).unwrap(),
        )
        .unwrap();

        let expected_exception = ScancodeLicense {
            key: "gpl-2.0-library".into(),
            short_name: "GPL 2.0 with Library exception".into(),
            name: "GNU General Public License 2.0 with Library exception".into(),
            category: Category::CopyleftLimited,
            owner: "Grammatica".into(),
            homepage_url: None,
            notes: None,
            spdx_license_key: None,
            text_urls: Vec::new(),
            osi_url: None,
            faq_url: None,
            other_urls: vec!["http://grammatica.percederberg.net/index.html".into()],
            is_exception: true,
            is_deprecated: false,
            minimum_coverage: None,
            ignorable_copyrights: Vec::new(),
            ignorable_holders: Vec::new(),
            ignorable_authors: Vec::new(),
            ignorable_urls: Vec::new(),
            ignorable_emails: Vec::new(),
            other_spdx_license_keys: Vec::new(),
            standard_notice: None,
            osi_license_key: None,
            language: None,
            text: "".to_string(),
        };

        assert_eq!(actual_exception, expected_exception);
    }

    #[test]
    fn deserialize_all_licenses_from_scancode_licensedb() -> Result<()> {
        let licenses = from_scancode_database()?;

        assert!(licenses.len() > 1000);

        Ok(())
    }

    #[test]
    #[ignore = "currently contains invalid yaml"]
    fn deserialize_all_licenses_from_scancode_source() -> Result<()> {
        let licenses = from_git_database(
            "https://github.com/nexB/scancode-toolkit.git",
            "src/licensedcode/data/licenses",
        )?;

        assert!(licenses.len() > 1000);

        Ok(())
    }
}

/// Error while interacting with ScanCode items.
#[derive(Debug, Error)]
pub enum ScancodeError {
    #[error("Error with path {path:?}.")]
    Io {
        source: std::io::Error,
        path: PathBuf,
    },

    #[error("Error with IO.")]
    OtherIo(#[from] std::io::Error),

    #[error("Error with git.")]
    Git(#[from] git2::Error),

    #[error("SerdeYaml error with path {path:?}.")]
    SerdeYaml {
        source: serde_yaml::Error,
        path: PathBuf,
    },

    #[error("Error with serde_yaml.")]
    OtherSerdeYaml(#[from] serde_yaml::Error),
}
