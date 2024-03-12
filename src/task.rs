use async_trait::async_trait;

/// A job that can be ran asynchronously.
#[async_trait]
pub trait Job {
    /// The type of input request to the job.
    type Request;

    /// The type of output response from the job.
    type Response;

    /// The type of shared resources that the job may access to perform its
    /// work.
    type Resources;

    /// The type of error that can occur when executing the work of the job.
    type Error;

    /// Executes the job.
    async fn run(
        &self,
        req: &Self::Request,
        rsrcs: &mut Self::Resources,
    ) -> Result<Self::Response, Self::Error>;
}
