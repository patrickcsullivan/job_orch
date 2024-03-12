use crate::task::Job;
use async_trait::async_trait;

/// The sending side of a multiple-producer channel for a job processor.
#[async_trait]
pub trait RequestSender<J>: Clone
where
    J: Job,
{
    /// An error that may occor when sending a job request to the processor.
    type Error;

    /// Sends a request to perform a job to the processor.
    fn send(&self, request: J::Request) -> Result<(), Self::Error>;
}

/// The receiving side of a multiple-consumer channel for a job processor.
#[async_trait]
pub trait ResponseReceiver<J>: Clone
where
    J: Job,
{
    /// An error that may occor when awaiting to receive a job response from the
    /// processor.
    type Error;

    /// Wait to receive a job response from the processor.
    async fn receive(&self) -> Result<Result<J::Response, J::Error>, Self::Error>;
}

/// A processor that receives asynchronous job requests, executes the jobs, and
/// then emits the job responses.  
#[async_trait]
pub trait Processor<J>
where
    J: Job,
{
    /// An error that may occur when running the processor.
    type Error;

    /// The sending side of a channel into which job requests are sent.
    type Sender: RequestSender<J>;

    /// The receiving side of a channel into which job responses are sent.
    type Receiver: ResponseReceiver<J>;

    /// Returns a sender for sending job requests to the processor.
    fn request_sender(&self) -> Self::Sender;

    /// Returns a receiver for receieving job responses from the processor.
    fn response_receiver(&self) -> Self::Receiver;

    /// Processes job requests sent to the processor and emit job responses.
    async fn run(&self) -> Result<(), Self::Error>;
}

pub struct JobAndProcessor<J, P>
where
    J: Job,
    P: Processor<J>,
{
    job: J,
    processor: P,
}

// impl<J> Sender<J> for async_channel::Sender<(J::Preprocessed, J)>
// where
//     J: Job,
// {
//     type Error = ();

//     fn send(&self, pre: J::Preprocessed, job: J) -> Result<(), Self::Error> {
//         self.send((pre, job));
//         Ok(())
//     }
// }
