mod protocol;
mod service;
mod workspace;

pub use service::run_stdio;

#[cfg(test)]
mod tests;
