use std::borrow::Cow;

use pad::{Alignment, PadStr};
use toml;

use cargo_edit::Manifest;
use cargo_edit::fetch::get_latest_version;
use list_error::ListError;

struct Row<'a> {
    name:     &'a String,
    version:  String,
    optional: bool,
    latest:   Option<String>,
}

/// List the dependencies for manifest section
#[allow(deprecated)] // connect -> join
pub fn list_section(manifest: &Manifest, section: &str, show_latest: bool) -> Result<String, ListError> {
    let mut output = vec![];

    let list = try!(manifest.data
        .get(section)
        .and_then(|field| field.as_table())
        .ok_or_else(|| ListError::SectionMissing(String::from(section))));

    let rows = try!(list.iter().map(|(name, val)| Ok(Row {
        name:    name,
        version: match *val {
            toml::Value::String(ref version) => version.clone(),
            toml::Value::Table(_) => {
                try!(val.lookup("version")
                    .and_then(|field| field.as_str().map(|s| s.to_owned()))
                    .or_else(|| val.lookup("git").map(|repo| format!("git: {}", repo)))
                    .or_else(|| val.lookup("path").map(|path| format!("path: {}", path)))
                    .ok_or_else(|| ListError::VersionMissing(name.clone(), section.to_owned())))
            }
            _ => String::from(""),
        },
        optional: if let toml::Value::Table(_) = *val {
            val.lookup("optional")
                .and_then(|field| field.as_bool())
                .unwrap_or(false)
        } else {
            false
        },
        latest: if show_latest {
            get_latest_version(name).map_err(|e| ListError::FetchVersionError{ err: e, package: name.to_owned() }).ok()
        } else {
            None
        }
    })).collect::<Result<Vec<_>, _>>());

    let name_max_len    = rows.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let version_max_len = rows.iter().map(|r| r.version.len()).max().unwrap_or(0);
    let has_optional    = rows.iter().any(|r| r.optional);

    for row in rows {
        output.push(format!("{name} {version}{optional} {latest}",
                            name =
                                row.name.pad_to_width_with_alignment(name_max_len, Alignment::Left),
                            version = row.version.pad_to_width_with_alignment(version_max_len, Alignment::Left),
                            optional = if row.optional {
                                " (optional) "
                            } else if has_optional {
                                "            "
                            } else {
                                ""
                            },
                            latest = row.latest.map(Cow::Owned).unwrap_or(Cow::Borrowed(""))));
    }

    Ok(output.connect("\n"))
}

#[cfg(test)]
mod test {
    use cargo_edit::Manifest;
    use super::list_section;

    static DEFAULT_CARGO_TOML: &'static str = r#"[package]
authors = ["Some Guy"]
name = "lorem-ipsum"
version = "0.1.0"

[dependencies]
foo-bar = "0.1"
lorem-ipsum = "0.4.2""#;

    #[test]
    fn basic_listing() {
        let manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

        assert_eq!(list_section(&manifile, "dependencies").unwrap(),
                   "\
foo-bar     0.1
lorem-ipsum 0.4.2");
    }

    #[test]
    #[should_panic]
    fn unknown_section() {
        let manifile: Manifest = DEFAULT_CARGO_TOML.parse().unwrap();

        list_section(&manifile, "lol-dependencies").unwrap();
    }
}
