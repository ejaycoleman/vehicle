use deno_core::anyhow::Error;
use deno_core::AsyncRefCell;
use deno_core::AsyncResult;
use deno_core::CancelHandle;
use deno_core::CancelTryFuture;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ResourceId;
use deno_core::serde::Serialize;
use deno_core::serde::Serializer;
use deno_core::serde::ser::SerializeStruct;
use std::rc::Rc;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

pub struct TcpListener {
  inner: tokio::net::TcpListener,
  cancel: CancelHandle,
}

impl TcpListener {
  pub async fn accept(self: Rc<Self>) -> Result<TcpStream, std::io::Error> {
    let cancel = RcRef::map(&self, |r| &r.cancel);
    let stream = self.inner.accept().try_or_cancel(cancel).await?.0.into();
    Ok(stream)
  }
}

impl Resource for TcpListener {
  fn close(self: Rc<Self>) {
    self.cancel.cancel();
  }
}

impl TryFrom<std::net::TcpListener> for TcpListener {
  type Error = std::io::Error;
  fn try_from(
    std_listener: std::net::TcpListener,
  ) -> Result<Self, Self::Error> {
    tokio::net::TcpListener::try_from(std_listener).map(|tokio_listener| Self {
      inner: tokio_listener,
      cancel: Default::default(),
    })
  }
}

pub struct TcpStream {
  rd: AsyncRefCell<tokio::net::tcp::OwnedReadHalf>,
  wr: AsyncRefCell<tokio::net::tcp::OwnedWriteHalf>,
  cancel: CancelHandle,
}

impl TcpStream {
  async fn read(self: Rc<Self>, data: &mut [u8]) -> Result<usize, Error> {
    let mut rd = RcRef::map(&self, |r| &r.rd).borrow_mut().await;
    let cancel = RcRef::map(self, |r| &r.cancel);
    let nread = rd.read(data).try_or_cancel(cancel).await?;
    Ok(nread)
  }

  async fn write(self: Rc<Self>, data: &[u8]) -> Result<usize, Error> {
    let mut wr = RcRef::map(self, |r| &r.wr).borrow_mut().await;
    let nwritten = wr.write(data).await?;
    Ok(nwritten)
  }
}

impl Resource for TcpStream {
  deno_core::impl_readable_byob!();
  deno_core::impl_writable!();

  fn close(self: Rc<Self>) {
    self.cancel.cancel()
  }
}

impl From<tokio::net::TcpStream> for TcpStream {
  fn from(s: tokio::net::TcpStream) -> Self {
    let (rd, wr) = s.into_split();
    Self {
      rd: rd.into(),
      wr: wr.into(),
      cancel: Default::default(),
    }
  }
}

pub struct ListenResult{
  pub resource_id: ResourceId,
  pub port: u64,
}

impl Serialize for ListenResult {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
      S: Serializer,
  {
      let mut s = serializer.serialize_struct("ListenResult", 2)?;
      s.serialize_field("resourceId", &self.resource_id)?;
      s.serialize_field("port", &self.port)?;
      s.end()
  }
}
