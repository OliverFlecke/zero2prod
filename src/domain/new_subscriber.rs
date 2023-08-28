use super::{SubscriberEmail, SubscriberName};

/// Represents a new subscriber and their information.
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
