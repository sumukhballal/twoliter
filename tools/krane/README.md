## krane-static

This crate packages the `krane` utility from [google/go-containerregistry].

The program is replicated as static library exposed via C FFI.
Rust bindings are provided which imitate `std::process::Command::output`.

[google/go-containerregistry]: https://github.com/google/go-containerregistry
