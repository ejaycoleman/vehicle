use deno_core::anyhow::Error;
use deno_core::AsyncRefCell;
use deno_core::AsyncResult;
use deno_core::CancelHandle;
use deno_core::CancelTryFuture;
use deno_core::error::AnyError;
use deno_core::Extension;
use deno_core::op;
use deno_core::OpState;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ResourceId;
use std::cell::RefCell;
use deno_core::serde::Serialize;
use deno_core::serde::Serializer;
use deno_core::serde::ser::SerializeStruct;
use std::rc::Rc;
use std::net::SocketAddr;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

struct TcpListener {
  inner: tokio::net::TcpListener,
  cancel: CancelHandle,
}

impl TcpListener {
  async fn accept(self: Rc<Self>) -> Result<TcpStream, std::io::Error> {
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

struct TcpStream {
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

struct ListenResult{
  resource_id: ResourceId,
  port: u64,
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

#[op]
fn op_listen(state: &mut OpState, port: u64) -> Result<ListenResult, Error> {
  let addr = format!("127.0.0.1:{port}").parse::<SocketAddr>().unwrap();
  let std_listener = std::net::TcpListener::bind(addr)?;
  std_listener.set_nonblocking(true)?;
  let listener = TcpListener::try_from(std_listener)?;
  let rid = state.resource_table.add(listener);
  Ok(ListenResult {resource_id: rid, port})
}

#[op]
async fn op_accept(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
) -> Result<ResourceId, Error> {
  let listener = state.borrow().resource_table.get::<TcpListener>(rid)?;
  let stream = listener.accept().await?;
  let rid = state.borrow_mut().resource_table.add(stream);
  Ok(rid)
}

#[op]
async fn timeout(duration: u64) -> Result<(), AnyError> {
    tokio::time::sleep(tokio::time::Duration::from_millis(duration)).await;
    Ok(())
}

#[op]
async fn op_read_file(path: String) -> Result<String, AnyError> {
    let contents = tokio::fs::read_to_string(path).await?;
    Ok(contents)
}

#[op]
async fn op_write_file(path: String, contents: String) -> Result<(), AnyError> {
    tokio::fs::write(path, contents).await?;
    Ok(())
}

#[op]
fn op_remove_file(path: String) -> Result<(), AnyError> {
    std::fs::remove_file(path)?;
    Ok(())
}

async fn vehicle(file_path: &str) -> Result<(), AnyError> {
    // specify module from directory
    let main_module = deno_core::resolve_path(file_path)?;
    let vehicle_extension = Extension::builder().ops(vec![
        op_read_file::decl(),
        op_write_file::decl(),
        op_remove_file::decl(),
        timeout::decl(),
        op_listen::decl(),
        op_accept::decl()
    ]).build();
    
    // create new JS runtime with file-system based module loader
    let mut js_runtime = deno_core::JsRuntime::new(deno_core::RuntimeOptions {
        module_loader: Some(Rc::new(deno_core::FsModuleLoader)),
        extensions: vec![vehicle_extension],
        ..Default::default()
    });
    js_runtime.execute_script("[vehicle:runtime.js]", include_str!("./runtime.js")).unwrap();

    // load module and all dependencies
    let mod_id = js_runtime.load_main_module(&main_module, None).await?;

    // evalutate ES module
    let result = js_runtime.mod_evaluate(mod_id);

    // await event loop completion
    js_runtime.run_event_loop(false).await?;

    result.await?
}

fn main() {    
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

    if let Err(error) = runtime.block_on(vehicle("./test.js")) {
        eprintln!("error: {}", error);
    }
}
