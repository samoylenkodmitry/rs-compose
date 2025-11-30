//! Testing utilities and harness for Compose-RS

#![allow(non_snake_case)]

pub mod testing;
pub mod robot;
pub mod robot_assertions;

#[cfg(feature = "robot-app")]
pub mod robot_helpers;

// Re-export testing utilities
pub use testing::*;
pub use robot::*;
pub use robot_assertions::{SemanticElementLike, Bounds};

#[cfg(feature = "robot-app")]
pub use robot_helpers::*;

pub mod prelude {
    pub use crate::testing::*;
    pub use crate::robot::*;
    pub use crate::robot_assertions;
    pub use crate::robot_assertions::{SemanticElementLike, Bounds};
    
    #[cfg(feature = "robot-app")]
    pub use crate::robot_helpers::*;
}

