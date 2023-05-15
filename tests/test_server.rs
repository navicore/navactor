// use poem::{http::StatusCode, test::TestClient};
// use poem_openapi::{registry::MetaApi, OpenApi, OpenApiService};
//
// #[tokio::test]
// async fn test_hello() {
//     let ep = OpenApiService::new(navactor::server::Api, "hello", "1.0");
//     let cli = TestClient::new(ep);
//
//     let resp = cli.get("/hello").send().await;
//     resp.assert_status(StatusCode::OK);
//
//     let resp = cli.get("/err").send().await;
//     resp.assert_status(StatusCode::NOT_FOUND);
//
//     let meta: MetaApi = navactor::server::Api::meta().remove(0);
//     assert_eq!(meta.paths[0].path, "/hello");
//     let responses = &meta.paths[0].operations[0].responses.responses;
//     assert_eq!(responses[0].status, Some(200));
// }
