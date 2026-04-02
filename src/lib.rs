//! # (yet another) MPRIS async client
//! ## The goal
//! The goal was to create an easy to use, type-safe MPRIS wrapper.
//!
//! ## How it works
//! To connect to the bus, create an [`Mpris`] object.
//!
//! Now you have 2 ways to go:

// An mpris object. Basically just a convinience object
// that also holds the connection (which is ARC-ed anyways)
// and a bus proxy
mod mpris;

// An individual player
mod player;

pub mod reexports {
    //! The reexports from other libraries

    pub use futures::StreamExt;
    pub use zbus::Error;
}

pub mod properties {
    //! The properties of a player and related things

    pub mod erased_types {
        //! Erased types to be able to combine different type of streams and properties

        pub use crate::player::properties::{AnyProperty, AnyStreamYield};
    }
    pub use crate::player::properties::{
        ControlWritableProperty, Property, WritableProperty, types::*,
    };
}

pub mod signals {
    //! Signals that could be emmited by a player and related things

    pub use crate::player::signals::{Signal, types::*};
}

pub mod player_types {
    //! Types a player can yield

    pub use crate::player::{Loop, Metadata, Playback};
}

pub mod streams {
    //! Streams and related types

    pub mod player {
        //! Streams of player

        pub use crate::player::streams::*;
    }

    pub mod mpris {
        //! Streams of mpris

        pub use crate::mpris::{BusEvent, MprisEvent, PlayerLoop, PlayerStream};
    }
}

pub use mpris::Mpris;
pub use player::Player;
