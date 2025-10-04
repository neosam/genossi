pub mod test_support {
    use crate::{create_app, RestStateDef};
    use std::net::SocketAddr;
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;
    
    pub struct TestServer {
        pub addr: SocketAddr,
        pub handle: JoinHandle<()>,
    }
    
    impl TestServer {
        pub fn url(&self, path: &str) -> String {
            format!("http://{}{}", self.addr, path)
        }
    }
    
    pub async fn start_test_server<RestState: RestStateDef>(
        rest_state: RestState,
    ) -> TestServer {
        // Bind to a random available port
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Could not bind to random port");
        
        let addr = listener.local_addr().expect("Could not get local address");
        
        let app = create_app(rest_state).await;
        
        // Spawn the server in a background task
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("Test server failed");
        });
        
        // Give the server a moment to start
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        TestServer { addr, handle }
    }
    
    impl Drop for TestServer {
        fn drop(&mut self) {
            // Abort the server task when the test completes
            self.handle.abort();
        }
    }
}