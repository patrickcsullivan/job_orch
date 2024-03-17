use async_trait::async_trait;

/// A trait for job whose cached response can be retrieved from [Rsrcs].
#[async_trait]
pub trait GetCache {
    /// The type of input request to the job.
    type Request;

    /// The type of output response from the job.
    type Response;

    /// The type of shared resources that the job may access to retrieve a
    /// cached response.
    type Resources;

    /// The type of error that can occur when retrieving a cached response.
    type Error;

    /// Try to get a cached version of the response.
    async fn get_cached(
        req: &Self::Request,
        rsrcs: &mut Self::Resources,
    ) -> Result<Option<Self::Response>, Self::Error>;
}
