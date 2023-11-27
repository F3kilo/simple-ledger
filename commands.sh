#!/bin/bash

# Run node
cargo run -p node -- --socket "127.0.0.1:50000"

# Transfer
cargo run -p client -- --socket "127.0.0.1:60000" --key "2f0177270629ec8840dc5b9fc424f611a84259f72e4769f26f88fb69bf855e64" --node "127.0.0.1:50000" --amount 100 --transfer-to "5e5c107330e6ab97ff2abc15b5c8ba14dd92709823e669b1ed4884e69a76fd60"

# Balance
cargo run -p client -- --socket "127.0.0.1:60000" --key "2f0177270629ec8840dc5b9fc424f611a84259f72e4769f26f88fb69bf855e64" --node "127.0.0.1:50000" --balance