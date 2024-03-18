use async_trait::async_trait;

/// A job that can run asynchronously.
#[async_trait]
pub trait Run {
    /// The type of input request to the job.
    type Request;

    /// The type of output response from the job.
    type Response;

    /// The type of shared resources that the job may access to perform its
    /// work.
    type Resources;

    /// The type of error that can occur when executing the work of the job.
    type Error;

    /// Runs the job.
    async fn run(
        req: &Self::Request,
        rsrcs: &mut Self::Resources,
    ) -> Result<Self::Response, Self::Error>;
}
