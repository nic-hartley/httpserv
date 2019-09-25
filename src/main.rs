use std::{
    collections::HashMap,
    env,
    io::{BufRead, BufReader, Write as _},
    net,
    path::{Path, PathBuf},
    time::Instant,
};

fn get_args() -> Result<(PathBuf, String, Vec<(String, String)>), String> {
    let mut args = env::args().skip(1);
    let dir = Path::new(&args.next().unwrap_or(".".into()))
        .canonicalize()
        .expect("invalid path");
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

fn get_path(input: &mut dyn BufRead) -> String {
    let mut first_line = String::new();
    input
        .read_line(&mut first_line)
        .expect("failed to read from TCP");
    let url_start = first_line.find(" ").expect("no space") + 1;
    let url_length = first_line[url_start..].find(" ").expect("no second space");
    first_line[url_start..url_start + url_length].to_owned()
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
        "Launched in {}us; listening on {}; serving from {}",
        load_time.as_micros(),
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
        let mut input = BufReader::new(&mut conn);
        let url = get_path(&mut input);
        for line in input.lines() {
            let line = line.expect("failed to read from TCP");
            if line == "" {
                break;
            }
            // println!("Header: {}", line);
        }
        // TOOD: Send response based on url, relevant headers
        write!(
            conn,
            concat!(
                "HTTP/1.1 200 OK\n",
                "Content-Type: text/plain\n",
                "Content-Length: 13\n",
                "\n",
                "Hello, World!",
            )
        )
        .expect("failed to write response");
        conn.flush().expect("failed to flush response");
        let resp_time = Instant::now().duration_since(resp_start);
        println!("Served {} in {}us", url, resp_time.as_micros());
    }
}
