use crate::task::Run;
use async_trait::async_trait;

/// The sending side of a multiple-producer channel.
pub trait MpSender: Clone {
    type Message;
    type SenderError;

    fn send(&self, msg: Self::Message) -> Result<(), Self::SenderError>;
}

/// The receiving side of a multiple-consumer channel.
#[async_trait]
pub trait McReceiver: Clone {
    type Message;
    type ReceiverError;

    async fn receive(&self) -> Result<Self::Message, Self::ReceiverError>;
}

/// The sending side of a multiple-producer channel for a job processor.
pub trait RequestSender: MpSender<Message = (Self::JobId, Self::Request)> {
    type JobId;
    type Request;

    /// Sends a request to perform a job to the processor.
    fn send_request(
        &self,
        job_id: Self::JobId,
        request: Self::Request,
    ) -> Result<(), Self::SenderError> {
        self.send((job_id, request))
    }
}

// /// The sending side of a multiple-producer channel for a job processor.
// pub trait RequestSender: Clone {
//     type Request;

//     /// An error that may occor when sending a job request to the processor.
//     type SenderError;

//     /// Sends a request to perform a job to the processor.
//     fn send(&self, request: Self::Request) -> Result<(), Self::SenderError>;
// }

/// The receiving side of a multiple-consumer channel for a job processor.
#[async_trait]
pub trait ResponseReceiver:
    McReceiver<Message = (Self::JobId, Result<Self::Response, Self::JobError>)>
{
    type JobId;
    type Response;
    type JobError;

    /// Wait to receive a job response from the processor.
    async fn receive_response(
        &self,
    ) -> Result<(Self::JobId, Result<Self::Response, Self::JobError>), Self::ReceiverError> {
        self.receive().await
    }
}

/// A processor that receives asynchronous job requests, executes the jobs, and
/// then emits the job responses.  
#[async_trait]
pub trait Processor<J>
where
    J: Run,
{
    /// An error that may occur when running the processor.
    ///
    /// This does not include job-specific errors and only pertains to
    /// processor-specific errors that may occur while attempting to run a job.
    type ProcessorError;

    /// The sending side of a channel into which job requests are sent.
    type Sender: RequestSender<Request = J::Request>;

    /// The receiving side of a channel into which job responses are sent.
    type Receiver: ResponseReceiver<Response = J::Response, JobError = J::Error>;

    /// Returns a sender for sending job requests to the processor.
    fn request_sender(&self) -> Self::Sender;

    /// Returns a receiver for receieving job responses from the processor.
    fn response_receiver(&self) -> Self::Receiver;

    /// Processes job requests sent to the processor and emit job responses.
    async fn run(&self) -> Result<(), Self::ProcessorError>;
}
