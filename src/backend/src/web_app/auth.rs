use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

use crate::config::CONFIG;

use super::validation::validate;

#[derive(Clone)]
pub struct UserId(pub u64);

#[derive(Clone)]
pub struct AuthLayer;

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware { inner }
    }
}

#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for AuthMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let init_data = {
            let header = req.headers().get("X-Init-Data");

            match header {
                Some(header) => {
                    let header = header.to_str().unwrap();
                    header
                }
                None => return Box::pin(async { Ok(StatusCode::UNAUTHORIZED.into_response()) }),
            }
        };

        let user_id = match validate(init_data, &CONFIG.telegram_bot_token) {
            Some(user_id) => user_id,
            None => return Box::pin(async { Ok(StatusCode::UNAUTHORIZED.into_response()) }),
        };

        req.extensions_mut().insert(UserId(user_id));

        let future = self.inner.call(req);
        Box::pin(async move {
            let response: Response = future.await?;
            Ok(response)
        })
    }
}
