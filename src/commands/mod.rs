pub mod clone;
pub mod list;
pub mod search;
pub mod checksum;
pub mod misc;

pub use clone::clone_repo;
pub use list::list_repos;
pub use search::search_repos;
pub use checksum::{checksum_command, verify_command};
pub use misc::{easter_egg, generate_completions, complete_suggestions};
