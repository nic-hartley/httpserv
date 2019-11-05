use std::{collections::HashMap, ffi::OsString, path::PathBuf};

#[derive(Debug)]
pub struct Config {
  pub root: PathBuf,
  pub hostname: String,
  pub mappings: HashMap<OsString, String>,
  pub log: bool,
}
