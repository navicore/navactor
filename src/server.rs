use warp::Filter;

pub async fn serve(interface: Option<String>, port: Option<u16>) {
    let p = port.unwrap_or(8800);
    let i = interface.unwrap_or("0.0.0.0".to_string());

    // GET /hello/warp => 200 OK with body "Hello, warp!"
    let hello = warp::path!("hello" / String).map(|name| format!("Hello, {}!", name));

    warp::serve(hello).run(([127, 0, 0, 1], p)).await;
}
