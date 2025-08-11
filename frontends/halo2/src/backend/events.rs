use std::rc::Rc;

use crate::halo2::{Expression, RegionIndex};
use crate::ir::stmt::IRStmt;
use anyhow::Result;

#[derive(Clone)]
pub enum BackendMessages<F: Clone> {
    EmitStmts(EmitStmtsMessage<F>),
}

impl<F: Clone> From<EmitStmtsMessage<F>> for BackendMessages<F> {
    fn from(value: EmitStmtsMessage<F>) -> Self {
        Self::EmitStmts(value)
    }
}

impl<F: Clone> Message for BackendMessages<F> {
    type Response = BackendResponse;
}

pub enum BackendResponse {
    EmitStmts(()),
}

#[derive(Clone)]
pub struct EmitStmtsMessage<F: Clone>(pub RegionIndex, pub Vec<IRStmt<Expression<F>>>);

impl<F: Clone> Message for EmitStmtsMessage<F> {
    type Response = ();
}

pub trait Message {
    type Response;
}

pub trait EventReceiver {
    type Message: Message;

    fn accept(&self, msg: &Self::Message) -> Result<<Self::Message as Message>::Response>;
}

#[derive(Clone)]
pub struct BackendEventReceiver<F: Clone> {
    inner: Rc<dyn EventReceiver<Message = BackendMessages<F>>>,
}

impl<F: Clone> BackendEventReceiver<F> {
    pub(crate) fn new<'i, I>(inner: I) -> Self
    where
        I: EventReceiver<Message = BackendMessages<F>> + Clone + 'static,
    {
        Self {
            inner: Rc::new(inner.clone()),
        }
    }
}

impl<F: Clone> EventReceiver for BackendEventReceiver<F> {
    type Message = BackendMessages<F>;

    fn accept(&self, msg: &Self::Message) -> Result<<Self::Message as Message>::Response> {
        self.inner.accept(msg)
    }
}

#[derive(Copy)]
pub struct EventSender<'r, R> {
    receiver: &'r R,
}

impl<'r, R> Clone for EventSender<'r, R> {
    fn clone(&self) -> Self {
        EventSender {
            receiver: self.receiver,
        }
    }
}

impl<'r, R> EventSender<'r, R> {
    pub fn new(receiver: &'r R) -> Self {
        Self { receiver }
    }

    pub fn send<M>(&self, msg: &M) -> Result<M::Response>
    where
        M: Message,
        R: EventReceiver<Message = M>,
    {
        self.receiver.accept(msg)
    }

    pub fn send_iter<'a, M>(
        &self,
        msgs: impl Iterator<Item = &'a M>,
    ) -> impl Iterator<Item = Result<M::Response>>
    where
        M: Message + 'a,
        R: EventReceiver<Message = M>,
    {
        msgs.map(|msg| self.receiver.accept(msg))
    }
}

pub struct OwnedEventSender<R> {
    receiver: R,
}

impl<R: Clone> Clone for OwnedEventSender<R> {
    fn clone(&self) -> Self {
        OwnedEventSender {
            receiver: self.receiver.clone(),
        }
    }
}

impl<R> OwnedEventSender<R> {
    pub fn new(receiver: R) -> Self {
        Self { receiver }
    }

    pub fn send<M>(&self, msg: &M) -> Result<M::Response>
    where
        M: Message,
        R: EventReceiver<Message = M>,
    {
        self.receiver.accept(msg)
    }

    pub fn send_iter<'a, M>(
        &self,
        msgs: impl Iterator<Item = &'a M>,
    ) -> impl Iterator<Item = Result<M::Response>>
    where
        M: Message + 'a,
        R: EventReceiver<Message = M>,
    {
        msgs.map(|msg| self.receiver.accept(msg))
    }
}
