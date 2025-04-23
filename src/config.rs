//              Copyright Adam McKellar 2025
// Distributed under the Boost Software License, Version 1.0.
//         (See accompanying file LICENSE or copy at
//          https://www.boost.org/LICENSE_1_0.txt)

/// Configures what backend is used for walking the registry source folder.
#[derive(Debug, Clone, Copy, Default)]
pub enum FetchBackend {
    /// Use functions provided by the rusts standard library.
    ///
    /// This is fairly performant and does not need an external dependency.
    #[default]
    Std,
}

/// Configures what type of cache is used.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheBackend {
    /// Serialize and compress to file.
    ///
    /// Use the default naive approach of saving all the cached licenses at once
    /// and reading the all again at the next build step.
    ///
    /// This approach brings the advantage of not pulling in more dependencies.
    #[default]
    BincodeZip,
}

/// Configure where the cache is saved.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheSaveLocation {
    /// Save the cache in a global cache.
    ///
    /// This results in a good performance, when using `license-fetcher` in many projects.
    ///
    /// Uses [ProjectDirs::cache_dir](directories::ProjectDirs::cache_dir) for the location.
    /// When compiling multiple projects at the same time and a [CacheBackend] is used,
    /// that does not support concurrent reads and writes, then there might be some minor waiting
    /// on file locks or some entries might be missing in the cache, as it was overwritten.
    #[default]
    Global,
    /// Uses the [`OUT_DIR`] for caching.
    ///
    /// Panics if [`OUT_DIR`] is not set!
    ///
    /// This should only be used in the context of fetching licenses during the building step and embedding them into your program.
    ///
    /// [`OUT_DIR`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    Local,
    /// Writes the cache into [`.license-fetcher/CARGO_MANIFEST_DIR`].
    ///
    /// Panics if [`CARGO_MANIFEST_DIR`] is not set!
    ///
    /// This is very useful if you wish to supply this cache with your sources. This then guarantees that
    /// builds never fail due errors during license fetching like `cargo` not being in path, or not having permissions to read the `~/.cargo` folder,
    /// or a file erroring and one of the many unwraps being called. That is if the cache was build with every operating system you are targeting.
    ///
    /// **Be sure to track said directory with [`git lfs`](https://git-lfs.com/)!**
    ///
    /// [`.license-fetcher/CARGO_MANIFEST_DIR`]: https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-crates
    Repository,
    /// Disables writing cache.
    None,
}

/// Configure how the cache behaves.
#[derive(Debug, Clone, Copy, Default)]
pub enum CacheBehavior {
    /// The first cache that is found is used.
    ///
    /// The following order applies for the search:
    /// 1. [Repository](CacheSaveLocation::Repository) *(only if `CARGO_MANIFEST_DIR` env var is set)*
    /// 2. [Local](CacheSaveLocation::Local) *(only if `OUT_DIR` env var is set)*
    /// 3. [Global](CacheSaveLocation::Global)
    #[default]
    CheckAllTakeFirst,
    /// Checks only global cache.
    ///
    /// Useful if you do not intend to fetch licenses during a build step.
    Global,
    /// Checking for cache is disabled.
    Disabled,
}
