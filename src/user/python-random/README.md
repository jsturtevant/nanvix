# `python-random`

You can run this app w/:

- In terminal A:
```
rm -rf /tmp/*.socket ; RUST_LOG=trace ./bin/nanvixd.elf -http-addr 127.0.0.1:8080
```

- In terminal B:
```
NANVIX_HTTP_ADDR=127.0.0.1:8080
NEW_JSON=$(jq -n \
    --arg tenant_id "foo" \
    --arg app_name "python_test" \
    --arg program "./sysroot-debug/bin/python3" \
    --arg program_args "./src/user/python-random/__main__.py" \
    '{tenant_id: $tenant_id, app_name: $app_name, program: $program, program_args: $program_args}'
)
NEW_RESPONSE=$(curl \
    --silent \
    --header "Content-Type: application/json" \
    --header "X-NVX-Message-Type: NEW" \
    --request POST \
    --data "${NEW_JSON}" \
    http://${NANVIX_HTTP_ADDR})
VM_ID=$(echo ${NEW_RESPONSE} | jq -r '.user_vm_id')
GATEWAY_SOCKADDR=$(echo ${NEW_RESPONSE} | jq -r '.gateway_sockaddr'); nc -U ${GATEWAY_SOCKADDR}

> Note: Before this, you must have built Nanvix w/ a command like: `./z build -- TOOLCHAIN_DIR=<your-toolchain-dir> MACHINE=microvm RELEASE=no LOG_LEVEL=trace PROFILER=no JAVY=  all`

It should output:
```
Practical Random Example for Nanvix
Using seed: 11324155
Random integer (1-100): 48
Random float: 0.31467531639187607

--- Simple Dice Game ---
You rolled: 5
Computer rolled: 6
Computer wins! 🤖
```