use std::{
  fmt,
  fs::File,
  io::{self, BufRead, BufReader, BufWriter, Write as _},
  net,
  path::{Component, Path},
};

use crate::cfg;

#[derive(Debug)]
pub enum ReqFail {
  EarlyInputEnd,
  InvalidFormat(String),
  IOOpFailed(io::Error),
  Malicious(&'static str),
  InvalidPercentEncode,
}

impl fmt::Display for ReqFail {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match self {
      ReqFail::EarlyInputEnd => write!(f, "input ended earlier than expected"),
      ReqFail::InvalidFormat(s) => {
        write!(f, "'{}' is incorrectly formatted", s)
      }
      ReqFail::IOOpFailed(e) => {
        write!(f, "Failed to complete action because of {:?}", e)
      }
      ReqFail::Malicious(w) => write!(f, "Suspected maliocious request: {}", w),
      ReqFail::InvalidPercentEncode => write!(f, "Invalid percent encoding"),
    }
  }
}

fn percent_decode(mut inp: &str) -> Option<String> {
  let mut out = Vec::new();
  loop {
      let next_pct = match inp.find('%') {
          Some(l) if l < inp.len() - 2 => l,
          Some(_) => return None,
          None => break,
      };
      let (push, pct_rest) = inp.split_at(next_pct);
      out.extend_from_slice(push.as_bytes());
      let (pct, rest) = pct_rest.split_at(3);
      inp = rest;
      if pct == "%2F" {
        return None;
      }
      let val = u8::from_str_radix(&pct[1..], 16).ok()?;
      out.push(val);
  }
  out.extend_from_slice(inp.as_bytes());
  String::from_utf8(out).ok()
}

pub struct Request {
  pub path: String,
}

impl Request {
  pub fn load(conn: &mut net::TcpStream) -> Result<Request, ReqFail> {
    let mut input = BufReader::new(conn).lines();
    let path = {
      // parse "GET /url/here HTTP/1.1" to "url/here"
      let first_line = input
        .next()
        .ok_or(ReqFail::EarlyInputEnd)?
        .or(Err(ReqFail::EarlyInputEnd))?;
      // +1 because we actually care about the next character, not this one
      let mut url_start = first_line
        .find(" ")
        .ok_or_else(|| ReqFail::InvalidFormat(first_line.clone()))?
        + 1;
      if first_line.chars().nth(url_start) == Some('/') {
        url_start += 1;
      }
      let url_length = first_line[url_start..]
        .find(" ")
        .ok_or_else(|| ReqFail::InvalidFormat(first_line.clone()))?;
      let url = &first_line[url_start..url_start + url_length];
      let end = url
        .find('?')
        .or_else(|| url.find('#'))
        .unwrap_or_else(|| url.len());
      let url = &url[..end];
      let url_path = Path::new(url);
      if url_path.components().any(|c| c == Component::ParentDir) {
        return Err(ReqFail::Malicious(".. component in path"));
      }
      percent_decode(url).ok_or(ReqFail::InvalidPercentEncode)?
    };
    // TODO: Read through request to get url + headers
    // let content_types;
    for line in input {
      let line = line.map_err(ReqFail::IOOpFailed)?;
      if line == "" {
        break;
      }
    }

    Ok(Request { path })
  }
}

#[derive(Debug)]
pub enum Response {
  Ok {
    headers: Vec<(String, String)>,
    body_type: String,
    body_len: usize,
    body: File,
  },
  NotFound,
  Moved(String),
}

impl Response {
  pub fn code(&self) -> u16 {
    match self {
      Response::Ok { .. } => 200,
      Response::NotFound => 404,
      Response::Moved(_) => 301,
    }
  }

  pub fn to(req: Request, cfg: &cfg::Config) -> io::Result<Response> {
    let filepath = cfg.root.join(&req.path);
    let filepath = if filepath.is_dir() {
      // enforce trailing / (except if request is for root)
      if req.path.len() > 0 && !req.path.ends_with("/") {
        return Ok(Response::Moved(format!("/{}/", req.path)));
      }
      filepath.join("index.html")
    } else {
      filepath
    };
    // TODO: More robust extension checking + checking for match with Accept header
    let ext = match filepath.extension() {
      Some(e) => e.to_owned(),
      None => "".into(),
    };
    let mapped_type = match cfg.mappings.get(&ext) {
      Some(t) => t.clone(),
      None => "text/plain".into(),
    };
    let doc = match File::open(filepath) {
      Ok(d) => d,
      Err(e) => match e.kind() {
        io::ErrorKind::NotFound => return Ok(Response::NotFound),
        _ => return Err(e),
      },
    };
    let metadata = doc.metadata()?;
    Ok(Response::Ok {
      headers: vec![],
      body_type: mapped_type,
      body_len: metadata.len() as usize,
      body: doc,
    })
  }

  pub fn write(self, conn: net::TcpStream) -> io::Result<()> {
    let mut bufout = BufWriter::new(conn);
    let mut head = |code, ctype, len| {
      write!(
          bufout,
          concat!(
              "HTTP/1.1 {code}\n",
              "Cache-Control: no-cache\n",
              "Connection: close\n",
              "Content-Type: {type}\n",
              "Content-Length: {len}\n",
          ),
          code = code,
          type = ctype,
          len = len,
      )
    };
    match self {
      Response::Ok {
        headers,
        body_type,
        body_len,
        mut body,
      } => {
        head("200 OK", &body_type[..], body_len)?;
        for (name, val) in headers {
          write!(bufout, "{}: {}\n", name, val)?;
        }
        write!(bufout, "\n")?;
        io::copy(&mut body, &mut bufout)?;
      }
      Response::NotFound => {
        head("404 Not Found", "text/plain", 0)?;
        write!(bufout, "\n")?;
      }
      Response::Moved(to) => {
        head("301 Moved Permanently", "text/plain", 0)?;
        write!(bufout, "Location: {to}\n\n", to = to)?;
      }
    };
    Ok(())
  }
}

#[cfg(test)]
mod test {
  use super::*;

  mod request {
    use super::*;

    // TODO: How to mock TcpStream?
    // to test:
    //  - correct URL extracted from first line
    //  - malicious URLs rejected
  }

  mod result {
    use super::*;

    // TODO: How to mock filesystem?
    // to test:
    //  - if directory but no trailing /, Moved("{}/")
    //  - if directory otherwise, loads index.html
    //  - correct file loaded based on path
    //  - correct content-type based on extension
    //  - NotFound for nonexistent files
  }

  mod write {
    use super::*;

    // TODO: How to mock TcpStream?
    // to test:
    //  - response format is correct
    //  - .code() matches output code
    //  - all required headers are outputted
  }
}
