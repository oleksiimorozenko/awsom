// SSM (Systems Manager) module for browsing and connecting to EC2 instances
#![allow(dead_code, unused_imports)]

mod client;

pub use client::{SsmInstance, SsmSdkClient, SsmStatus};
