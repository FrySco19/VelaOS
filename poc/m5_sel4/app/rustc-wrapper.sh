#!/bin/sh

rustc=$1
shift
previous=

for argument do
    case "$argument" in
        sel4_*)
            if [ "$previous" = "--crate-name" ]; then
                exec "$rustc" "-Zcrate-attr=feature(asm_cfg)" "$@"
            fi
            ;;
    esac
    previous=$argument
done

exec "$rustc" "$@"
