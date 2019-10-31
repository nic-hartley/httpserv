use std::{
  env,
  collections::HashMap,
  fmt,
  ffi::OsString,
  path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum ArgFail {
  InvalidFormat(String),
}

impl fmt::Display for ArgFail {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      ArgFail::InvalidFormat(s) => {
        write!(f, "'{}' is incorrectly formatted", s)
      }
    }
  }
}

#[derive(Debug)]
pub struct Config {
  pub root: PathBuf,
  pub hostname: String,
  pub mappings: HashMap<OsString, String>,
}

impl Config {
  pub fn from_env() -> Result<Config, ArgFail> {
    let mut args = env::args().skip(1);
    let root = Path::new(&args.next().unwrap_or(".".into())).to_path_buf();
    let hostname = args.next().unwrap_or("localhost:8080".into());
    let mut mappings = HashMap::new();
    mappings.insert("html".into(), "text/html".into());
    mappings.insert("css".into(), "text/css".into());
    mappings.insert("js".into(), "text/javascript".into());
    mappings.insert("png".into(), "image/png".into());
    mappings.insert("jpg".into(), "image/jpeg".into());
    mappings.insert("jpeg".into(), "image/jpeg".into());
    mappings.insert("ico".into(), "image/vnd.microsoft.icon".into());
    mappings.insert("svg".into(), "image/svg+xml".into());
    for pair in args {
      let eq_idx = pair
        .find("=")
        .ok_or_else(|| ArgFail::InvalidFormat(pair.clone()))?;
      let (ext, mime) = pair.split_at(eq_idx);
      mappings.insert(ext.into(), mime.into());
    }

    Ok(Config {
      root,
      hostname,
      mappings,
    })
  }
}
