use std::{
    collections::HashMap,
    env, fmt,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Lines, Read, Write as _},
    net,
    path::{Path, PathBuf},
    time::Instant,
    ffi::OsString,
};

#[derive(Debug)]
enum Failure {
    EarlyInputEnd,
    EarlyOutputEnd,
    InvalidFormat(String),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Failure::EarlyInputEnd => write!(f, "input ended earlier than expected"),
            Failure::EarlyOutputEnd => write!(f, "output pipe broke while we had more to write"),
            Failure::InvalidFormat(s) => write!(f, "'{}' is incorrectly formatted", s),
        }
    }
}

fn get_args() -> Result<(PathBuf, String, HashMap<OsString, String>), Failure> {
    let mut args = env::args().skip(1);
    let dir = Path::new(&args.next().unwrap_or(".".into())).to_path_buf();
    let host = args.next().unwrap_or("localhost:8080".into());
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
            .ok_or_else(|| Failure::InvalidFormat(pair.clone()))?;
        let (ext, mime) = pair.split_at(eq_idx);
        mappings.insert(ext.into(), mime.into());
    }

    Ok((dir, host, mappings))
}

fn get_path<B: BufRead>(input: &mut Lines<B>) -> Result<String, Failure> {
    let first_line = input
        .next()
        .ok_or(Failure::EarlyInputEnd)?
        .or(Err(Failure::EarlyInputEnd))?;
    // +1 because we actually care about the next character, not this one
    let url_start = first_line
        .find(" ")
        .ok_or_else(|| Failure::InvalidFormat(first_line.clone()))?
        + 1;
    let url_length = first_line[url_start..]
        .find(" ")
        .ok_or_else(|| Failure::InvalidFormat(first_line.clone()))?;
    Ok(first_line[url_start..url_start + url_length].to_owned())
}

enum Response {
    Ok {
        headers: Vec<(String, String)>,
        body_type: String,
        body_len: u64,
        body: Box<dyn Read>, // TODO replace with concrete type?
    },
    NotFound,
    Error(String),
}

impl Response {
    fn code(&self) -> u16 {
        match self {
            Response::Ok { .. } => 200,
            Response::NotFound => 404,
            Response::Error(_) => 500,
        }
    }
}

fn get_response(filepath: &Path, mappings: &HashMap<OsString, String>) -> Response {
    // TODO: More robust extension checking + checking for match with Accept header
    let ext = match filepath.extension() {
        Some(e) => e.to_owned(),
        None => "".into(),
    };
    let mapped_type = match mappings.get(&ext) {
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
    let metadata = doc
        .metadata()
        .expect("failed to read metadata for real file");
    Response::Ok {
        headers: vec![],
        body_type: mapped_type,
        body_len: metadata.len(),
        body: Box::new(doc),
    }
}

fn write_response(conn: net::TcpStream, response: Response) -> io::Result<()> {
    let mut bufout = BufWriter::new(conn);
    match response {
        Response::Ok {
            headers,
            body_type,
            body_len,
            mut body,
        } => {
            write!(bufout, "HTTP/1.1 200 OK\n")?;
            write!(bufout, "Content-Type: {}\n", body_type)?;
            write!(bufout, "Content-Length: {}\n", body_len)?;
            write!(bufout, "Connection: close\n",)?;
            for (name, val) in headers {
                write!(bufout, "{}: {}\n", name, val)?;
            }
            write!(bufout, "\n")?;
            io::copy(&mut body, &mut bufout)?;
        }
        Response::NotFound => {
            write!(
                bufout,
                concat!(
                    "HTTP/1.1 404 Not Found\n",
                    "Content-Length: 0\n",
                    "Connection: close\n",
                    "\n"
                )
            )?;
        }
        Response::Error(msg) => {
            write!(
                bufout,
                concat!(
                    "HTTP/1.1 404 Not Found\n",
                    "Content-Length: {len}\n",
                    "Connection: close\n",
                    "\n",
                    "{msg}"
                ),
                len = msg.len(),
                msg = msg
            )?;
        }
    };
    Ok(())
}

fn main() {
    let load_start = Instant::now();
    let (dir, host, mappings) = match get_args() {
        Ok(t) => t,
        Err(msg) => {
            println!("Invalid command: {:?}", msg);
            return;
        }
    };
    let listener = match net::TcpListener::bind(&host) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to listen on {}: {:?}", host, e);
            return;
        }
    };
    let load_time = Instant::now().duration_since(load_start);
    println!(
        "Launched in {}ms; listening on {}; serving from {}",
        load_time.as_millis(),
        host,
        dir.display()
    );
    for conn in listener.incoming() {
        let resp_start = Instant::now();
        // just ignore failed connections
        let mut conn = match conn {
            Ok(s) => s,
            Err(_) => continue,
        };
        // TODO: Read through request to get url + headers
        let mut input = BufReader::new(&mut conn).lines();
        let url = match get_path(&mut input) {
            Ok(u) => u,
            Err(e) => {
                println!("Failed to get path: {:?}", e);
                continue;
            }
        };
        let filepath = dir.join(&url[1..]);
        let filepath = if filepath.is_dir() {
            filepath.join("index.html")
        } else {
            filepath
        };
        // let content_types;
        for line in input {
            let line = line.expect("failed to read from TCP");
            if line == "" {
                break;
            }
            // println!("Header: {}", line);
        }
        let response = get_response(&filepath, &mappings);
        let code = response.code();
        if let Err(e) = write_response(conn, response) {
            println!("Failed to write to pipe: {:?}", e);
            continue;
        }
        let resp_time = Instant::now().duration_since(resp_start);
        println!(
            "Served {} with {} in {}ms",
            url,
            code,
            resp_time.as_millis()
        );
    }
}
