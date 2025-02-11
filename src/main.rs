use lambda_http::{run, service_fn, tracing, Error};
mod http_handler;
use http_handler::function_handler;
mod media_upload;
mod info_upload;
mod post_download;
mod recommendations;
mod post_sorting;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}
