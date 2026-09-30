pub mod diagnostic;
#[cfg(feature = "hash")]
pub mod hash;
#[cfg(feature = "io")]
pub mod io;
pub mod path;
pub mod text;

#[cfg(test)]
mod tests;
