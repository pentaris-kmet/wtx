use crate::{
  database::client::postgres::{
    executor_buffer::ExecutorBufferPartsMut, query, Executor, ExecutorBuffer, MessageTy,
  },
  misc::{FilledBufferWriter, LeaseMut, Stream},
};

impl<E, EB, S> Executor<E, EB, S>
where
  EB: LeaseMut<ExecutorBuffer>,
  S: Stream,
{
  pub(crate) async fn simple_query_execute(
    &mut self,
    cmd: &str,
    mut cb: impl FnMut(u64),
  ) -> crate::Result<()> {
    let ExecutorBufferPartsMut { nb, rb, vb, .. } = self.eb.lease_mut().parts_mut();
    ExecutorBuffer::clear_cmd_buffers(nb, rb, vb);
    let mut fbw = FilledBufferWriter::from(&mut self.eb.lease_mut().nb);
    query(cmd.as_bytes(), &mut fbw)?;
    self.stream.write_all(fbw._curr_bytes()).await?;
    loop {
      let msg = Self::fetch_msg_from_stream(
        &mut self.is_closed,
        &mut self.eb.lease_mut().nb,
        &mut self.stream,
      )
      .await?;
      match msg.ty {
        MessageTy::CommandComplete(n) => cb(n),
        MessageTy::EmptyQueryResponse => {
          cb(0);
        }
        MessageTy::ReadyForQuery => return Ok(()),
        _ => return Err(crate::Error::UnexpectedDatabaseMessage { received: msg.tag }),
      }
    }
  }
}
