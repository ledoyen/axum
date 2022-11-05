use crate::extract::FromRequestParts;
use http::request::Parts;

mod sealed {
    pub trait Sealed {}
    impl Sealed for http::request::Parts {}
}

/// Extension trait that adds additional methods to [`Parts`].
pub trait RequestPartsExt: sealed::Sealed + Sized {
    /// Apply an extractor to this `Parts`.
    ///
    /// This is just a convenience for `E::from_request_parts(parts, &())`.
    fn extract<E>(&mut self) -> E::Future<'_>
    where
        E: FromRequestParts<()>;

    /// Apply an extractor that requires some state to this `Parts`.
    ///
    /// This is just a convenience for `E::from_request_parts(parts, state)`.
    fn extract_with_state<'a, E, S>(&'a mut self, state: &'a S) -> E::Future<'a>
    where
        E: FromRequestParts<S>;
}

impl RequestPartsExt for Parts {
    fn extract<E>(&mut self) -> E::Future<'_>
    where
        E: FromRequestParts<()>,
    {
        self.extract_with_state::<E, _>(&())
    }

    fn extract_with_state<'a, E, S>(&'a mut self, state: &'a S) -> E::Future<'a>
    where
        E: FromRequestParts<S>,
    {
        E::from_request_parts(self, state)
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, future::Future};

    use super::*;
    use crate::{
        ext_traits::tests::{RequiresState, State},
        extract::FromRef,
    };
    use http::{Method, Request};

    #[tokio::test]
    async fn extract_without_state() {
        let (mut parts, _) = Request::new(()).into_parts();

        let method: Method = parts.extract().await.unwrap();

        assert_eq!(method, Method::GET);
    }

    #[tokio::test]
    async fn extract_with_state() {
        let (mut parts, _) = Request::new(()).into_parts();

        let state = "state".to_owned();

        let State(extracted_state): State<String> = parts
            .extract_with_state::<State<String>, String>(&state)
            .await
            .unwrap();

        assert_eq!(extracted_state, state);
    }

    // this stuff just needs to compile
    #[allow(dead_code)]
    struct WorksForCustomExtractor {
        method: Method,
        from_state: String,
    }

    impl<S> FromRequestParts<S> for WorksForCustomExtractor
    where
        S: Send + Sync,
        String: FromRef<S>,
    {
        type Rejection = Infallible;

        fn from_request_parts<'a>(
            parts: &'a mut Parts,
            state: &'a S,
        ) -> impl Future<Output = Result<Self, Self::Rejection>> + 'a {
            async move {
                let RequiresState(from_state) = parts.extract_with_state(state).await?;
                let method = parts.extract().await?;

                Ok(Self { method, from_state })
            }
        }
    }
}
