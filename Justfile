set positional-arguments

init:
    cargo binstall cross

build:
    cross build

release:
    cross build --release

run *args="":
    cross run -- "$@"