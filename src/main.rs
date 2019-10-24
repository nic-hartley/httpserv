use std::{
  collections::HashMap,
  env,
  ffi::OsString,
  fmt,
  fs::File,
  io::{self, BufRead, BufReader, BufWriter, Lines, Write as _},
  net,
  path::{Component, Path, PathBuf},
  time::Instant,
};

#[derive(Debug)]
struct Config {
  root: PathBuf,
  hostname: String,
  mappings: HashMap<OsString, String>,
}

#[derive(Debug)]
enum ArgFail {
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

fn get_args() -> Result<Config, ArgFail> {
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

#[derive(Debug)]
enum ReqFail {
  EarlyInputEnd,
  InvalidFormat(String),
  IOOpFailed(io::Error),
  Malicious(&'static str),
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
    }
  }
}

fn get_path<B: BufRead>(input: &mut Lines<B>) -> Result<String, ReqFail> {
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
  let url_path = Path::new(&url);
  if url_path.components().any(|c| c == Component::ParentDir) {
    return Err(ReqFail::Malicious(".. component in path"));
  }
  Ok(url.to_owned())
}

struct Request {
  path: String,
}

fn get_request(conn: &mut net::TcpStream) -> Result<Request, ReqFail> {
  let mut input = BufReader::new(conn).lines();
  let path = get_path(&mut input)?;
  // TODO: Read through request to get url + headers
  // let content_types;
  for line in input {
    let line = line.map_err(ReqFail::IOOpFailed)?;
    if line == "" {
      break;
    }
    // println!("Header: {}", line);
  }

  Ok(Request { path })
}

#[derive(Debug)]
enum Response {
  Ok {
    headers: Vec<(String, String)>,
    body_type: String,
    body_len: usize,
    body: File, // TODO replace with concrete type?
  },
  NotFound,
  Error(String),
  Moved(String),
}

impl Response {
  fn code(&self) -> u16 {
    match self {
      Response::Ok { .. } => 200,
      Response::NotFound => 404,
      Response::Error(_) => 500,
      Response::Moved(_) => 301,
    }
  }
}

fn get_response(req: Request, cfg: &Config) -> Response {
  let filepath = cfg.root.join(&req.path);
  let filepath = if filepath.is_dir() {
    // enforce trailing / (except if request is for root)
    if req.path.len() > 0 && !req.path.ends_with("/") {
      return Response::Moved(format!("{}/", req.path));
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
      io::ErrorKind::NotFound => return Response::NotFound,
      o => return Response::Error(format!("{:?}", o)),
    },
  };
  let metadata = match doc.metadata() {
    Ok(m) => m,
    Err(e) => {
      return Response::Error(format!("Failed to read metadata: {}", e))
    }
  };
  Response::Ok {
    headers: vec![],
    body_type: mapped_type,
    body_len: metadata.len() as usize,
    body: doc,
  }
}

fn write_response(conn: net::TcpStream, response: Response) -> io::Result<()> {
  let mut bufout = BufWriter::new(conn);
  let mut head = |code, ctype, len| {
    write!(
        bufout,
        concat!(
            "HTTP/1.1 {code}\n",
            "Cache-Control: no-cache\n",
            "Connection: close\n",
            "Content-Type: {type}; charset=UTF-8\n",
            "Content-Length: {len}\n",
        ),
        code = code,
        type = ctype,
        len = len,
    )
  };
  match response {
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
    Response::Error(msg) => {
      head("500 Internal Server Error", "text/plain", msg.len())?;
      write!(bufout, "\n{msg}", msg = msg)?;
    }
    Response::Moved(to) => {
      head("301 Moved Permanently", "text/plain", 0)?;
      write!(bufout, "Location: {to}\n\n", to = to)?;
    }
  };
  Ok(())
}

fn main() {
  let load_start = Instant::now();

  let cfg = match get_args() {
    Ok(t) => t,
    Err(msg) => {
      println!("Invalid command: {}", msg);
      return;
    }
  };

  let listener = match net::TcpListener::bind(&cfg.hostname) {
    Ok(l) => l,
    Err(e) => {
      eprintln!("Failed to listen on {}: {}", cfg.hostname, e);
      return;
    }
  };

  let load_time = Instant::now().duration_since(load_start);
  println!(
    "Launched in {}ms; listening on {}; serving from {}",
    load_time.as_millis(),
    cfg.hostname,
    cfg.root.display()
  );

  for conn in listener.incoming() {
    let resp_start = Instant::now();

    // just ignore failed connections
    let mut conn = match conn {
      Ok(s) => s,
      Err(_) => continue,
    };

    let request = match get_request(&mut conn) {
      Ok(u) => u,
      Err(e) => {
        println!("Failed to get path: {}", e);
        continue;
      }
    };
    print!("Served /{} ", request.path);

    let response = get_response(request, &cfg);
    print!("with {} ", response.code());

    if let Err(e) = write_response(conn, response) {
      println!("Failed to write to pipe: {:?}", e);
      continue;
    }

    let resp_time = Instant::now().duration_since(resp_start);
    println!("in {}ms.", resp_time.as_millis());
  }
}
