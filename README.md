# volo-ai

Rust AI for [Screeps: World][screeps], the JavaScript-based MMO game.

This uses the [`screeps-game-api`] bindings from the [rustyscreeps] organization.

[`wasm-pack`] is used for building the Rust code to WebAssembly. This example uses [Rollup] to
bundle the resulting javascript, [Babel] to transpile generated code for compatibility with older
Node.js versions running on the Screeps servers, and the [`screeps-api`] Node.js package to deploy.

Documentation for the Rust version of the game APIs is at https://docs.rs/screeps-game-api/.

Almost all crates on https://crates.io/ are usable (only things which interact with OS
apis are broken).

## Quickstart:

```sh
# Install rustup: https://rustup.rs/

# Install wasm-pack
cargo install wasm-pack

# Install wasm-opt
cargo install wasm-opt

# Install Node.js for build steps - versions 16 through 22 have been tested, any should work
# nvm is recommended but not required to manage the install, follow instructions at:
# Mac/Linux: https://github.com/nvm-sh/nvm
# Windows: https://github.com/coreybutler/nvm-windows

# Installs Node.js at version 22
nvm install 22
nvm use 22

# Clone the starter
git clone https://github.com/abogaichuk/volo-ai.git
cd volo-ai
# note: if you customize the name of the crate, you'll need to update the MODULE_NAME
# variable in the js_src/main.js file and the module import with the updated name, as well
# as the "name" in the package.json

# Install dependencies for JS build
npm install

# Copy the example config, and set up at least one deployment mode.
cp .example-screeps.yaml .screeps.yaml
nano .screeps.yaml

# compile for a configured server but don't upload
npm run deploy -- --server ptr --dryrun

# compile and upload to a configured server
npm run deploy -- --server mmo
```