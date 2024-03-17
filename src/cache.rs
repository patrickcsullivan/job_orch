use async_trait::async_trait;

/// A trait for job whose cached response can be retrieved from [Rsrcs].
#[async_trait]
pub trait GetCache {
    type Request;
    type Response;
    type Resources;
    type Error;

    /// Try to get a cached version of the response.
    async fn get_cached(
        req: &Self::Request,
        rsrcs: &mut Self::Resources,
    ) -> Result<Option<Self::Response>, Self::Error>;
}
