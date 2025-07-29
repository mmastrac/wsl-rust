# WSL APIs for Rust

This crate provides a Rust API for interacting with WSL1 and WSL2 using the more
advanced API available in WSL2.

The API works with WSL1 and WSL2 instance and is capable of:

 - Registering and exporting distributions
 - Enumerating distributions
 - Setting the version of a distribution
 - Launching processes in the distribution

Note that while WSL1 distributions are supported, you must run them under WSL2 to access this API.
