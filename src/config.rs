use anyhow::{Context as _, Result};
use log::debug;
use opendal::{Operator, Scheme};
use std::collections::HashMap;

const SCHEME_KEY: &str = "scheme";
const SECTION_NAME: &str = "lfs-dal";
const CONFIG_FILE: &str = ".lfsdalconfig";

pub fn remote_operator() -> Result<Operator> {
    let map0 = get_map(&gix_config::File::from_path_no_includes(
        CONFIG_FILE.into(),
        gix_config::Source::Local,
    )?);
    let map1 = get_map(&gix_config::File::from_git_dir(".git".into())?);
    let map: HashMap<String, String> = map0.into_iter().chain(map1).collect();
    debug!("opendal map: {:?}", map);

    let scheme_str = map
        .get(SCHEME_KEY)
        .with_context(|| format!("{} not found", SCHEME_KEY))?;
    let scheme = Scheme::enabled()
        .into_iter()
        .find(|s| &s.to_string() == scheme_str)
        .with_context(|| format!("invalid scheme {}", scheme_str))?;

    Ok(Operator::via_map(scheme, map)?)
}

fn get_map(f: &gix_config::File) -> HashMap<String, String> {
    let section = f.section_by_key(SECTION_NAME.into());
    if section.is_err() {
        return HashMap::new();
    }
    let section = section.unwrap();
    section
        .value_names()
        .map(|k| {
            (
                k.to_string().replace('-', "_"),
                section.value(k).unwrap().to_string(),
            )
        })
        .collect()
}
