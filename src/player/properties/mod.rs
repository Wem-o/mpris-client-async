//! Types of the properties of a [`Player`](super::Player)

use std::any::Any;
use std::fmt::Debug;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;

use futures::Stream;
use zbus::zvariant::OwnedValue;

use crate::Player;
use crate::player::enums::Interface;

pub mod types;

/// A DBus property.
///
/// Properties also may implement [WritableProperty], or [ControlWritableProperty] (but shouldn't implement both at the same time).
pub trait Property: Debug + Clone + Copy + Hash + PartialEq {
    /// Parses form zbus's Value as this, with into_output transformations may be applied
    type ParseAs: serde::de::DeserializeOwned + Send + 'static + Clone;

    /// The output type of the property
    type Output: Send + 'static;

    /// The interface the property is on.
    fn interface(&self) -> Interface {
        Interface::default()
    }

    /// The name as specified by the [specs](https://specifications.freedesktop.org/mpris/latest/Media_Player.html)
    fn name(&self) -> &'static str;

    /// Convert the parsed value into the final Output
    fn into_output(&self, value: Self::ParseAs) -> Self::Output;

    fn into_any(&self) -> Box<Self> {
        Box::new(self.clone())
    }
}

/// Implementators of this are writable [properties](Property).
///
/// A [Property] should not implement both this and [ControlWritableProperty] at the same time!
pub trait WritableProperty: Property + Clone {
    /// The opposite of [Property::into_output], as it converts the [Property::Output] into [Property::ParseAs]
    fn from_output(&self, value: Self::Output) -> Self::ParseAs;
}

/// Implementors are [properties](Property) that can be modified,
/// but only if [`CanControl`](types::CanControl) is true.
///
/// According to the specs, this describes the player's implementation, rather than
/// the current state, meaning this wont change after an object (player) is registered.
///
/// A [Property] should not implement both this and [WritableProperty] at the same time!
pub trait ControlWritableProperty: Property + Clone {
    /// The opposite of [Property::into_output], as it converts the [Property::Output] into [Property::ParseAs]
    fn from_output(&self, value: Self::Output) -> Self::ParseAs;
}

#[derive(Debug)]
/// A generic stream yield used to combine different property streams into one.
///
/// You need to downcast to your type before you can access the value.
pub struct AnyStreamYield {
    pub value: Arc<dyn Any + Send>,
}

/// An object (or dyn) safe version of [`Property`].
///
/// Used to create a common collection of different properties.
pub trait AnyProperty: Debug {
    fn name(&self) -> &'static str;

    fn clone_box(&self) -> Box<dyn AnyProperty + Send + Sync>;

    fn subscribe_erased(
        &self,
        player: Arc<Player>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Pin<Box<dyn Stream<Item = Arc<AnyStreamYield>> + Send + 'static>>,
                        zbus::Error,
                    >,
                > + Send,
        >,
    >;
}
impl<P> AnyProperty for P
where
    P: Property + Unpin + Send + Sync + 'static + Clone,
    P::ParseAs: TryFrom<OwnedValue> + Send + Clone,
    P::Output: Send + 'static + Clone,
{
    fn name(&self) -> &'static str {
        P::name(&self)
    }

    fn clone_box(&self) -> Box<dyn AnyProperty + Send + Sync> {
        Box::new(self.clone())
    }

    fn subscribe_erased(
        &self,
        player: Arc<Player>,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Pin<Box<dyn Stream<Item = Arc<AnyStreamYield>> + Send + 'static>>,
                        zbus::Error,
                    >,
                > + Send,
        >,
    > {
        let property = self.clone();
        Box::pin(async move { player.subscribe_property_change_erased(property).await })
    }
}
