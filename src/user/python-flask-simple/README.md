# `python-flask-simple`

Before running this app, you need to have `Flask` installed. You can install it using pip:
```
pip install --target ./sysroot-debug/lib/python3.12/site-packages flask
```

Currently, this app fails w/:

```
[TRACE][nvx::panic] PANIC file='src/libs/syscall/src/poll/message.rs', line=69 :: nfds must be > 0 && <= 4
```