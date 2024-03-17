# Summary

The `job_orch` library provides mechanisms for orchestrating jobs in an asynchronous, event-driven manner.

# Core concepts

## Summary

- **Job**: A unit of computation that implements the [Run] trait.
- **Processor**: An asynchronous process or thread which recieves job requests, executes jobs, and publishes job responses.
- **Request Sender**: The sending side of a channel that is used by other threads or processes to send job requests to a processor.
- **Response Receiver**: The receiving side of a channel that is used by other threads or processes to await job responses from a processor.

## Job

A job is a unit of computation that implements the [Run] trait. When a job runs it has access to an input [Request] and some mutable [Resources] such as a database or an external service. When a job finishes running it produces a [Response].

The definition of a job is intendened should be independent of where and how the job is executed.

## Processor

Although the definition of a job using the [Run] trait is intended to be independent of where and how a job is executed, the purpose of this library is to provide useful abstractions around running jobs asynchronously and in an event-driven manner.

In general, we expect that users of this library will want to run jobs as separate threads or processes which listen for requests and then asynchronously emit responses when they are done. The inputs and outputs of these individual jobs can then be chained together in an even-driven manner.

We refer to one of these job processing threads or processes as a processor. A processor implements the [Processor] trait, and each [Processor] is associated with a specific implementation of the [Run] trait, a channel for sending requests to the processor, and a channel for sending responses from the processor. Each [Processor] implements a `run` method, in which the processor should wait on requests sent to the requests channel and then broadcast responses to the responses channel. The [Processor] can also return instances of the request channel's sender end and the response channel's receiver end so that multiple clients may send requests to and wait on responses from the job processor.

## Request sender

The [MpSend] trait is an abstraction over "senders", the sender sides of multiple-producer channels. This trait has a [send] method that when called should send a request along a channel to a job processor. The following are some example implementations of this traits:

- the sender side of an in-memory channel 
- a client which forwards requests to an AWS SQS queue
- a mock which logs requests for inspection in unit tests

## Response receiver

The [McReceive] trait is an abstraction over "receivers", the receiver sides of multiple-consumer (i.e., broadcast) channels. Ths trait has a [receive] method that when called should wait on a response to be emitted along a channel from a job processor.  The following are some example implementations of this trait:

- the receiver side of an in-memory channel used to broadcast responses
- a client which polls an AWS SQS queue for responses
- a mock which logs calls to receive responses for inspection in unit tests

## Caching

A job may implement the [GetCache] trait. This trait provides the [get_cache] method which accepts a request and optionally returns a cached response. By implementing this trait on a job, we can indicate that it may be possible to retrieve a cached response and avoid the expensive work of executing a job's [run] method.