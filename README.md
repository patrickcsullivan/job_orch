# Summary

The `job_orch` library provides mechanisms for orchestrating jobs in an asynchronous, event-driven manner.

# Core concepts

## Jobs and the `Run` trait

A job is a unit of computation that implements the [Run] trait. When a job runs it has access to an input [Request] and some mutable [Resources] such as a database or an external service. When a job finishes running it produces a [Response].

The definition of a job is intendened should be independed of where and how the job is executed.

## Asynchronously starting and waiting on jobs with the `SendRequest` and `ReceiveResponse` traits

Although the definition of a job using the [Run] trait is intended to be independent of where and how a job is executed, the purpose of this library is to provide useful abstractions around running jobs asynchronously and in an event-driven manner.

In general, we expect that users of this library will want to run jobs as separate "nodes" which listen for requests and then asynchronously emit responses when they are done. The inputs and outputs of these individual nodes can then be chained together in an even-driven manner.

The `MpSend` and `SendRequest` traits are abstractions over "senders", the sender sides of multiple-producer channels. These traits implement `send` and `send_request` methods, respectively, that when called will synchronously send a request along a channel to a job processor. The following are some example implementations of these traits:

- the sender side of an in-memory channel 
- a client which forwards requests to an AWS SQS queue
- a mock which logs requests for inspection in unit tests

The `McReceive` and `ReceiveResponse` traits are abstractions over "receivers", the receiver sides of multiple-consumer (i.e., broadcast) channels. These traits implement `receive` and `receive_response` methods, respectively, that when called will wait on a response to be emitted along a channel from a job processor.  The following are some example implementations of these traits:

- the receiver side of an in-memory channel used to broadcast responses
- a client which polls an AWS SQS queue for responses
- a mock which logs calls to receive responses for inspection in unit tests

## Starting job handlers with the `Processor` trait

A job processor implements the [Processor] trait. Each [Processor] is associated with specific implementations of [Run], [SendRequest], and [RecieveResponse]. Each [Processor] implements a `run` method, in which the processor should wait on requests sent to the [SendRequest] "sender" and then broadcast responses that can be read from the [ReceiveResponse] "receiver". The [Processor] can also return instances of the related [SendRequest]s and [ReceiveResponse]s so that multiple clients may send requests to and wait on responses from the job processor.

## Executing and chaining jobs requests with the `Runner` trait

## Accessing cached job results with the `GetCache` trait