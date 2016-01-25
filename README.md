# Shamir

[![Coverage Status](https://img.shields.io/coveralls/Nebulosus/shamir.svg?style=flat-square)](https://coveralls.io/github/Nebulosus/shamir)
[![Build](https://img.shields.io/travis-ci/Nebulosus/shamir.svg?style=flat-square)](https://travis-ci.org/Nebulosus/shamir)

Shamir is a pure Rust implementation of [Shamir's secret sharing][shamirs].

[shamirs]: https://en.wikipedia.org/wiki/Shamir%27s_Secret_Sharing

## Install

To install [shamir][this_app] into your application, you need to add it to your `cargo.toml`:

```yaml
[dependencies]
shamir = "~1.0"
```

and you need to include it at the top of oyur `main.rs`:

```rust
extern crate shamir;

use shamir::SecretData;
```

[this_app]: https://github.com/Nebulosus/shamir

## Usage

```rust
extern crate shamir;

use shamir::SecretData;

fn main() {
    let secret_data = SecretData::with_secret("Hello World!", 3);

    let share1 = secret_data.get_share(1);
    let share2 = secret_data.get_share(2);
    let share3 = secret_data.get_share(3);

    let recovered = SecretData::recover_secret(3, vec![share1, share2, share3]).unwrap();

    println!("Recovered {}", recovered);
}
```
