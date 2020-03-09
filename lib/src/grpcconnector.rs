use std::sync::{Arc};
use std::net::ToSocketAddrs;
use std::net::SocketAddr;

use futures::{Future};

use tower_h2;
use tower_util::MakeService;
use tower_grpc::Request;

use tokio_rustls::client::TlsStream;
use tokio_rustls::{rustls::ClientConfig, TlsConnector};

use tokio::executor::DefaultExecutor;
use tokio::net::tcp::TcpStream;

use crate::grpc_client::{RawTransaction, Empty, LightdInfo};
use crate::grpc_client::client::CompactTxStreamer;

mod danger {
    use rustls;
    use webpki;

    pub struct NoCertificateVerification {}

    impl rustls::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(&self,
                              _roots: &rustls::RootCertStore,
                              _presented_certs: &[rustls::Certificate],
                              _dns_name: webpki::DNSNameRef<'_>,
                              _ocsp: &[u8]) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
            Ok(rustls::ServerCertVerified::assertion())
        }
    }
}

/// A Secure (https) grpc destination.
struct Dst {
    addr:        SocketAddr, 
    host:        String,
    no_cert:     bool,
}

impl tower_service::Service<()> for Dst {
    type Response = TlsStream<TcpStream>;
    type Error = ::std::io::Error;
    type Future = Box<dyn Future<Item = TlsStream<TcpStream>, Error = ::std::io::Error> + Send>;

    fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: ()) -> Self::Future {
        let mut config = ClientConfig::new();


        config.alpn_protocols.push(b"h2".to_vec());
        config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        
        if self.no_cert {
            config.dangerous()
                .set_certificate_verifier(Arc::new(danger::NoCertificateVerification {}));
        }

        let config = Arc::new(config);
        let tls_connector = TlsConnector::from(config);

        let addr_string_local = self.host.clone();

        let domain = match webpki::DNSNameRef::try_from_ascii_str(&addr_string_local) {
            Ok(d)  => d,
            Err(_) => webpki::DNSNameRef::try_from_ascii_str("localhost").unwrap()
        };
        let domain_local = domain.to_owned();

        let stream = TcpStream::connect(&self.addr).and_then(move |sock| {
            sock.set_nodelay(true).unwrap();
            tls_connector.connect(domain_local.as_ref(), sock)
        })
            .map(move |tcp| tcp);

        Box::new(stream)
    }
}

// Same implementation but without TLS. Should make it straightforward to run without TLS
// when testing on local machine
//
// impl tower_service::Service<()> for Dst {
//     type Response = TcpStream;
//     type Error = ::std::io::Error;
//     type Future = Box<dyn Future<Item = TcpStream, Error = ::std::io::Error> + Send>;
//
//     fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
//         Ok(().into())
//     }
//
//     fn call(&mut self, _: ()) -> Self::Future {
//         let mut config = ClientConfig::new();
//         config.alpn_protocols.push(b"h2".to_vec());
//         config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
//
//         let stream = TcpStream::connect(&self.addr)
//             .and_then(move |sock| {
//                 sock.set_nodelay(true).unwrap();
//                 Ok(sock)
//             });
//         Box::new(stream)
//     }
// }


macro_rules! make_grpc_client {
    ($protocol:expr, $host:expr, $port:expr, $nocert:expr) => {{
        let uri: http::Uri = format!("{}://{}", $protocol, $host).parse().unwrap();

        let addr = format!("{}:{}", $host, $port)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap();

        let h2_settings = Default::default();
        let mut make_client = tower_h2::client::Connect::new(Dst {addr, host: $host.to_string(), no_cert: $nocert}, h2_settings, DefaultExecutor::current());

        make_client
            .make_service(())
            .map_err(|e| { format!("HTTP/2 connection failed; err={:?}.\nIf you're connecting to a local server, please pass --dangerous to trust the server without checking its TLS certificate", e) })
            .and_then(move |conn| {
                let conn = tower_request_modifier::Builder::new()
                    .set_origin(uri)
                    .build(conn)
                    .unwrap();

                CompactTxStreamer::new(conn)
                    // Wait until the client is ready...
                    .ready()
                    .map_err(|e| { format!("client closed: {:?}", e) })
            })
    }};
}


// ==============
// GRPC code
// ==============

pub fn get_info(uri: http::Uri, no_cert: bool) -> Result<LightdInfo, String> {
    let runner = make_grpc_client!(uri.scheme_str().unwrap(), uri.host().unwrap(), uri.port_part().unwrap(), no_cert)
        .and_then(move |mut client| {
            client.get_lightd_info(Request::new(Empty{}))
                .map_err(|e| {
                    format!("ERR = {:?}", e)
                })
                .and_then(move |response| {
                    Ok(response.into_inner())
                })
                .map_err(|e| {
                    format!("ERR = {:?}", e)
                })
        });

    tokio::runtime::current_thread::Runtime::new().unwrap().block_on(runner)
}

pub fn broadcast_raw_tx(uri: &http::Uri, no_cert: bool, tx_bytes: Box<[u8]>) -> Result<String, String> {
    let runner = make_grpc_client!(uri.scheme_str().unwrap(), uri.host().unwrap(), uri.port_part().unwrap(), no_cert)
        .and_then(move |mut client| {
            client.send_transaction(Request::new(RawTransaction {data: tx_bytes.to_vec(), height: 0}))
                .map_err(|e| {
                    format!("ERR = {:?}", e)
                })
                .and_then(move |response| {
                    let sendresponse = response.into_inner();
                    if sendresponse.error_code == 0 {
                        let mut txid = sendresponse.error_message;
                        if txid.starts_with("\"") && txid.ends_with("\"") {
                            txid = txid[1..txid.len()-1].to_string();
                        }

                        Ok(txid)
                    } else {
                        Err(format!("Error: {:?}", sendresponse))
                    }
                })
                .map_err(|e| { format!("ERR = {:?}", e) })
        });

    tokio::runtime::current_thread::Runtime::new().unwrap().block_on(runner)
}
