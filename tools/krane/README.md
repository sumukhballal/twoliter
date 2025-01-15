## krane-bundle

This crate packages the `krane` utility from [google/go-containerregistry].

The utility is compiled by a build script, the output of which is compressed and stored in the Rust
crate as via `include_bytes!`.
At runtime, `krane-bundle` writes the decompressed binary to a temp file, passing the
filepath of that file to any caller.

[google/go-containerregistry]: https://github.com/google/go-containerregistry
