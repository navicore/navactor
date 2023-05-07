use poem::{listener::TcpListener, Route};
use poem_openapi::{param::Query, payload::PlainText, OpenApi, OpenApiService};

pub struct Api;

#[OpenApi]
impl Api {
    #[oai(path = "/hello", method = "get")]
    async fn index(&self, name: Query<Option<String>>) -> PlainText<String> {
        match name.0 {
            Some(name) => PlainText(format!("hello, {}!", name)),
            None => PlainText("hello!".to_string()),
        }
    }
}

pub async fn serve(
    interface: Option<String>,
    port: Option<u16>,
    external_host: Option<String>,
    uipath: Option<String>,
    disable_ui: Option<bool>,
) -> Result<(), std::io::Error> {
    let p = port.unwrap_or(8800);
    let i = interface.unwrap_or("127.0.0.1".to_string());
    let disui = disable_ui.unwrap_or(false);
    let ifc_host_str = format!("{}:{}", i, p);
    let default_external_host_str = format!("http://localhost:{}", p);
    let external_host_str = external_host.unwrap_or(default_external_host_str);
    let swagger_api_target = format!("{}/api", external_host_str);

    log::debug!("navactor server starting on {}:{}.", i, p);

    let api_service =
        OpenApiService::new(Api, "Hello World", "1.0").server(swagger_api_target.clone());

    let app = if disui {
        Route::new().nest("/api", api_service)
    } else {
        let uip = uipath
            .unwrap_or("".to_string())
            .trim_start_matches('/')
            .to_string();
        let swagger_ui_host = format!("{}/{}", external_host_str, uip);
        log::debug!("swagger UI is available at {}.", swagger_ui_host);
        let ui = api_service.swagger_ui();
        Route::new()
            .nest("/api", api_service)
            .nest(format!("/{}", uip), ui)
    };

    let server = poem::Server::new(TcpListener::bind(ifc_host_str)).run(app);
    log::info!("navactor API is available at {}.", swagger_api_target);
    server.await
}
