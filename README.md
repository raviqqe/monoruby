# monoruby

[![Rust](https://github.com/sisshiki1969/monoruby/actions/workflows/rust.yml/badge.svg?branch=master)](https://github.com/sisshiki1969/monoruby/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/sisshiki1969/monoruby/branch/master/graph/badge.svg?token=vAvpafdKER)](https://codecov.io/gh/sisshiki1969/monoruby)

another toy Ruby implementation with a fast JIT compiler written in Rust.

## Features

- virtual machine executer written in x86-64 assembly (yes, we support only x86-64!).

- a compact and fast JIT compiler. (internally using self-made dynamic assembler *monoasm* (https://github.com/sisshiki1969/monoasm))

- currently, supports only Integer(including Bignum), Float, String, and boolean, nil.

## Benchmark

- app_fib.rb (fib 40)

|     impl         | time(sec)  |
|:----------------:|-----------:|
|  monoruby --jit  |   0.906    |
|  monoruby        |   5.766    |
|  ruby(3.2.0dev)  |   5.726    |
|  ruby --yjit     |   1.468    |
|  ruby --mjit     |   1.929    |

- so_mandelbrot.rb

|     impl         | time(sec)  |
|:----------------:|-----------:|
|  monoruby --jit  |   0.775    |
|  monoruby        |   1.155    |
|  ruby(3.2.0dev)  |   1.797    |
|  ruby --yjit     |   1.287    |
|  ruby --mjit     |   1.351    |

## How to run

To build monoruby, You'll need installation of Rust.
Please be aware that **only nightly version of Rust works** for monoruby.

To run ruby program file on monoruby,

```sh
% cargo run test.rb
```

or

```sh
% cargo run --release -- test.rb
```

one liner

```sh
% cargo run -- -e "puts 100"
```

You can launch REPL, omitting file name.

```sh
% cargo run
```
