//! Http2 echo server.

#[path = "./common/mod.rs"]
mod common;

use wtx::{
  http::{server::OptionedServer, Headers, RequestStr, Response, StatusCode},
  http2::{Http2Buffer, Http2Params, StreamBuffer},
  misc::ByteVector,
  rng::StdRng,
};

#[tokio::main]
async fn main() {
  OptionedServer::tokio_http2(
    common::_host_from_args().parse().unwrap(),
    Some(999),
    |err| eprintln!("Error: {err:?}"),
    handle,
    || Ok(Http2Buffer::new(StdRng::default())),
    || Http2Params::default(),
    || Ok(StreamBuffer::default()),
  )
  .await
  .unwrap()
}

async fn handle<'buffer>(
  req: RequestStr<'buffer, (&'buffer mut ByteVector, &'buffer mut Headers)>,
) -> Result<Response<(&'buffer mut ByteVector, &'buffer mut Headers)>, wtx::Error> {
  req.data.1.clear();
  Ok(Response::http2(req.data, StatusCode::Ok))
}
