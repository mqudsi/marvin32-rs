## Marvin32 for Rust

This crate houses a native implementation of the non-cryptographic but DoS-resistant marvin32 hash algorithm, most known for being the default string hash algorithm in .NET Framework and .NET Core.

Compared to rust's default SipHash algorithm:
  * It is is somewhat faster for shorter string lengths
  * It takes a mandatory 64-bit seed (should be securely randomly generated and kept private)
  * It generates a 32-bit hash value

If you're interested in replacing SipHash and you understand what DoS resistance means in the context of a non-cryptographic hash and know that you don't need a proper cryptographic hash, marvin32 might be an option if you still need DOS protection. Otherwise, you're certainly better off with either a fast cryptographic hash like [blake3](https://crates.io/crates/blake3) or an even faster non-cryptographic (and non-DoS-resistant) hash like [fnv](https://github.com/servo/rust-fnv) or [aHash](https://crates.io/crates/ahash) for hashmap/hash table lookups or something like [xxh3](https://crates.io/crates/xxhash-rust) or [aHash](https://crates.io/crates/ahash) for general purpose hashing.

The biggest reason this crate exists is really for interop compatibility with .NET Core, letting you generate C#-compatible string hashes from a rust context.

Note that while Microsoft's implementation of marvin32 is released under the MIT license as part of the open source [.NET runtime](https://github.com/dotnet/runtime), it is known to be covered by at least one Microsoft patent. Microsoft has [a patent waiver](https://github.com/dotnet/runtime) covering the use of marvin32 as part of the .NET Framework and .NET Library/Runtime, but you're on your own otherwise.
