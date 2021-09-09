use std::ffi::OsStr;
use std::fs::read_dir;
use std::path::Path;

use git2::Repository;
use tempfile::tempdir;

use crate::models::Result;
use crate::models::ScancodeError;
use crate::models::ScancodeLicense;

/// Get all licenses from the [ScanCode license database](https://github.com/nexB/scancode-licensedb).
pub fn from_scancode_database() -> Result<Vec<ScancodeLicense>> {
    let licenses = from_git_database("https://github.com/nexB/scancode-licensedb.git", "docs")?;

    Ok(licenses)
}

/// Get all licenses from a given license database. Specify the url to the repository and the
/// path where the license files are located, relative to the root of the repository.
pub fn from_git_database<P: AsRef<Path>>(
    url: &str,
    path_in_repo: P,
) -> Result<Vec<ScancodeLicense>> {
    let tempdir = tempdir()?;
    Repository::clone(url, tempdir.path())?;

    let licenses_path = tempdir.path().join(path_in_repo);
    let licenses_dir = read_dir(&licenses_path).map_err(|err| ScancodeError::Io {
        source: err,
        path: licenses_path,
    })?;

    let mut licenses: Vec<ScancodeLicense> = Vec::new();

    for path in licenses_dir {
        let path = path?;

        // The database contains 'index.yml' that doesn't include a license.
        if path.file_name() == "index.yml" {
            continue;
        }

        if path.path().extension() == Some(OsStr::new("yml")) {
            let license = ScancodeLicense::from_yaml_file(&path.path())?;
            licenses.push(license);
        }
    }

    Ok(licenses)
}
