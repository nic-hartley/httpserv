use std::{collections::HashMap, fmt, path::Path, time::Instant};

use httpserv::*;

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

fn get_cfg(args: impl Iterator<Item = String>) -> Result<Config, ArgFail> {
  let mut args = args.skip(1);
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
    let (_, mime) = mime.split_at(1);
    mappings.insert(ext.into(), mime.into());
  }

  Ok(Config {
    root,
    hostname,
    mappings,
    log: true,
  })
}

fn main() {
  let load_start = Instant::now();

  let cfg = match get_cfg(std::env::args()) {
    Ok(c) => c,
    Err(e) => {
      println!("Failed to get config: {:?}", e);
      return;
    }
  };

  let mut server = match Httpserv::new(cfg) {
    Ok(s) => s,
    Err(e) => {
      println!("Failed to launch server: {:?}", e);
      return;
    }
  };

  println!(
    "Launched in {}us; listening on {}; serving from {}",
    (Instant::now() - load_start).as_micros(),
    server.config().hostname,
    server.config().root.display()
  );

  server.run();
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::ffi::OsStr;

  #[test]
  fn with_no_args() {
    let cfg = get_cfg(vec!["".into()].into_iter());
    if let Ok(cfg) = cfg {
      assert_eq!(
        cfg.root,
        Path::new(".").to_path_buf(),
        "default root not cwd"
      );
      assert_eq!(
        cfg.hostname, "localhost:8080",
        "default hostname not localhost:8080"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("html")),
        Some(&"text/html".into()),
        "mappings[html] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("css")),
        Some(&"text/css".into()),
        "mappings[css] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("js")),
        Some(&"text/javascript".into()),
        "mappings[js] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("png")),
        Some(&"image/png".into()),
        "mappings[png] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("jpg")),
        Some(&"image/jpeg".into()),
        "mappings[jpg] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("jpeg")),
        Some(&"image/jpeg".into()),
        "mappings[jpeg] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("ico")),
        Some(&"image/vnd.microsoft.icon".into()),
        "mappings[ico] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("svg")),
        Some(&"image/svg+xml".into()),
        "mappings[svg] is wrong"
      );
      assert!(cfg.log, "not logging timings by default");
    } else {
      assert!(false, "Getting config returned error");
    }
  }

  #[test]
  fn given_root() {
    let cfg = get_cfg(vec!["", "foo/bar/../baz"].into_iter().map(Into::into));
    if let Ok(cfg) = cfg {
      assert_eq!(
        cfg.root,
        Path::new("foo/bar/../baz").to_path_buf(),
        "given root doesn't match"
      );
    } else {
      assert!(false, "Getting config returned error");
    }
  }

  #[test]
  fn given_hostname() {
    let cfg = get_cfg(vec!["", "", "laksdla:12313"].into_iter().map(Into::into));
    if let Ok(cfg) = cfg {
      assert_eq!(
        cfg.hostname, "laksdla:12313",
        "given hostname doesn't match"
      );
    } else {
      assert!(false, "Getting config returned error");
    }
  }

  #[test]
  fn given_mappings() {
    let cfg = get_cfg(vec!["", "", "", "a=b", "c=d"].into_iter().map(Into::into));
    if let Ok(cfg) = cfg {
      assert_eq!(
        cfg.mappings.get(OsStr::new("html")),
        Some(&"text/html".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("css")),
        Some(&"text/css".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("js")),
        Some(&"text/javascript".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("png")),
        Some(&"image/png".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("jpg")),
        Some(&"image/jpeg".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("jpeg")),
        Some(&"image/jpeg".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("ico")),
        Some(&"image/vnd.microsoft.icon".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("svg")),
        Some(&"image/svg+xml".into()),
        "Old mapping was changed"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("a")),
        Some(&"b".into()),
        "mappings[a] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("c")),
        Some(&"d".into()),
        "mappings[c] is wrong"
      );
    } else {
      assert!(false, "Getting config returned error");
    }
  }

  #[test]
  fn overwritten_mappings() {
    let cfg = get_cfg(
      vec!["", "", "", "ico=foobar", "jpg=baz"]
        .into_iter()
        .map(Into::into),
    );
    if let Ok(cfg) = cfg {
      assert_eq!(
        cfg.mappings.get(OsStr::new("ico")),
        Some(&"foobar".into()),
        "overwritten mappings[ico] is wrong"
      );
      assert_eq!(
        cfg.mappings.get(OsStr::new("jpg")),
        Some(&"baz".into()),
        "overwritten mappings[jpg] is wrong"
      );
    } else {
      assert!(false, "Getting config returned error");
    }
  }
}
