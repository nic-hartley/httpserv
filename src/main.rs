use std::{
    collections::HashMap,
    env,
    io::{Write as _},
    net,
};

fn get_args() -> Result<(String, String, Vec<(String, String)>), String> {
    let mut args = env::args().skip(1);
    let dir = args.next().unwrap_or(".".into());
    let host = args.next().unwrap_or("localhost:8080".into());
    let mappings = vec![
        (".html", "text/html"),
        (".css", "text/css"),
        (".js", "text/javascript"),
        (".png", "image/png"),
        (".jpg", "image/jpeg"),
        (".jpeg", "image/jpeg"),
        (".ico", "image/vnd.microsoft.icon"),
        (".svg", "image/svg+xml"),
    ]
    .into_iter()
    .chain(
        args.next()
            .unwrap_or("".into())
            .split(",")
            .filter_map(|a| Some(a.split_at(a.find("=")?))),
    )
    .fold(HashMap::new(), |mut hm, (e, m)| {
        hm.insert(e, m);
        hm
    })
    .into_iter()
    .map(|(ext, mime)| (ext.into(), mime.into()))
    .collect();

    Ok((dir, host, mappings))
}

fn main() {
    let (dir, host, _mappings) = match get_args() {
        Ok(t) => t,
        Err(msg) => {
            eprintln!("{}", msg);
            return;
        }
    };
    println!("Listening on {}; serving from {}", host, dir);

    let listener = match net::TcpListener::bind(host) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to listen where told: {:?}", e);
            return;
        }
    };

    for conn in listener.incoming() {
        // just ignore failed connections
        let mut conn = match conn {
            Ok(s) => s,
            Err(_) => continue,
        };
        // TODO: Read through request to get url + headers
        // TOOD: Send response based on url, relevant headers
        println!("Connected from {:?}", conn.peer_addr());
        std::thread::sleep(std::time::Duration::from_millis(500));
        write!(conn, concat!(
            "HTTP/1.1 200 OK\n",
            "Content-Type: text/plain\n",
            "Content-Length: 13\n",
            "\n",
            "Hello, World!"
        )).expect("AA");
        conn.flush().expect("AAA");
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}
