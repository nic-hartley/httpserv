use std::{
  net,
  time::Instant,
};

mod cfg;
mod http;

fn main() {
  let load_start = Instant::now();

  let cfg = match cfg::Config::from_env() {
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
    "Launched in {}us; listening on {}; serving from {}",
    load_time.as_micros(),
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

    let request = match http::Request::load(&mut conn) {
      Ok(u) => u,
      Err(e) => {
        println!("Failed to get path: {}", e);
        continue;
      }
    };
    print!("Served /{} ", request.path);

    let response = http::Response::to(request, &cfg);
    print!("with {} ", response.code());

    if let Err(e) = response.write(conn) {
      println!("Failed to write to pipe: {:?}", e);
      continue;
    }

    let resp_time = Instant::now().duration_since(resp_start);
    println!("in {}us.", resp_time.as_micros());
  }
}
