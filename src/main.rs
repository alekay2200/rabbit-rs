use std::io;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::collections::HashMap;
use std::num;
use std::os::linux::raw;
use std::process;
use std::thread;
use std::sync::{Arc, Mutex};
use std::mem;
use std::time::Duration;
use threadpool::ThreadPool;

const IP: &str = "localhost";
const PORT: u16 = 8000;

#[derive(PartialEq, Eq, Hash)]
enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    UPDATE
}

struct Request<'a> {
    method: Method,
    path: &'a str,
}

impl Request<'static> {
    fn new(raw_http_request: &String) -> Request {
        let mut lines = raw_http_request.lines();
        
        let string = lines.next().unwrap();

        let v: Vec<&str> = string.split(" ").collect();

        let m = v[0];

        let mut method = Method::GET;

        match m {
            "GET" => method = Method::GET,
            "" => {},
            &_ => {}
        }

        return Request { method, path: v[1] };
    }
}

struct Response<'a> {
    msg: &'a str
}


// HashMap to store for each http method another HashMap with the path an the function to respond
// at the given request.
// The function must receive one parameter which is the request, and must return a Response
struct Router<'a> {
    routes: HashMap<Method, HashMap<&'a str, fn(Request) -> Response>>
}

impl<'a> Router<'a> {
    fn new() -> Self {
        let methods = HashMap::<Method, HashMap<&str, fn(Request) -> Response>>::new();
        return Router { routes: methods };
    }

    // Add a new route. If the given route already exists then the function response will be updated.
    fn add(&mut self, method: Method, path: &'a str, response_function: fn(Request) -> Response) {
        let urls = self.routes.get_mut(&method);

        match urls {
            Some(paths) => {
                // The method has any urls, insert or update
                paths.insert(path, response_function);
            },
            
            None => {
                // Create new HasMap for this method
                let mut r = HashMap::<&'a str, fn(Request) -> Response>::new();
                r.insert(path, response_function);
                self.routes.insert(method, r);
            }
        }
    }   

    fn get_callback(&self, request: &Request) -> Option<fn(Request) -> Response> {
        if self.routes.contains_key(&request.method) {
            // Find if any routes match the request
            let routes = self.routes.get(&request.method).unwrap();
            
            if routes.contains_key(request.path) {
                return Some(*routes.get(request.path).unwrap());
            } else {
                return None;
            }


        } else {
            return None;
        }
    }

}

struct Rabbit<'a> {
    router: Router<'a>,
    server: Option<TcpListener>,
    ip: &'a str,
    port: u16,
    stop: Arc<Mutex<bool>>,
    max_threads: usize,
}

impl<'a> Rabbit<'a> {
    fn new(ip: &'a str, port: u16, max_threads: usize) -> Self {
        Rabbit { router: Router::new(), ip, port, server: None, stop: Arc::new(Mutex::new(false)), max_threads }
    }

    fn add_route(&mut self, method: Method, path: &'a str, response: fn(Request) -> Response) {
        self.router.add(method, path, response);
    }

    fn stop(&mut self) {
        let mut stop = *(self.stop.lock().unwrap());
        mem::replace(&mut stop, false);
        
    }

    fn is_running(&self) -> bool {
        let stop = *(self.stop.lock().unwrap());
        return stop;
    }

    fn start(&mut self) {
        let address = self.ip.to_string() + ":" +  &self.port.to_string();

        let server = TcpListener::bind(address.as_str());

        match server {

            Ok(socket) => {
                self.server = Some(socket);
                println!("Waiting for connections...")
            },

            Err(error) => {
                eprintln!("Error creating the server: {}", error.to_string());
                process::exit(1);
            }
        }

        loop {
            // Create a ThreadPool to resolve clients requests
            let pool = ThreadPool::new(self.max_threads);

            loop {
                let conn = self.server.as_ref().unwrap().accept();
    
                match conn {
                    Ok((mut socket, address)) => {
                        println!("New connection from: {:?}", address);

                        let mut buffer = [0u8; 1024];

                        let num_bytes_read = match socket.read(buffer.as_mut()) {
                            Ok(n) => n,
                            Err(e) => 0
                        };
                        
                        // The request contain data, convert to a Request structure
                        if num_bytes_read > 0 {
                            println!("Data Received: \n");
                            io::stdout().write_all(&buffer[..num_bytes_read]).unwrap();
                            // Try to find the route
                            
                            let request = String::from_utf8_lossy(buffer.as_mut()).to_string();
                            let request = Request::new(&request);
                            
                            // Get callback
                            // let callback = self.router.get_callback(request);
                            match self.router.get_callback(&request) {
                                Some(function) => {
                                    let response = function(request);
                                    socket.write(response.msg.as_bytes());
                                },

                                None => {
                                    socket.write(b"Not Found");
                                }
                            }

                            
                        }
                    },
    
                    Err(error) => println!("Couldn't get client: {:?}", error)
                }


                // Using ThreadPool

                // pool.execute(move || {
                //     match conn {
                //         Ok((mut socket, address)) => {
                //             println!("New connection from: {:?}", address);

                //             let mut buffer = vec![];
    
                //             let num_bytes_read = match socket.read(buffer.as_mut()) {
                //                 Ok(n) => n,
                //                 Err(e) => 0
                //             };
                            
                //             // The request contain data, convert to a Request structure
                //             if num_bytes_read > 0 {
                //                 println!("Data Received: \n");
                //                 io::stdout().write_all(&buffer[..num_bytes_read]).unwrap();
                //                 socket.write(b"Hello from rabbit");
                //                 // Try to find the route
                                
                //                 let request = String::from_utf8_lossy(buffer.as_mut()).to_string();
                //                 let request = Request::new(&request);
                                
                //                 // Get callback
                //                 // let callback = self.router.get_callback(request);
                //                 // self.router.get_callback(request).unwrap()(request);
                //             }
                //         },
        
                //         Err(error) => println!("Couldn't get client: {:?}", error)
                //     }
                    
                //     thread::sleep(Duration::new(10,0));
                //     println!("Peticion servida")
                            
                // });
            }

        }
    }
}

fn get_cars(request: Request) -> Response {
    println!("The cars are:");
    return Response {msg: "[fiat, audi, sergio rata]"};
}

fn sergio_rata(request: Request) -> Response {
    return Response { msg: "Sergio erest un rata"};
}

fn main() {
    let mut rabbit = Rabbit::new(IP, PORT, 4);

    rabbit.add_route(Method::GET, "/api/cars", get_cars);
    rabbit.add_route(Method::GET, "/sergio/rata", sergio_rata);

    rabbit.start();
}