#![allow(clippy::useless_let_if_seq)]
use crate::actors::actor::Handle;
use crate::actors::genes::gene::GeneType;
use crate::actors::message::Message;
use crate::actors::message::MtHint;
use crate::utils::nvtime::extract_datetime;
use poem::{
    http::StatusCode, listener::TcpListener, web::Data, EndpointExt, Error, FromRequest, Request,
    RequestBody, Result, Route,
};
use poem_openapi::{
    param::Path,
    payload::{Json, PlainText},
    ApiResponse, Object, OpenApi, OpenApiService,
};
use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;
use tracing::debug;
use tracing::info;

pub struct HttpServerConfig {
    pub port: u16,
    pub interface: String,
    pub external_host: String,
    pub namespace: String,
}

impl HttpServerConfig {
    #[must_use]
    pub fn new(
        port: Option<u16>,
        interface: Option<String>,
        external_host: Option<String>,
        namespace: String,
    ) -> Self {
        Self {
            port: port.unwrap_or(8800),
            interface: interface.unwrap_or_else(|| "127.0.0.1".to_string()),
            external_host: external_host.unwrap_or_else(|| "http://localhost:8800".to_string()),
            namespace,
        }
    }
}

impl fmt::Display for HttpServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{} on {}:{} as {}]",
            self.namespace, self.interface, self.port, self.external_host
        )
    }
}

#[derive(Object)]
pub struct ApiObservations {
    pub datetime: String,
    pub values: HashMap<i32, f64>,
    pub path: String,
}

#[derive(Object)]
struct ApiStateReport {
    datetime: String,
    path: String,
    values: HashMap<i32, f64>,
}

#[derive(Object)]
struct ApiGeneMapping {
    path: String,
    gene_type: String,
}

#[derive(ApiResponse)]
enum PostObservationResponse {
    #[oai(status = 200)]
    ApiStateReport(Json<ApiStateReport>),

    #[oai(status = 404)]
    NotFound(PlainText<String>),

    #[oai(status = 400)]
    BadRequest(PlainText<String>),

    #[oai(status = 409)]
    ConstraintViolation(PlainText<String>),

    #[oai(status = 500)]
    InternalServerError(PlainText<String>),
}

#[derive(ApiResponse)]
enum GetStateResponse {
    #[oai(status = 200)]
    ApiStateReport(Json<ApiStateReport>),

    #[oai(status = 404)]
    NotFound(PlainText<String>),

    #[oai(status = 500)]
    InternalServerError(PlainText<String>),
}

#[derive(ApiResponse)]
enum GetGeneMappingResponse {
    #[oai(status = 200)]
    ApiGeneMapping(Json<ApiGeneMapping>),

    #[oai(status = 404)]
    NotFound(PlainText<String>),

    #[oai(status = 500)]
    InternalServerError(PlainText<String>),
}

#[derive(ApiResponse)]
enum PostGeneMappingResponse {
    #[oai(status = 200)]
    ApiGeneMapping(Json<ApiGeneMapping>),

    #[oai(status = 409)]
    ConstraintViolation(PlainText<String>),

    #[oai(status = 500)]
    InternalServerError(PlainText<String>),
}

fn prepend_slash(mut s: String) -> String {
    if !s.starts_with('/') {
        s.insert(0, '/');
    }
    s
}

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
        debug!("from_request");

        req.data::<Arc<Handle>>().map_or_else(
            || {
                Err(Error::from_string(
                    "error",
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            },
            |shared_handle| Ok(Self(Arc::clone(shared_handle))),
        )
    }
}

struct ActorsApi;

#[OpenApi]
impl ActorsApi {
    #[oai(path = "/:namespace<.+/>:id", method = "get")]
    async fn get_state(
        &self,
        nv: Data<&SharedHandle>,
        namespace: Path<String>,
        id: Path<String>,
    ) -> Result<GetStateResponse, poem::Error> {
        let fullpath = format!("{}{}", namespace.as_str(), id.as_str());
        let fullpath = prepend_slash(fullpath);
        debug!("get state for {}", fullpath);
        // query state of actor one from above updates
        let cmd = Message::Query {
            path: fullpath,
            hint: MtHint::State,
        };
        match nv.ask(cmd).await {
            Ok(Message::StateReport {
                datetime: _,
                path: _,
                values,
            }) if values.is_empty() => Ok(GetStateResponse::NotFound(PlainText(format!(
                "No observations for id `{}`",
                id.0
            )))),
            Ok(Message::StateReport {
                datetime,
                path,
                values,
            }) => Ok(GetStateResponse::ApiStateReport(Json(ApiStateReport {
                datetime: datetime.to_string(),
                path,
                values,
            }))),
            m => Ok(GetStateResponse::InternalServerError(PlainText(format!(
                "server error for id {}: {:?}",
                id.0, m
            )))),
        }
    }

    #[oai(path = "/:namespace<.+/>:id", method = "post")]
    async fn post_observations(
        &self,
        nv: Data<&SharedHandle>,
        namespace: Path<String>,
        id: Path<String>,
        body: Json<ApiObservations>,
    ) -> Result<PostObservationResponse, poem::Error> {
        let ns = namespace.trim_end_matches('/').to_string();
        let ns = prepend_slash(ns);
        debug!("post observations {}/{}", ns, id.as_str());
        // record observation
        if let Ok(dt) = extract_datetime(&body.0.datetime) {
            let cmd = Message::Observations {
                path: body.0.path,
                datetime: dt,
                values: body.0.values,
            };

            match nv.ask(cmd).await {
                Ok(Message::StateReport {
                    datetime: _,
                    path: _,
                    values,
                }) if values.is_empty() => Ok(PostObservationResponse::NotFound(PlainText(
                    format!("No actor resurected with id `{}`", id.0),
                ))),
                Ok(Message::StateReport {
                    datetime,
                    path,
                    values,
                }) => Ok(PostObservationResponse::ApiStateReport(Json(
                    ApiStateReport {
                        datetime: datetime.to_string(),
                        path,
                        values,
                    },
                ))),
                Ok(Message::ConstraintViolation {}) => {
                    Ok(PostObservationResponse::ConstraintViolation(PlainText(
                        format!("contraint violation with id {}", id.0),
                    )))
                }
                e => Ok(PostObservationResponse::InternalServerError(PlainText(
                    format!("server error with id {}: {:?}", id.0, e),
                ))),
            }
        } else {
            // TODO: how can this be located near the parse???
            Ok(PostObservationResponse::BadRequest(PlainText(format!(
                "cannot parse datetime {} for id {}",
                body.0.datetime, id.0
            ))))
        }
    }
}

fn extract_gene_type(gene_type_str: &str) -> GeneType {
    match gene_type_str {
        "Gauge" => GeneType::Gauge,
        "Accum" => GeneType::Accum,
        _ => GeneType::GaugeAndAccum,
    }
}

struct GenesApi;

#[OpenApi]
impl GenesApi {
    #[oai(path = "/:namespace<.+/>:id", method = "get")]
    async fn get_gene(
        &self,
        nv: Data<&SharedHandle>,
        namespace: Path<String>,
        id: Path<String>,
    ) -> Result<GetGeneMappingResponse, poem::Error> {
        let fullpath = format!("{}{}", namespace.as_str(), id.as_str());
        let fullpath = prepend_slash(fullpath);
        debug!("get gene for {}", fullpath);
        // query state of actor one from above updates
        let cmd: Message<f64> = Message::Content {
            path: Some(fullpath),
            text: String::new(),
            hint: MtHint::GeneMappingQuery,
        };
        match nv.ask(cmd).await {
            Ok(Message::GeneMapping { path, gene_type }) => Ok(
                GetGeneMappingResponse::ApiGeneMapping(Json(ApiGeneMapping {
                    path,
                    gene_type: gene_type.to_string(),
                })),
            ),
            Ok(Message::NotFound { path }) => Ok(GetGeneMappingResponse::NotFound(PlainText(
                format!("No gene mapping for `{path}`"),
            ))),

            m => Ok(GetGeneMappingResponse::InternalServerError(PlainText(
                format!("server error for path {}: {:?}", id.0, m),
            ))),
        }
    }

    #[oai(path = "/:namespace<.+/>:id", method = "post")]
    async fn post_gene_mapping(
        &self,
        nv: Data<&SharedHandle>,
        namespace: Path<String>,
        id: Path<String>,
        body: Json<ApiGeneMapping>,
    ) -> Result<PostGeneMappingResponse, poem::Error> {
        let fullpath = format!("{}{}", namespace.as_str(), id.as_str());
        let fullpath = prepend_slash(fullpath);
        debug!("post gene mapping for {fullpath}");

        let cmd = Message::GeneMapping {
            path: fullpath,
            gene_type: extract_gene_type(&body.0.gene_type),
        };

        match nv.ask(cmd).await {
            Ok(Message::GeneMapping { path, gene_type }) => Ok(
                PostGeneMappingResponse::ApiGeneMapping(Json(ApiGeneMapping {
                    path,
                    gene_type: gene_type.to_string(),
                })),
            ),
            Ok(Message::ConstraintViolation {}) => {
                Ok(PostGeneMappingResponse::ConstraintViolation(PlainText(
                    format!("contraint violation with id {}", id.0),
                )))
            }
            e => Ok(PostGeneMappingResponse::InternalServerError(PlainText(
                format!("server error with id {}: {:?}", id.0, e),
            ))),
        }
    }
}

impl Clone for SharedHandle {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

/// start a server on port and interface
///
/// # Errors
///
/// Returns `Err` if server can not be started
pub async fn serve(
    nv: Arc<Handle>,
    server_config: HttpServerConfig,
    uipath: Option<String>,
    disable_ui: Option<bool>,
) -> Result<(), std::io::Error> {
    info!("starting server: {server_config}");

    let disui = disable_ui.unwrap_or(false);
    let ifc_host_str = format!("{}:{}", server_config.interface, server_config.port);
    let swagger_api_target = format!("{}/api", server_config.external_host);

    let actors_service =
        OpenApiService::new(ActorsApi, clap::crate_name!(), clap::crate_version!())
            .server(swagger_api_target.clone());

    let spec = actors_service.spec();
    std::fs::write("/tmp/navactor_spec.json", spec)?;

    let genes_service = OpenApiService::new(GenesApi, clap::crate_name!(), clap::crate_version!())
        .server(swagger_api_target.clone());

    let app = {
        if disui {
            Route::new()
                .nest("/api/actors", actors_service)
                .nest("/api/genes", genes_service)
                .data(SharedHandle(nv.clone()))
        } else {
            let uip = uipath
                .unwrap_or_default()
                .trim_start_matches('/')
                .to_string();
            let actors_ui = actors_service.swagger_ui();
            let genes_ui = genes_service.swagger_ui();
            Route::new()
                .nest(format!("/{uip}/actors"), actors_ui)
                .nest(format!("/{uip}/genes"), genes_ui)
                .nest("/api/actors", actors_service)
                .nest("/api/genes", genes_service)
                .data(SharedHandle(nv.clone()))
        }
    };

    let server = poem::Server::new(TcpListener::bind(ifc_host_str)).run(app);
    info!("navactor API is available at {}.", swagger_api_target);
    server.await
}
