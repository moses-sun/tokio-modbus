use super::service::NewService;

use crate::{frame::*, server::tcp_server::TcpServer};

use futures::future;
use std::{future::Future, io::Error, net::SocketAddr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Server {
    socket_addr: SocketAddr,
    threads: Option<usize>,
}

impl Server {
    /// Set the address for the server (mandatory).
    pub fn new(socket_addr: SocketAddr) -> Self {
        Self {
            socket_addr,
            threads: None,
        }
    }

    /// Set the number of threads running simultaneous event loops (optional, Unix only).
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// Start a Modbus TCP server that blocks the current thread.
    pub fn serve<S>(self, service: S)
    where
        S: NewService<Request = crate::frame::Request, Response = crate::frame::Response>
            + Send
            + Sync
            + 'static,
        S::Request: From<Request>,
        S::Response: Into<Response>,
        S::Error: Into<Error>,
        S::Instance: Send + Sync + 'static,
    {
        self.serve_until(service, future::pending());
    }

    /// Start a Modbus TCP server that blocks the current thread.
    pub fn serve_until<S, Sd>(self, service: S, shutdown_signal: Sd)
    where
        S: NewService<Request = crate::frame::Request, Response = crate::frame::Response>
            + Send
            + Sync
            + 'static,
        Sd: Future<Output = ()> + Sync + Send + Unpin + 'static,
        S::Request: From<Request>,
        S::Response: Into<Response>,
        S::Error: Into<Error>,
        S::Instance: Send + Sync + 'static,
    {
        let mut server = TcpServer::new(self.socket_addr);
        if let Some(threads) = self.threads {
            server.threads(threads);
        }
        server.serve_until(service, shutdown_signal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::Service;

    use futures::future;

    #[tokio::test]
    async fn service_wrapper() {
        #[derive(Clone)]
        struct DummyService {
            response: Response,
        };

        impl Service for DummyService {
            type Request = Request;
            type Response = Response;
            type Error = Error;
            type Future = future::Ready<Result<Self::Response, Self::Error>>;

            fn call(&self, _: Self::Request) -> Self::Future {
                future::ready(Ok(self.response.clone()))
            }
        }

        let service = DummyService {
            response: Response::ReadInputRegisters(vec![0x33]),
        };

        let pdu = Request::ReadInputRegisters(0, 1);
        let rsp_adu = service.call(pdu).await.unwrap();

        assert_eq!(rsp_adu, service.response);
    }
}
