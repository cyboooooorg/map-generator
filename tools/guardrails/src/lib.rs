//! Architecture checks of the workspace, run as tier-1 tests
//! (`tests/workspace.rs`), and the test-tier budget gate (`budget`, used
//! by the binary). See CLAUDE.md, Architecture and Tests.

pub mod budget;
pub mod clippy_config;
pub mod constants;
pub mod deps;
pub mod lints;
pub mod readme;
pub mod science_pages;
