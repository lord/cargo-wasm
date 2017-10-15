use std::thread::spawn;
use std::{thread, time};
use std::sync::{Arc, Mutex};
use tiny_http::{Server, Response};

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

            println!("received request! method: {:?}, url: {:?}, headers: {:?}",
                request.method(),
                request.url(),
                request.headers()
            );
            let response = Response::from_string("hello world");
            request.respond(response);
        }
    });

    while !run_server.lock().unwrap().clone() {
        thread::sleep(time::Duration::from_millis(10));
    }

    run_server
}
