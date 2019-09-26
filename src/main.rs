use std::{
    collections::HashMap,
    env,
    io::{self, Read, Lines, BufRead, BufReader, Write as _},
    net,
    path::{Path, PathBuf},
    fs::File,
    time::Instant,
};

fn get_args() -> Result<(PathBuf, String, Vec<(String, String)>), String> {
    let mut args = env::args().skip(1);
    let dir = Path::new(&args.next().unwrap_or(".".into())).to_path_buf();
    let host = args.next().unwrap_or("localhost:8080".into());
    let mut mappings = HashMap::new();
    mappings.insert(".html".into(), "text/html".into());
    mappings.insert(".css".into(), "text/css".into());
    mappings.insert(".js".into(), "text/javascript".into());
    mappings.insert(".png".into(), "image/png".into());
    mappings.insert(".jpg".into(), "image/jpeg".into());
    mappings.insert(".jpeg".into(), "image/jpeg".into());
    mappings.insert(".ico".into(), "image/vnd.microsoft.icon".into());
    mappings.insert(".svg".into(), "image/svg+xml".into());
    for pair in args {
        let eq_idx = pair.find("=").expect("no =");
        let (ext, mime) = pair.split_at(eq_idx);
        mappings.insert(ext.into(), mime.into());
    }
    let mappings = mappings.into_iter().collect();

    Ok((dir, host, mappings))
}

fn get_path<B: BufRead>(input: &mut Lines<B>) -> String {
    let first_line = input.next().expect("pipe ended early").expect("failed to read from TCP");
    println!("First line: '{}'", first_line);
    let url_start = first_line.find(" ").expect("no space") + 1;
    let url_length = first_line[url_start..].find(" ").expect("no second space");
    // + 1 to remove slash
    first_line[url_start + 1..url_start + url_length].to_owned()
}

struct Response {
    code: &'static str,
    headers: Vec<(String, String)>,
    body_type: String,
    body_len: u64,
    body: Option<Box<dyn Read>>,
}

fn get_response(filepath: &Path) -> Response {
    Response {
        code: "200 OK",
        headers: vec![],
        body_type: "text/plain".into(),
        body_len: 13,
        body: Some(Box::new("Hello, World!".as_bytes())),
    }
    /*
    // TOOD: Send response based on url, relevant headers
    let mut doc = File::open(filepath).expect("failed to open file");
    let metadata = doc.metadata().expect("failed to read metadata");
    */
}

fn main() {
    let load_start = Instant::now();
    let (dir, host, _mappings) = match get_args() {
        Ok(t) => t,
        Err(msg) => {
            eprintln!("{}", msg);
            return;
        }
    };
    let listener = match net::TcpListener::bind(&host) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to listen where told: {:?}", e);
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
        let url = get_path(&mut input);
        let filepath = dir.join(&url);
        println!("request is for {:?} = {:?}", url, filepath);
        // let content_types;
        for line in input {
            let line = line.expect("failed to read from TCP");
            if line == "" {
                break;
            }
            // println!("Header: {}", line);
        }
        let response = get_response(&filepath);
        write!(
            conn,
            concat!(
                "HTTP/1.1 {status}\n",
                "Content-Type: {type}\n",
                "Content-Length: {length}\n",
            ),
            status = response.code,
            type = response.body_type,
            length = response.body_len,
        ).expect("failed to write start to stream");
        for (name, val) in response.headers {
            write!(conn, "{}: {}\n", name, val).expect("failed to write header to stream");
        }
        write!(conn, "\n").expect("failed to write separator to stream");
        if let Some(mut body) = response.body {
            io::copy(&mut body, &mut conn).expect("failed to write body to stream");
        }
        conn.flush().expect("failed to flush response");
        let resp_time = Instant::now().duration_since(resp_start);
        println!("Served {} ({}) in {}ms", url, response.code, resp_time.as_millis());
    }
}
