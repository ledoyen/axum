use super::{FromRequest, FromRequestParts};
use crate::response::{IntoResponse, Response};
use http::request::{Parts, Request};
use std::{convert::Infallible, future::Future};

impl<S> FromRequestParts<S> for () {
    type Future<'a> = impl Future<Output = Result<Self, Self::Rejection>> + 'a
    where
        S: 'a;
    type Rejection = Infallible;

    fn from_request_parts<'a>(_: &'a mut Parts, _: &'a S) -> Self::Future<'a> {
        async move { Ok(()) }
    }
}

macro_rules! impl_from_request {
    (
        [$($ty:ident),*], $last:ident
    ) => {
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<S, $($ty,)* $last> FromRequestParts<S> for ($($ty,)* $last,)
        where
            $( $ty: FromRequestParts<S> + Send, )*
            $last: FromRequestParts<S> + Send,
            S: Send + Sync,
        {
            type Future<'a> = impl Future<Output = Result<Self, Self::Rejection>> + 'a
            where
                S: 'a;
            type Rejection = Response;

            fn from_request_parts<'a>(
                parts: &'a mut Parts,
                state: &'a S,
            ) -> Self::Future<'a> {
                async move {
                    $(
                        let $ty = $ty::from_request_parts(parts, state)
                            .await
                            .map_err(|err| err.into_response())?;
                    )*
                    let $last = $last::from_request_parts(parts, state)
                        .await
                        .map_err(|err| err.into_response())?;

                    Ok(($($ty,)* $last,))
                }
            }
        }

        // This impl must not be generic over M, otherwise it would conflict with the blanket
        // implementation of `FromRequest<S, B, Mut>` for `T: FromRequestParts<S>`.
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<S, B, $($ty,)* $last> FromRequest<S, B> for ($($ty,)* $last,)
        where
            $( $ty: FromRequestParts<S> + Send, )*
            $last: FromRequest<S, B> + Send,
            B: Send + 'static,
            S: Send + Sync,
        {
            type Future<'a> = impl Future<Output = Result<Self, Self::Rejection>> + 'a
            where
                S: 'a;
            type Rejection = Response;

            fn from_request<'a>(
                req: Request<B>,
                state: &'a S,
            ) -> Self::Future<'a> {
                async move {
                    let (mut parts, body) = req.into_parts();

                    $(
                        let $ty = $ty::from_request_parts(&mut parts, state).await.map_err(|err| err.into_response())?;
                    )*

                    let req = Request::from_parts(parts, body);

                    let $last = $last::from_request(req, state).await.map_err(|err| err.into_response())?;

                    Ok(($($ty,)* $last,))
                }
            }
        }
    };
}

all_the_tuples!(impl_from_request);

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use http::Method;

    use crate::extract::{FromRequest, FromRequestParts};

    fn assert_from_request<M, T>()
    where
        T: FromRequest<(), http_body::Full<Bytes>, M>,
    {
    }

    fn assert_from_request_parts<T: FromRequestParts<()>>() {}

    #[test]
    fn unit() {
        assert_from_request_parts::<()>();
        assert_from_request::<_, ()>();
    }

    #[test]
    fn tuple_of_one() {
        assert_from_request_parts::<(Method,)>();
        assert_from_request::<_, (Method,)>();
        assert_from_request::<_, (Bytes,)>();
    }

    #[test]
    fn tuple_of_two() {
        assert_from_request_parts::<((), ())>();
        assert_from_request::<_, ((), ())>();
        assert_from_request::<_, (Method, Bytes)>();
    }

    #[test]
    fn nested_tuple() {
        assert_from_request_parts::<(((Method,),),)>();
        assert_from_request::<_, ((((Bytes,),),),)>();
    }
}
