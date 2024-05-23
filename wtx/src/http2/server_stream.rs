use tokio::sync::MutexGuard;

use crate::{
  http::{Method, ReqResData, Response},
  http2::{
    misc::{send_go_away, send_reset_stream},
    send_msg::send_msg,
    HpackStaticRequestHeaders, HpackStaticResponseHeaders, Http2Buffer, Http2Data, Http2ErrorCode,
    StreamBuffer, StreamControlRecvParams, U31,
  },
  misc::{ByteVector, Lease, LeaseMut, Lock, RefCounter, Stream, _Span},
};

/// Created when a server receives an initial stream.
#[derive(Debug)]
pub struct ServerStream<HD> {
  hd: HD,
  method: Method,
  span: _Span,
  stream_id: U31,
}

impl<HD> ServerStream<HD> {
  #[inline]
  pub(crate) const fn new(hd: HD, method: Method, span: _Span, stream_id: U31) -> Self {
    Self { hd, method, span, stream_id }
  }
}

impl<HB, HD, S, SB> ServerStream<HD>
where
  HB: LeaseMut<Http2Buffer<SB>>,
  HD: RefCounter,
  for<'guard> HD::Item: Lock<
      Guard<'guard> = MutexGuard<'guard, Http2Data<HB, S, SB, false>>,
      Resource = Http2Data<HB, S, SB, false>,
    > + 'guard,
  S: Stream,
  SB: LeaseMut<StreamBuffer>,
{
  /// Awaits for all remaining data to build a request.
  #[inline]
  pub async fn recv_req(&mut self) -> crate::Result<(SB, Method)> {
    let _e = self.span._enter();
    _trace!("Receiving request");
    process_receipt_loop!(self.hd, |guard| {
      let hdpm = guard.parts_mut();
      if hdpm.hb.sorp.get(&self.stream_id).map_or(false, |el| el.stream_state.recv_eos()) {
        if let Some(sorp) = hdpm.hb.sorp.remove(&self.stream_id) {
          let _ = hdpm.hb.scrp.insert(
            self.stream_id,
            StreamControlRecvParams { stream_state: sorp.stream_state, windows: sorp.windows },
          );
          return Ok((sorp.sb, self.method));
        }
      }
    });
  }

  /// Sends a GOAWAY frame to the peer, which cancels the connection and consequently all ongoing
  /// streams.
  pub async fn send_go_away(self, error_code: Http2ErrorCode) {
    let mut guard = self.hd.lock().await;
    let hdpm = guard.parts_mut();
    send_go_away(error_code, hdpm.is_conn_open, *hdpm.last_stream_id, hdpm.stream).await
  }

  /// Auxiliary high-level method that sends a response.
  ///
  /// Should be called after [Self::recv_req] is successfully executed.
  #[inline]
  pub async fn send_res<D>(
    &mut self,
    hpack_enc_buffer: &mut ByteVector,
    res: Response<D>,
  ) -> crate::Result<()>
  where
    D: ReqResData,
    D::Body: Lease<[u8]>,
  {
    let _e = self.span._enter();
    _trace!("Sending response");
    send_msg::<_, _, _, _, false>(
      res.data.body().lease(),
      &self.hd,
      res.data.headers(),
      hpack_enc_buffer,
      (
        HpackStaticRequestHeaders::EMPTY,
        HpackStaticResponseHeaders { status_code: Some(res.status_code) },
      ),
      self.stream_id,
      |hdpm| {
        let _ = hdpm.hb.scrp.remove(&self.stream_id);
      },
    )
    .await
  }

  /// Sends a stream reset to the peer, which cancels this stream.
  pub async fn send_reset(&mut self, error_code: Http2ErrorCode) -> crate::Result<()> {
    let mut guard = self.hd.lock().await;
    let hdpm = guard.parts_mut();
    send_reset_stream(error_code, &mut hdpm.hb.sorp, self.stream_id, hdpm.stream).await?;
    Ok(())
  }
}
