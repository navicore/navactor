use crate::actor::Handle;
use crate::message::Message;
use crate::message::MtHint;
use poem::{
    http::StatusCode, listener::TcpListener, web::Data, EndpointExt, Error, FromRequest, Request,
    RequestBody, Result, Route,
};
use serde::Serialize;
use serde_json::to_string;
use std::ops::Deref;

use poem_openapi::{
    param::Path,
    payload::{Json, PlainText},
    ApiResponse, Object, OpenApi, OpenApiService,
};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Object, Serialize)]
struct ApiStateReport {
    datetime: String,
    path: String,
    values: HashMap<i32, f64>, // TODO: why does the api ui show this as a String, Number map?
}

#[derive(ApiResponse)]
enum PostResponse {
    #[oai(status = 200)]
    ApiStateReport(Json<ApiStateReport>),

    #[oai(status = 404)]
    NotFound(PlainText<String>),
}

#[derive(ApiResponse)]
enum GetResponse {
    #[oai(status = 200)]
    ApiStateReport(Json<ApiStateReport>),

    #[oai(status = 404)]
    NotFound(PlainText<String>),
}

struct NvApi;
pub struct SharedHandle(Arc<Handle>);

impl Deref for SharedHandle {
    type Target = Handle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[poem::async_trait]
impl<'a> FromRequest<'a> for SharedHandle {
    async fn from_request(req: &'a Request, _body: &mut RequestBody) -> Result<Self> {
        log::debug!("from_request");
        match req.data::<Arc<Handle>>() {
            Some(shared_handle) => Ok(SharedHandle(Arc::clone(&shared_handle))),
            None => Err(Error::from_string("error", StatusCode::BAD_REQUEST)),
        }
    }
}

// TODO: /actors/building1/floor10/room5/mymachine

#[OpenApi]
impl NvApi {
    #[oai(path = "/:namespace/:id", method = "get")]
    async fn get(
        &self,
        nv: Data<&SharedHandle>,
        namespace: Path<String>,
        id: Path<String>,
    ) -> Result<GetResponse, poem::Error> {
        log::debug!("get {}/{}", namespace.as_str(), id.as_str());
        // query state of actor one from above updates
        let cmd: Message<f64> = Message::Content {
            text: format!(
                "{{ \"path\": \"/{}/{}\" }}",
                namespace.as_str(),
                id.as_str()
            ),
            path: None,
            hint: MtHint::Query,
        };
        match nv.ask(cmd).await {
            Ok(r) => {
                if let Message::StateReport {
                    datetime,
                    path,
                    values,
                } = r
                {
                    Ok(GetResponse::ApiStateReport(Json(ApiStateReport {
                        datetime: datetime.to_string(),
                        path,
                        values,
                    })))
                } else {
                    Ok(GetResponse::NotFound(PlainText(format!(
                        "todo `{}` not found ha ha too",
                        id.0
                    ))))
                }
            }
            Err(e) => Ok(GetResponse::NotFound(PlainText(format!(
                "todo `{}` not found?",
                id.0
            )))),
        }
    }

    #[oai(path = "/:namespace/:id", method = "post")]
    async fn post(
        &self,
        nv: Data<&SharedHandle>,
        namespace: Path<String>,
        id: Path<String>,
        body: Json<ApiStateReport>,
    ) -> Result<PostResponse, poem::Error> {
        log::debug!("post {}/{}", namespace.as_str(), id.as_str());
        // record observation
        let body_str = to_string(&body.0).unwrap_or_else(|e| {
            log::error!("Failed to serialize JSON: {:?}", e);
            String::new()
        });
        let cmd: Message<f64> = Message::Content {
            text: body_str,
            path: None,
            hint: MtHint::Update,
        };
        match nv.ask(cmd).await {
            Ok(r) => {
                if let Message::StateReport {
                    datetime,
                    path,
                    values,
                } = r
                {
                    Ok(PostResponse::ApiStateReport(Json(ApiStateReport {
                        datetime: datetime.to_string(),
                        path,
                        values,
                    })))
                } else {
                    Ok(PostResponse::NotFound(PlainText(format!(
                        "todo `{}` not found ha ha too",
                        id.0
                    ))))
                }
            }
            Err(e) => Ok(PostResponse::NotFound(PlainText(format!(
                "todo `{}` not found?",
                id.0
            )))),
        }
    }
}

impl Clone for SharedHandle {
    fn clone(&self) -> Self {
        SharedHandle(Arc::clone(&self.0))
    }
}

pub async fn serve<'a>(
    nv: Arc<Handle>,
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

    let api_service = OpenApiService::new(NvApi, clap::crate_name!(), clap::crate_version!())
        .server(swagger_api_target.clone());

    let app = {
        let uip = uipath
            .unwrap_or("".to_string())
            .trim_start_matches('/')
            .to_string();
        let swagger_ui_host = format!("{}/{}", external_host_str, uip);
        log::debug!("swagger UI is available at {}.", swagger_ui_host);
        let ui = api_service.swagger_ui();

        if disui {
            Route::new()
                .nest("/api", api_service)
                .data(SharedHandle(nv.clone()))
        } else {
            Route::new()
                .nest(format!("/{}", uip), ui)
                .nest("/api", api_service)
                .data(SharedHandle(nv.clone()))
        }
    };

    let server = poem::Server::new(TcpListener::bind(ifc_host_str)).run(app);
    log::info!("navactor API is available at {}.", swagger_api_target);
    server.await
}
