use std::ops;

use tokio::{net::UnixStream, try_join};
use zbus::{Connection, connection};

use crate::common::{Example, ExampleProxy};

pub struct TestClient<'ep> {
    proxy: ExampleProxy<'ep>,
    _conn: Connection,
    _server: Connection,
}

impl<'ep> ops::Deref for TestClient<'ep> {
    type Target = ExampleProxy<'ep>;

    fn deref(&self) -> &Self::Target {
        &self.proxy
    }
}

impl<'ep> TestClient<'ep> {
    pub async fn new() -> Self {
        let guid = zbus::Guid::generate();
        let (client_sock, server_sock) = UnixStream::pair().unwrap();
        let (client_conn, server_conn) = try_join!(
            connection::Builder::unix_stream(client_sock).p2p().build(),
            connection::Builder::unix_stream(server_sock)
                .server(guid)
                .unwrap()
                .p2p()
                .build()
        )
        .unwrap();

        server_conn
            .object_server()
            .at("/org/myservice/Example", Example)
            .await
            .unwrap();

        let proxy = ExampleProxy::new(&client_conn).await.unwrap();

        Self {
            proxy,
            _conn: client_conn,
            _server: server_conn,
        }
    }
}
