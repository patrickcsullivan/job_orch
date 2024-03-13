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

#[async_trait]
impl<J1, J2> Run for (J1, J2)
where
    J1: Run,
    J2: Run,
{
    type Request = J1::Request;
    type Response = J2::Response;
    type Resources = (J1::Resources, J2::Resources);
    type Error = (Option<J1::Error>, Option<J2::Error>);

    async fn run(
        _req: &Self::Request,
        _rsrcs: &mut Self::Resources,
    ) -> Result<Self::Response, Self::Error> {
        // The implementation of this trait is not useful in itself. However, it
        // is useful to be able to indicate at a type level that (J1, J2)
        // implements Run when both J1 and J2 implement Run so that we can
        // construct unique types with which to tag implementations of Runner by
        // RunnerThen.
        Err((None, None))
    }
}
