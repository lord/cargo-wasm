use std::thread::spawn;
use std::{thread, time};
use std::sync::{Arc, Mutex};
use tiny_http;
use tiny_http::{Server, Response};
use std::fs::{read_dir, File};
use std::io::prelude::*;

const PAGE: &str = "<doctype !HTML>
<html>
  <head>
    <script>
      var kinds = ['error', 'info', 'debug', 'warn', 'log'];
      window.TEST_LOGS = [];
      kinds.forEach(function (kind) {
        var old = console[kind];
        console[kind] = function() {
          var args = Array.prototype.slice.call(arguments);
          old.apply(this, args);
          window.TEST_LOGS.push(args.join(' '));
        }
      });
    </script>
    <script src='/load.js'></script>
  </head>
  <body>
  </body>
</html>";

pub fn start_server() -> Arc<Mutex<bool>> {
    let run_server = Arc::new(Mutex::new(false));
    let run_server_copy = run_server.clone();

    spawn(move || {
        let server = Server::http("0.0.0.0:7777").unwrap();
        *(run_server_copy.lock().unwrap()) = true;
        while run_server_copy.lock().unwrap().clone() {
            let request = match server.try_recv() {
                Ok(Some(rq)) => rq,
                Ok(None) => {
                    thread::sleep(time::Duration::from_millis(10));
                    continue;
                }
                Err(e) => {
                    println!("error: {}", e);
                    continue;
                }
            };

            if request.url() == "/" {
                let html_header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap();
                request.respond(Response::from_string(PAGE).with_header(html_header)).expect("failed to send reply");
            } else if request.url() == "/load.js" {
                let paths = read_dir("./target/wasm32-unknown-emscripten/debug/deps").expect("failed to find output js file");
                let mut target_path = None;
                for path in paths {
                    let path = path.expect("failed to unwrap path").path();
                    if let Some(s) = path.clone().extension() {
                        if s == "js" {
                            target_path = Some(path)
                        }
                    }
                }
                if let Some(p) = target_path {
                    let mut file = File::open(p).expect("unable to open file");
                    let mut contents = Vec::new();
                    file.read_to_end(&mut contents).expect("unable to read file");
                    request.respond(Response::from_data(contents)).expect("failed to send reply");
                } else {
                    request.respond(Response::from_string("file not found")).expect("failed to send reply");
                }
            } else {
                match File::open("./target/wasm32-unknown-emscripten/debug/deps".to_owned() + request.url()) {
                    Ok(mut file) => {
                        let mut contents = Vec::new();
                        file.read_to_end(&mut contents).expect("unable to read file");
                        request.respond(Response::from_data(contents)).expect("failed to send reply");
                    }
                    Err(_) => {
                        request.respond(Response::from_string("file not found")).expect("failed to send reply");
                    }
                }
            }
        }
    });

    while !run_server.lock().unwrap().clone() {
        thread::sleep(time::Duration::from_millis(10));
    }

    run_server
}
