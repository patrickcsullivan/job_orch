use std::marker::PhantomData;

use crate::{
    processor::{RequestSender, ResponseReceiver},
    task::Job,
};

pub enum Runner<HeadSender, HeadS, HeadR, TailJ, TailS, TailR> {
    Single(RunnerSingle<HeadJ, HeadS, HeadR>),
}

pub struct RunnerSingle<J, S, R>
where
    J: Job,
    S: RequestSender<J>,
    R: ResponseReceiver<J>,
{
    _j: PhantomData<J>,
    request_sender: S,
    response_receiver: R,
}

impl<J, S, R> RunnerSingle<J, S, R>
where
    J: Job,
    S: RequestSender<J>,
    R: ResponseReceiver<J>,
{
    pub fn new(request_sender: S, response_receiver: R) -> Self {
        Self {
            _j: PhantomData,
            request_sender,
            response_receiver,
        }
    }

    pub async fn run(
        &self,
        req: &J::Request,
        rsrcs: &mut J::Resources,
    ) -> Result<J::Response, RunnerError<J::Error, S::Error, R::Error>> {
        todo!()
    }
}

pub enum RunnerError<JErr, SErr, RErr> {
    Job(JErr),
    RequestSender(SErr),
    ResponseReceiver(RErr),
}
